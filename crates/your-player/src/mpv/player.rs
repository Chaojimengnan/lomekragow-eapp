use eframe::glow::{self, HasContext};
use libmpv::Format;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const VIDEO_ROTATE_LIST: [(&str, i64); 4] = [("0", 0), ("90", 90), ("180", 180), ("270", 270)];

pub const VIDEO_ASPECT_LIST: [(&str, f64); 4] = [
    ("auto", -1.0),
    ("4:3", 4.0 / 3.0),
    ("16:9", 16.0 / 9.0),
    ("2.35:1", 2.35 / 1.0),
];

pub type ListIdx = usize;

#[derive(Clone, PartialEq, Debug)]
pub enum PlayState {
    Play,
    Pause,
    Stop,
    EndReached,
}

impl PlayState {
    pub fn is_playing(&self) -> bool {
        matches!(self, PlayState::Play)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct State {
    /// the title of media, may diffent from media filename
    #[serde(skip)]
    pub media_title: String,
    #[serde(skip)]
    pub media_size: (i64, i64),
    #[serde(skip)]
    pub play_state: PlayState,

    pub video_rotate: ListIdx,

    #[serde(skip)]
    pub playback_time: f64,
    #[serde(skip)]
    pub duration: f64,

    pub sub_visibility: bool,
    pub sub_delay: i64,
    pub speed: f64,
    pub mute: bool,
    pub volume: i64,
    pub video_aspect: ListIdx,

    pub brightness: i64,
    pub contrast: i64,
    pub saturation: i64,
    pub gamma: i64,
    pub hue: i64,
    pub sharpen: f64,

    #[serde(skip)]
    pub chapters: Vec<(String, f64)>,
    #[serde(skip)]
    pub audio_tracks: Vec<(String, i64)>,
    #[serde(skip)]
    pub subtitle_tracks: Vec<(String, i64)>,
    #[serde(skip)]
    pub cur_audio_idx: usize,
    #[serde(skip)]
    pub cur_subtitle_idx: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            media_title: Default::default(),
            media_size: (0, 0),
            play_state: PlayState::Stop,
            video_rotate: 0,
            playback_time: 0.0,
            duration: 0.0,
            sub_visibility: true,
            sub_delay: 0,
            speed: 1.0,
            mute: false,
            volume: 50,
            video_aspect: 0,
            brightness: 0,
            contrast: 0,
            saturation: 0,
            gamma: 0,
            hue: 0,
            sharpen: 0.0,
            chapters: Default::default(),
            audio_tracks: Default::default(),
            subtitle_tracks: Default::default(),
            cur_audio_idx: 0,
            cur_subtitle_idx: 0,
        }
    }
}

impl State {
    pub fn reset_media_related_state(&mut self) {
        self.play_state = PlayState::Stop;
        self.duration = 0.0;
        self.playback_time = 0.0;
        self.media_size = (0, 0);
        self.media_title.clear();
        self.chapters.clear();
        self.audio_tracks.clear();
        self.subtitle_tracks.clear();
        self.cur_audio_idx = 0;
        self.cur_subtitle_idx = 0;
    }
}

pub struct Player {
    mpv: super::BasicMpvWrapper,
    tex: glow::Texture,
    fbo: glow::Framebuffer,
    state: State,
}

impl Player {
    pub fn new(
        options: &str,
        state: &State,
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (opts_before_init, opts) = Self::parse_options(options);

        let handle = libmpv::Mpv::with_initializer(|i| {
            for (key, value) in opts_before_init {
                i.set_property(key, value)?;
            }
            Ok(())
        })?;

        for (key, value) in opts {
            handle.set_property(key, value)?;
        }

        handle.set_property("terminal", "no")?;
        handle.set_property("keep-open", "yes")?;
        handle.set_property("video-timing-offset", 0)?;
        handle.request_log_messages("v")?;

        let mpv = super::BasicMpvWrapper::new(handle, cc)?;
        let e = &mpv.event_ctx;

        e.observe_property("duration", Format::Double, 0)?;
        e.observe_property("playback-time", Format::Double, 0)?;
        e.observe_property("eof-reached", Format::Flag, 0)?;
        e.observe_property("track-list", Format::Node, 0)?;
        e.observe_property("chapter-list", Format::Node, 0)?;

        let state = state.clone();

        let gl = cc.gl.as_ref().unwrap();
        unsafe {
            let (fbo, tex) = super::get_frame_buffer_with_texture(gl)?;
            let mut this = Self {
                mpv,
                tex,
                fbo,
                state,
            };

            this.apply_mpv_related_states();
            Ok(this)
        }
    }

