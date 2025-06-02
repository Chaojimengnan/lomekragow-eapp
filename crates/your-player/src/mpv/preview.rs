use crate::mpv::get_texture;
use eframe::{
    egui::ahash::{HashMap, HashMapExt},
    glow::{self, HasContext},
};
use libmpv::Format;

pub struct Preview {
    mpv: super::BasicMpvWrapper,
    tex: glow::Texture,
    fbo: glow::Framebuffer,
    max_size: i64,
    size: (i64, i64),
    preview: HashMap<u64, (bool, glow::Texture)>,
    update_idx: u64,
    cur_seek_idx: u64,
    interval: f64,
}

impl Preview {
    const MAX_PREVIEW_LEN: f64 = 1000.0;

    pub fn new(
        max_size: i64,
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let handle = libmpv::Mpv::new()?;

        handle.set_property("mute", true)?;
        handle.set_property("pause", true)?;
        let mpv = super::BasicMpvWrapper::new(handle, cc)?;
        mpv.event_ctx
            .observe_property("duration", Format::Double, 0)?;

        let preview = HashMap::new();
        let size = (max_size, max_size);
        let update_idx = 0;
        let cur_seek_idx = 0;
        let interval = 5.0;

        let gl = cc.gl.as_ref().unwrap();
        unsafe {
            let (fbo, tex) = super::get_frame_buffer_with_texture(gl)?;

            Ok(Self {
                mpv,
                tex,
                fbo,
                max_size,
                size,
                preview,
                update_idx,
                cur_seek_idx,
                interval,
            })
        }
    }

    pub fn clear(&mut self) {
        self.interval = 5.0;
        self.cur_seek_idx = 0;
        self.update_idx = 0;
        self.preview
            .iter_mut()
            .for_each(|(_, (ready, _))| *ready = false);
    }

    pub fn update(&mut self, gl: &glow::Context) {
        use libmpv::events::Event;
        while let Some(event) = self.mpv.event_ctx.wait_event(0.0) {
            match event {
                Err(err) => log::error!("preview mpv error: {err}"),
                Ok(event) => match event {
                    Event::FileLoaded => {
                        eapp_utils::capture_error!(
                            err => log::error!("preview mpv get property fails: {err}"),
                            {
                                let width: i64 = self.mpv.handle.get_property("width")?;
                                let height: i64 = self.mpv.handle.get_property("height")?;

                                let scale_factor = (self.max_size as f64 / width as f64)
                                    .min(self.max_size as f64 / height as f64);
                                self.size.0 = (width as f64 * scale_factor).round() as _;
                                self.size.1 = (height as f64 * scale_factor).round() as _;

                                unsafe {
                                    gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
                                    gl.tex_image_2d(
                                        glow::TEXTURE_2D,
                                        0,
                                        glow::SRGB8_ALPHA8 as _,
                                        self.size.0 as _,
                                        self.size.1 as _,
                                        0,
                                        glow::RGBA,
                                        glow::UNSIGNED_BYTE,
                                        glow::PixelUnpackData::Slice(None),
                                    );
                                    gl.bind_texture(glow::TEXTURE_2D, None);
                                    eframe::egui_glow::check_for_gl_error!(gl);
                                }
                            }
                        );
                    }
                    Event::CommandReply(idx) => {
                        if idx != 0 {
                            self.update_idx = idx;
                        }
                    }
                    Event::PropertyChange {
                        name,
                        change,
                        reply_userdata: _,
                    } => {
                        if name == "duration" {
                            use libmpv::events::PropertyData::*;
                            if let Double(value) = change {
                                self.interval = value / Self::MAX_PREVIEW_LEN;
                            }
                        }
                    }
                    _ => (),
                },
            }
        }

        if self.mpv.consume_need_update_flag() {
            if let Err(err) = self.mpv.render_ctx.render::<glow::Context>(
                self.fbo.0.get() as _,
                self.size.0 as _,
                self.size.1 as _,
                false,
            ) {
                log::error!("preview mpv render fbo fails: {err}");
            }

            if self.update_idx == 0 {
                return;
            }

            let idx = self.update_idx - 1;

            self.preview
                .entry(idx)
                .or_insert_with(|| (false, unsafe { get_texture(gl).unwrap() }));

            let (ready, tex) = self.preview.get_mut(&idx).unwrap();

            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::SRGB8_ALPHA8 as _,
                    self.size.0 as _,
                    self.size.1 as _,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
                gl.copy_tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::SRGB8_ALPHA8 as _,
                    0,
                    0,
                    self.size.0 as _,
                    self.size.1 as _,
                    0,
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.bind_texture(glow::TEXTURE_2D, None);
            }

            *ready = true;
        }
    }

    pub fn set_media(&mut self, media_path: &str) {
        self.clear();

        if let Err(err) = self.mpv.handle.command_async(0, &["loadfile", media_path]) {
            log::error!("preview set media '{media_path}' fails: {err}");
        }
    }

    pub fn size(&self) -> (i64, i64) {
        self.size
    }

    pub fn get(&mut self, playback_time: f64) -> Option<&glow::Texture> {
        let idx = (playback_time / self.interval) as u64;

        if let Some((ready, tex)) = self.preview.get(&idx) {
            if *ready {
                return Some(tex);
            }
        }

        if self.cur_seek_idx == idx + 1 {
            return None;
        }

        self.cur_seek_idx = idx + 1;

        if let Err(err) = self.mpv.handle.command_async(
            self.cur_seek_idx,
            &["seek", &playback_time.to_string(), "absolute"],
        ) {
            log::error!("preview seek {playback_time} fails: {err}");
        }

        None
    }
}
