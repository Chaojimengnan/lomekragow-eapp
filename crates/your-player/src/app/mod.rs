use crate::{
    danmu,
    mpv::{self, player::PlayState},
    playlist::Playlist,
    tex_register::TexRegister,
};
use eframe::egui::{self, Color32, Rounding, TextBuffer, ViewportCommand};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod background;
mod contents;
mod opts_highlight;
mod playlist;
mod popups;

pub struct App {
    state: State,
    playlist: Playlist,
    player: mpv::player::Player,
    preview: mpv::preview::Preview,
    tex_register: TexRegister,
    danmu: danmu::Manager,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    pub playlist_open: bool,

    /// mpv options
    pub options: String,

    #[serde(skip)]
    pub volume_popup_open: bool,
    #[serde(skip)]
    pub chapters_popup_open: bool,
    #[serde(skip)]
    pub setting_popup_open: bool,
    #[serde(skip)]
    pub long_setting_popup_open: bool,

    /// how the setting to show
    #[serde(skip)]
    pub setting_type: SettingType,

    /// how the playlist to show
    #[serde(skip)]
    pub playlist_type: PlaylistType,

    /// how the long setting to show
    #[serde(skip)]
    pub long_setting_type: LongSettingType,

    /// danmu regex string
    pub danmu_regex_str: String,

    #[serde(skip)]
    pub danmu_regex: Option<regex::Regex>,

    #[serde(skip)]
    pub danmu_regex_err_str: Option<String>,

    /// filter keywords
    pub playlist_key: String,

    /// current selectd media by user
    #[serde(skip)]
    pub playlist_cur_sel: Option<(String, String)>,

    /// how to do when media end reached
    pub end_reached: EndReached,

    /// playback time of the simulation, for smoother danmu movement
    #[serde(skip)]
    pub last_playback_time: f64,

    /// last [`mpv::player::State::playback_time`],
    /// used to check if the playback time of the mpv changes at current frame
    #[serde(skip)]
    pub last_real_playback_time: f64,

    /// for calculating [`State::last_playback_time`]
    /// when real playback time doesn't changes at current frame
    #[serde(skip)]
    pub last_instant: std::time::Instant,

    /// the content rect of last frame, used by video frame
    #[serde(skip)]
    pub content_rect: egui::Rect,

    pub enable_danmu: bool,

    /// used for adding danmu font
    pub danmu_font_path: String,

    #[serde(skip)]
    pub last_prevert_sleep_call: Duration,
}

#[derive(PartialEq)]
pub enum SettingType {
    Play,
    Color,
    Danmu,
}

#[derive(PartialEq)]
pub enum LongSettingType {
    MpvOptions,
    DanmuFonts,
}

#[derive(PartialEq)]
pub enum PlaylistType {
    Playlist,
    Danmu,
}

#[derive(PartialEq, Deserialize, Serialize, Clone, Copy)]
pub enum EndReached {
    Idle,
    Repeat,
    Next,
}

pub const END_REACHED_LIST: [(EndReached, &str); 3] = [
    (EndReached::Idle, "Idle"),
    (EndReached::Repeat, "Repeat"),
    (EndReached::Next, "Next"),
];

impl Default for State {
    fn default() -> Self {
        Self {
            playlist_open: true,
            options: mpv::DEFAULT_OPTS.to_owned(),
            volume_popup_open: false,
            chapters_popup_open: false,
            setting_popup_open: false,
            long_setting_popup_open: false,
            setting_type: SettingType::Play,
            playlist_type: PlaylistType::Playlist,
            long_setting_type: LongSettingType::MpvOptions,
            danmu_regex_str: String::default(),
            danmu_regex: None,
            danmu_regex_err_str: None,
            playlist_key: String::default(),
            playlist_cur_sel: None,
            end_reached: EndReached::Idle,
            last_playback_time: 0.0,
            last_real_playback_time: 0.0,
            last_instant: std::time::Instant::now(),
            content_rect: egui::Rect::ZERO,
            enable_danmu: true,
            danmu_font_path: String::default(),
            last_prevert_sleep_call: Duration::default(),
        }
    }
}

impl App {
    pub const APP_KEY: &'static str = "app_state";
    pub const MPV_KEY: &'static str = "mpv_state";
    pub const PLAYLIST_KEY: &'static str = "playlist_state";
    pub const DANMU_KEY: &'static str = "danmu_state";

    pub const ACTIVE_COL: Color32 = Color32::from_rgba_premultiplied(80, 138, 214, 160);
    pub const INACTIVE_COL: Color32 = Color32::from_rgba_premultiplied(40, 74, 122, 160);

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.style_mut(|style| style.animation_time = 0.11);

