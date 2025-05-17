use libmpv::{
    Mpv,
    events::EventContext,
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub mod player;
pub mod preview;

pub const DEFAULT_OPTS: &str = r#"# write your own mpv options here
hwdec=auto
"#;

pub const VIDEO_FORMATS: [&str; 24] = [
    "mp4", "mkv", "avi", "flv", "wmv", "webm", "vob", "mts", "ts", "m2ts", "mov", "rm", "rmvb",
    "asf", "m4v", "mpg", "mp2", "mpeg", "mpe", "mpv", "m2v", "m4v", "3gp", "f4v",
];

pub const AUDIO_FORMATS: [&str; 9] = [
    "mp3", "wav", "ogg", "flac", "aac", "ape", "ac3", "m4a", "mka",
];

pub fn make_time_string(seconds: f64) -> String {
    let hour = (seconds / 3600.0) as i32;
    let min = (seconds / 60.0) as i32 % 60;
    let sec = (seconds) as i32 % 60;
    format!("{}:{:02}:{:02}", hour, min, sec)
}

struct BasicMpvWrapper {
    pub render_ctx: RenderContext,
    pub event_ctx: EventContext,
    pub need_update: Arc<AtomicBool>,
    pub handle: Mpv,
}

impl BasicMpvWrapper {
    pub fn new(mut handle: Mpv, cc: &eframe::CreationContext<'_>) -> libmpv::Result<Self> {
        let need_update = Arc::new(AtomicBool::new(false));
        let mut render_ctx = RenderContext::new(
            unsafe { handle.ctx.as_mut() },
            vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: Self::get_proc_address,
                    ctx: cc.get_proc_address.unwrap(),
                }),
            ],
        )?;
        render_ctx.set_update_callback({
            let egui_ctx = cc.egui_ctx.clone();
            let need_update = need_update.clone();
            move || {
                need_update.store(true, Ordering::Release);
                egui_ctx.request_repaint();
            }
        });

        let mut event_ctx = handle.create_event_context();
        event_ctx.disable_deprecated_events()?;
        event_ctx.set_wakeup_callback({
            let egui_ctx = cc.egui_ctx.clone();
            move || {
                egui_ctx.request_repaint();
            }
        });

        Ok(Self {
            handle,
            render_ctx,
            event_ctx,
            need_update,
        })
    }

    fn get_proc_address(
        ctx: &&dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void,
        name: &str,
    ) -> *mut std::ffi::c_void {
        let s = std::ffi::CString::new(name).unwrap();
        ctx(&s) as _
    }

    pub fn consume_need_update_flag(&self) -> bool {
        self.need_update
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn get_texture(
    gl: &eframe::glow::Context,
) -> Result<eframe::glow::Texture, Box<dyn std::error::Error>> {
    unsafe {
        use eframe::glow::{self, HasContext};
        let tex = gl.create_texture()?;

        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as _,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        eframe::egui_glow::check_for_gl_error!(gl);

        Ok(tex)
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn get_frame_buffer_with_texture(
    gl: &eframe::glow::Context,
) -> Result<(eframe::glow::Framebuffer, eframe::glow::Texture), Box<dyn std::error::Error>> {
    unsafe {
        use eframe::glow::{self, HasContext};
        let tex = get_texture(gl)?;
        let fbo = gl.create_framebuffer()?;

        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(tex),
            0,
        );

        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.bind_texture(glow::TEXTURE_2D, None);

        eframe::egui_glow::check_for_gl_error!(gl);

        Ok((fbo, tex))
    }
}