    pub fn update(&mut self, gl: &glow::Context) {
        use libmpv::events::Event;
        while let Some(event) = self.mpv.event_ctx.wait_event(0.0) {
            match event {
                Err(err) => log::error!("mpv error: {err}"),
                Ok(event) => match event {
                    Event::LogMessage {
                        prefix,
                        level,
                        text,
                        log_level: _,
                    } => match level {
                        "fatal" | "error" => log::error!("[{prefix}][{level}]: {text}"),
                        "warn" => log::warn!("[{prefix}][{level}]: {text}"),
                        _ => log::info!("[{prefix}][{level}]: {text}"),
                    },
                    Event::EndFile(reason) => {
                        if reason == libmpv::mpv_end_file_reason::Error {
                            self.set_play_state_internal(PlayState::Stop);
                        }
                    }
                    Event::FileLoaded => {
                        eapp_utils::capture_error!(
                            err,
                            { log::error!("mpv get property fails: {err}") },
                            {
                                self.state.media_title =
                                    self.mpv.handle.get_property("media-title")?;
                                self.state.media_size.0 = self.mpv.handle.get_property("width")?;
                                self.state.media_size.1 = self.mpv.handle.get_property("height")?;

                                self.set_cur_audio_idx(self.state.cur_audio_idx);
                                self.set_cur_subtitle_idx(self.state.cur_subtitle_idx);

                                unsafe {
                                    gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
                                    gl.tex_image_2d(
                                        glow::TEXTURE_2D,
                                        0,
                                        glow::SRGB8_ALPHA8 as _,
                                        self.state.media_size.0 as _,
                                        self.state.media_size.1 as _,
                                        0,
                                        glow::RGBA,
                                        glow::UNSIGNED_BYTE,
                                        None,
                                    );
                                    gl.bind_texture(glow::TEXTURE_2D, None);
                                    eframe::egui_glow::check_for_gl_error!(gl);
                                }
                            }
                        );
                    }
                    Event::PropertyChange {
                        name,
                        change,
                        reply_userdata: _,
                    } => {
                        use libmpv::events::PropertyData::*;
                        match name {
                            "duration" => {
                                if let Double(value) = change {
                                    self.state.duration = value;
                                }
                            }
                            "playback-time" => {
                                if let Double(value) = change {
                                    self.state.playback_time = value;
                                }
                            }
                            "eof-reached" => {
                                if let Flag(end_reached) = change {
                                    if end_reached {
                                        self.state.play_state = PlayState::EndReached;
                                    }
                                }
                            }
                            "track-list" => {
                                if let Node(node) = change {
                                    self.state.audio_tracks.clear();
                                    self.state.subtitle_tracks.clear();

                                    || -> Option<()> {
                                        for item in node.to_array()? {
                                            let map: HashMap<&str, libmpv::MpvNode> =
                                                item.to_map()?.collect();
                                            let mut title = "Unknown";
                                            if let Some(str) = map.get("title") {
                                                title = str.to_str()?;
                                            }
                                            let track_type = map.get("type")?.to_str()?;
                                            let id = map.get("id")?.to_i64()?;

                                            if track_type == "audio" {
                                                self.state
                                                    .audio_tracks
                                                    .push((title.to_owned(), id));
                                            }
                                            if track_type == "sub" {
                                                self.state
                                                    .subtitle_tracks
                                                    .push((title.to_owned(), id));
                                            }
                                        }
                                        Some(())
                                    }();

                                    self.state.cur_audio_idx = self
                                        .state
                                        .cur_audio_idx
                                        .clamp(0, self.state.audio_tracks.len());
                                    self.state.cur_subtitle_idx = self
                                        .state
                                        .cur_subtitle_idx
                                        .clamp(0, self.state.subtitle_tracks.len());
                                    self.set_cur_audio_idx(self.state.cur_audio_idx);
                                    self.set_cur_subtitle_idx(self.state.cur_subtitle_idx);
                                }
                            }
                            "chapter-list" => {
                                if let Node(node) = change {
                                    self.state.chapters.clear();

                                    || -> Option<()> {
                                        for item in node.to_array()? {
                                            let map: HashMap<&str, libmpv::MpvNode> =
                                                item.to_map()?.collect();
                                            let mut title = "Unknown";
                                            if let Some(str) = map.get("title") {
                                                title = str.to_str()?;
                                            }
                                            let time = map.get("time")?.to_f64()?;
                                            self.state.chapters.push((title.to_owned(), time));
                                        }
                                        Some(())
                                    }();
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                },
            }
        }

        if self.mpv.consume_need_update_flag() && self.state.media_size != (0, 0) {
            if let Err(err) = self.mpv.render_ctx.render::<glow::Context>(
                self.fbo.0.get() as _,
                self.state.media_size.0 as _,
                self.state.media_size.1 as _,
                false,
            ) {
                log::error!("mpv render fbo fails: {err}");
            }
        }
    }

    pub fn texture(&self) -> &glow::Texture {
        &self.tex
    }

    fn apply_mpv_related_states(&mut self) {
        self.set_video_rotate(self.state.video_rotate);
        self.set_sub_visibility(self.state.sub_visibility);
        self.set_sub_delay(self.state.sub_delay);
        self.set_speed(self.state.speed);
        self.set_mute(self.state.mute);
        self.set_volume(self.state.volume);
        self.set_video_aspect(self.state.video_aspect);
        self.set_brightness(self.state.brightness);
        self.set_contrast(self.state.contrast);
        self.set_saturation(self.state.saturation);
        self.set_gamma(self.state.gamma);
        self.set_hue(self.state.hue);
        self.set_sharpen(self.state.sharpen);
    }

    fn parse_options(options: &str) -> (HashMap<&str, &str>, HashMap<&str, &str>) {
        let keys_before_init = HashSet::from([
            "config",
            "config-dir",
            "input-conf",
            "load-scripts",
            "script",
            "scripts",
        ]);

        let mut opts_before_init = HashMap::new();
        let mut opts = HashMap::new();

        for line in options.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = if let Some((key, value)) = line.split_once('=') {
                (key, value)
            } else {
                (line, "")
            };

            if keys_before_init.contains(key) {
                opts_before_init.insert(key, value);
            } else {
                opts.insert(key, value);
            }
        }

        (opts_before_init, opts)
    }
}

macro_rules! simple_setter {
    ($fn_name:ident, $var_name:ident, $mpv_name:literal, $type:ty) => {
        pub fn $fn_name(&mut self, $var_name: $type) {
            match self.mpv.handle.set_property($mpv_name, $var_name) {
                Ok(_) => self.state.$var_name = $var_name,
                Err(err) => log::error!("set {} fails: {err}", $mpv_name),
            }
        }
    };
}

impl Player {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn set_media(&mut self, media_path: &str) {
        match self.mpv.handle.command_async(0, &["loadfile", media_path]) {
            Ok(_) => self.set_play_state_internal(PlayState::Play),
            Err(err) => log::error!("set media '{media_path}' fails: {err}"),
        }
    }

    pub fn set_play_state(&mut self, play_state: PlayState) {
        if self.state.play_state == PlayState::Stop || play_state == PlayState::EndReached {
            return;
        }

        if self.state.play_state == PlayState::EndReached {
            self.seek(0.0, false);
        }

        self.set_play_state_internal(play_state);
    }

    fn set_play_state_internal(&mut self, play_state: PlayState) {
        if self.state.play_state == play_state {
            return;
        }

        self.state.play_state = play_state;

        eapp_utils::capture_error!(err, { log::error!("error when set play state: {err}") }, {
            match self.state.play_state {
                PlayState::Play => self.mpv.handle.set_property("pause", false)?,
                PlayState::Pause => self.mpv.handle.set_property("pause", true)?,
                PlayState::Stop => {
                    self.mpv.handle.command("stop", &[])?;
                    self.state.reset_media_related_state();
                }
                PlayState::EndReached => (),
            }
        });
    }

    pub fn set_video_rotate(&mut self, video_rotate: ListIdx) {
        match self
            .mpv
            .handle
            .set_property("video-rotate", VIDEO_ROTATE_LIST[video_rotate].1)
        {
            Ok(_) => self.state.video_rotate = video_rotate,
            Err(err) => log::error!("set video rotate fails: {err}"),
        }
    }

    pub fn seek(&mut self, playback_time: f64, relative: bool) {
        if self.state.play_state == PlayState::Stop {
            return;
        }

        if let Err(err) = if relative {
            self.mpv
                .handle
                .command_async(0, &["seek", &playback_time.to_string()])
        } else {
            self.mpv
                .handle
                .command_async(0, &["seek", &playback_time.to_string(), "absolute"])
        } {
            log::error!("seek fails: {err}");
        }
    }

    pub fn set_sub_visibility(&mut self, sub_visibility: bool) {
        match self
            .mpv
            .handle
            .set_property("sub-visibility", if sub_visibility { "yes" } else { "no" })
        {
            Ok(_) => self.state.sub_visibility = sub_visibility,
            Err(err) => log::error!("set sub visibility fails: {err}"),
        }
    }

    pub fn set_video_aspect(&mut self, video_aspect: ListIdx) {
        match self
            .mpv
            .handle
            .set_property("video-aspect-override", VIDEO_ASPECT_LIST[video_aspect].1)
        {
            Ok(_) => self.state.video_aspect = video_aspect,
            Err(err) => log::error!("set video aspect fails: {err}"),
        }
    }

    simple_setter!(set_sub_delay, sub_delay, "sub-delay", i64);

    simple_setter!(set_speed, speed, "speed", f64);

    simple_setter!(set_mute, mute, "mute", bool);

    simple_setter!(set_volume, volume, "volume", i64);

    simple_setter!(set_brightness, brightness, "brightness", i64);

    simple_setter!(set_contrast, contrast, "contrast", i64);

    simple_setter!(set_saturation, saturation, "saturation", i64);

    simple_setter!(set_gamma, gamma, "gamma", i64);

    simple_setter!(set_hue, hue, "hue", i64);

    simple_setter!(set_sharpen, sharpen, "sharpen", f64);

    pub fn set_cur_audio_idx(&mut self, cur_audio_idx: usize) {
        if self.state.audio_tracks.is_empty() {
            return;
        }

        let cur_audio_idx = cur_audio_idx.clamp(0, self.state.audio_tracks.len());
        match self
            .mpv
            .handle
            .set_property("aid", self.state.audio_tracks[cur_audio_idx].1)
        {
            Ok(_) => self.state.cur_audio_idx = cur_audio_idx,
            Err(err) => log::error!("set cur audio idx fails: {err}"),
        }
    }

    pub fn set_cur_subtitle_idx(&mut self, cur_subtitle_idx: usize) {
        if self.state.subtitle_tracks.is_empty() {
            return;
        }

        let cur_subtitle_idx = cur_subtitle_idx.clamp(0, self.state.subtitle_tracks.len());
        match self
            .mpv
            .handle
            .set_property("sid", self.state.subtitle_tracks[cur_subtitle_idx].1)
        {
            Ok(_) => self.state.cur_subtitle_idx = cur_subtitle_idx,
            Err(err) => log::error!("set cur subtitle idx fails: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test_mpv_opts_parse() {
        let opts = crate::mpv::DEFAULT_OPTS;
        println!("{:?}", Player::parse_options(opts));
    }
}