        let mut state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, Self::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };

        let mpv_state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, Self::MPV_KEY).unwrap_or_default()
        } else {
            mpv::player::State::default()
        };

        let playlist = if let Some(storage) = cc.storage {
            eframe::get_value(storage, Self::PLAYLIST_KEY).unwrap_or_default()
        } else {
            Playlist::default()
        };

        let danmu_state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, Self::DANMU_KEY).unwrap_or_default()
        } else {
            danmu::State::default()
        };

        let player = loop {
            match mpv::player::Player::new(&state.options, &mpv_state, cc) {
                Ok(v) => break v,
                Err(err) => {
                    log::error!("create mpv player fails: {err}");

                    if &state.options == mpv::DEFAULT_OPTS {
                        panic!("create mpv player fails");
                    }

                    state.options = mpv::DEFAULT_OPTS.to_owned();
                }
            }
        };

        let tex_register = TexRegister::default();
        let preview = mpv::preview::Preview::new(200, cc).unwrap();
        let danmu = danmu::Manager::new(danmu_state, cc).unwrap();

        if !state.danmu_regex_str.is_empty() {
            state.danmu_regex = match regex::Regex::new(&state.danmu_regex_str) {
                Ok(v) => Some(v),
                Err(err) => {
                    state.danmu_regex_err_str = Some(err.to_string());
                    None
                }
            };
        }

        let mut this = Self {
            state,
            playlist,
            player,
            preview,
            tex_register,
            danmu,
        };

        if let Some(path_str) = std::env::args().nth(1) {
            if std::path::Path::new(&path_str).is_file() {
                this.set_media(&path_str);
                this.playlist.set_current_play(None);
            }
        }

        this
    }

    /// set media to player and preview, regardless playlist
    pub fn set_media(&mut self, media_path: &str) {
        self.player.set_media(media_path);
        self.preview.set_media(media_path);

        let mut path = std::path::PathBuf::from(media_path);
        path.set_extension("json");

        if path.is_file() {
            let path_str = path.to_string_lossy();
            match self.danmu.load_danmu(path.to_string_lossy().as_str()) {
                Ok(_) => return,
                Err(err) => log::error!("load danmu '{}' fails: {err}", path_str.as_str()),
            }
        }
        self.danmu.clear();
    }

    fn adjust(&self, rounding: Rounding) -> Rounding {
        let mut rounding = rounding;
        if self.state.playlist_open {
            rounding.nw = 0.0;
            rounding.sw = 0.0;
        }
        rounding
    }

    fn adjust_fullscreen(&self, ui: &egui::Ui, rounding: Rounding) -> Rounding {
        if !ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
            rounding
        } else {
            0.0.into()
        }
    }

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if ui.memory(|mem| mem.focused().is_none()) {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.player.seek(-0.5, true);
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.player.seek(0.5, true);
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.player.set_volume(self.player.state().volume + 5);
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.player.set_volume(self.player.state().volume - 5);
            }

            if ui.input(|i| i.key_pressed(egui::Key::M)) {
                self.player.set_mute(!self.player.state().mute);
            }

            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                self.player
                    .set_play_state(if self.player.state().play_state.is_playing() {
                        PlayState::Pause
                    } else {
                        PlayState::Play
                    });
            }

            let quit_fullscreen = ui.input(|i| {
                i.viewport().fullscreen.unwrap_or(false) && i.key_pressed(egui::Key::Escape)
            });
            if quit_fullscreen {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Fullscreen(false));
            }

            let mut opt_path = None;
            ui.ctx().input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    if let Some(path) = &i.raw.dropped_files.first().unwrap().path {
                        opt_path = Some(path.to_string_lossy().into_owned());
                    }
                }
            });

            // we should be careful for deadlock
            if let Some(path) = opt_path {
                if std::path::Path::new(&path).is_file() {
                    self.set_media(&path);
                    self.playlist.set_current_play(None);
                } else {
                    self.playlist.add_list(&path);
                }
            }
        }
    }

    fn process_if_end_reached(&mut self) {
        if self.player.state().play_state != PlayState::EndReached {
            return;
        }

        match self.state.end_reached {
            EndReached::Idle => (),
            EndReached::Repeat => self.player.set_play_state(PlayState::Play),
            EndReached::Next => {
                if let Some(next) = self.playlist.next() {
                    self.set_media(&next);
                }
            }
        }
    }

    fn prevent_sleep_if_media_playing(&mut self) {
        if !self.player.state().play_state.is_playing() {
            return;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        if now - self.state.last_prevert_sleep_call >= Duration::from_secs(120) {
            self.state.last_prevert_sleep_call = now;
            eapp_utils::platform::prevent_sleep();
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::APP_KEY, &self.state);
        eframe::set_value(storage, Self::MPV_KEY, &self.player.state());
        eframe::set_value(storage, Self::PLAYLIST_KEY, &self.playlist);
        eframe::set_value(storage, Self::DANMU_KEY, &self.danmu.state());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        eapp_utils::borderless::window_frame(ctx, Some(Color32::BLACK)).show(ctx, |ui| {
            eapp_utils::borderless::handle_resize(ui);

            self.prevent_sleep_if_media_playing();

            let gl = frame.gl().unwrap();

            self.player.update(gl);
            self.preview.update(gl);

            self.ui_background(ui);

            if self.player.state().play_state.is_playing()
                && self.state.enable_danmu
                && !self.danmu.danmu().is_empty()
            {
                ctx.request_repaint();

                let playback_time = self.player.state().playback_time;
                self.danmu.emit(
                    playback_time..(playback_time + 0.1),
                    self.state.danmu_regex.as_ref(),
                );
                self.danmu.update(ui, gl);
            }

            self.process_if_end_reached();

            self.ui_playlist(ui);
            self.ui_contents(ui);

            self.process_inputs(ui);

            self.tex_register.register_native_tex_if_any(frame);
        });
    }
}
