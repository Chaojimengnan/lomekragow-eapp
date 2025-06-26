use crate::{
    mpv::{self, player::PlayState},
    playlist::Playlist,
    tex_register::TexRegister,
};
use eapp_utils::borderless;
use eframe::egui::{self, CornerRadius, ViewportCommand};
use serde::{Deserialize, Serialize};

#[cfg(feature = "danmu")]
use crate::danmu;
#[cfg(feature = "danmu")]
use eframe::egui::TextBuffer;

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
    #[cfg(feature = "danmu")]
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

    /// the content rect of last frame, used by video frame
    #[serde(skip)]
    pub content_rect: egui::Rect,

    #[serde(skip)]
    pub last_prevent_sleep_time: f64,

    /// danmu regex string
    #[cfg(feature = "danmu")]
    pub danmu_regex_str: String,

    #[serde(skip)]
    #[cfg(feature = "danmu")]
    pub danmu_regex: Option<regex::Regex>,

    #[serde(skip)]
    #[cfg(feature = "danmu")]
    pub danmu_regex_err_str: Option<String>,

    #[cfg(feature = "danmu")]
    pub enable_danmu: bool,

    /// used for adding danmu font
    #[cfg(feature = "danmu")]
    pub danmu_font_path: String,
}

#[derive(PartialEq)]
pub enum SettingType {
    Play,
    Color,
    #[cfg(feature = "danmu")]
    Danmu,
}

#[derive(PartialEq)]
pub enum LongSettingType {
    MpvOptions,
    #[cfg(feature = "danmu")]
    DanmuFonts,
}

#[derive(PartialEq)]
pub enum PlaylistType {
    Playlist,
    #[cfg(feature = "danmu")]
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
            playlist_key: String::default(),
            playlist_cur_sel: None,
            end_reached: EndReached::Idle,
            last_playback_time: 0.0,
            content_rect: egui::Rect::ZERO,
            last_prevent_sleep_time: 0.0,
            #[cfg(feature = "danmu")]
            danmu_regex_str: String::default(),
            #[cfg(feature = "danmu")]
            danmu_regex: None,
            #[cfg(feature = "danmu")]
            danmu_regex_err_str: None,
            #[cfg(feature = "danmu")]
            enable_danmu: true,
            #[cfg(feature = "danmu")]
            danmu_font_path: String::default(),
        }
    }
}

impl App {
    pub const APP_KEY: &'static str = "app_state";
    pub const MPV_KEY: &'static str = "mpv_state";
    pub const PLAYLIST_KEY: &'static str = "playlist_state";
    #[cfg(feature = "danmu")]
    pub const DANMU_KEY: &'static str = "danmu_state";

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

        let player = loop {
            match mpv::player::Player::new(&state.options, &mpv_state, cc) {
                Ok(v) => break v,
                Err(err) => {
                    log::error!("create mpv player fails: {err}");

                    if state.options == mpv::DEFAULT_OPTS {
                        panic!("create mpv player fails");
                    }

                    state.options = mpv::DEFAULT_OPTS.to_owned();
                }
            }
        };

        let tex_register = TexRegister::default();
        let preview = mpv::preview::Preview::new(200, cc).unwrap();

        #[cfg(feature = "danmu")]
        let danmu_state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, Self::DANMU_KEY).unwrap_or_default()
        } else {
            danmu::State::default()
        };
        #[cfg(feature = "danmu")]
        let danmu = danmu::Manager::new(danmu_state, cc).unwrap();

        #[cfg(feature = "danmu")]
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
            #[cfg(feature = "danmu")]
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
        if !self.player.state().is_audio {
            self.preview.set_media(media_path);
        }

        #[cfg(feature = "danmu")]
        {
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
    }

    fn adjust(&self, corner_radius: CornerRadius) -> CornerRadius {
        let mut corner_radius = corner_radius;
        if self.state.playlist_open {
            corner_radius.nw = 0;
            corner_radius.sw = 0;
        }
        corner_radius
    }

    fn adjust_fullscreen(&self, ui: &egui::Ui, corner_radius: CornerRadius) -> CornerRadius {
        if !ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
            corner_radius
        } else {
            0.into()
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
                    self.playlist.add_list(path);
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
                if let Some(next) = self.playlist.next_item() {
                    self.set_media(&next);
                }
            }
        }
    }

    fn prevent_sleep_if_media_playing(&mut self, ui: &egui::Ui) {
        if !self.player.state().play_state.is_playing() {
            return;
        }

        let now = ui.ctx().input(|i| i.time);
        if now - self.state.last_prevent_sleep_time >= 120.0 {
            self.state.last_prevent_sleep_time = now;
            eapp_utils::platform::prevent_sleep();
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::APP_KEY, &self.state);
        eframe::set_value(storage, Self::MPV_KEY, &self.player.state());
        eframe::set_value(storage, Self::PLAYLIST_KEY, &self.playlist);
        #[cfg(feature = "danmu")]
        eframe::set_value(storage, Self::DANMU_KEY, &self.danmu.state());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.extreme_bg_color)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            self.prevent_sleep_if_media_playing(ui);

            let gl = frame.gl().unwrap();

            self.player.update(gl);
            if !self.player.state().is_audio {
                self.preview.update(gl);
            }

            self.ui_background(ui);

            #[cfg(feature = "danmu")]
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
