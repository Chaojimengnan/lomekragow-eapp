use eframe::egui;

pub mod animation;
pub mod borderless;
pub mod codicons;
pub mod natordset;
pub mod platform;
pub mod task;
pub mod waker;
pub mod widgets;

#[macro_export]
macro_rules! capture_error {
    ($i:ident => $handler:expr, $block_to_capture:expr) => {
        if let Err($i) = || -> ::core::result::Result<(), Box<dyn ::std::error::Error>> {
            $block_to_capture;
            Ok(())
        }() {
            $handler;
        }
    };
}

/// Setup fonts for application
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    include_flate::flate!(static UNIFONT: [u8] from "../../assets/unifont-15.1.04.otf");
    include_flate::flate!(static CODICON: [u8] from "../../assets/codicon.ttf");

    fonts.font_data.insert(
        "unifont".to_owned(),
        egui::FontData::from_static(&UNIFONT).into(),
    );
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(&CODICON).into(),
    );

    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();

    proportional.insert(0, "unifont".to_owned());
    proportional.insert(1, "codicon".to_owned());

    ctx.set_fonts(fonts);
    ctx.style_mut(setup_text_size);
}

pub fn setup_text_size(style: &mut egui::Style) {
    use crate::egui::FontFamily::Proportional;
    use crate::egui::FontId;
    use crate::egui::TextStyle::*;
    style.text_styles = [
        (Heading, FontId::new(18.0, Proportional)),
        (Body, FontId::new(16.0, Proportional)),
        (Monospace, FontId::new(16.0, Proportional)),
        (Button, FontId::new(16.0, Proportional)),
        (Small, FontId::new(12.0, Proportional)),
    ]
    .into();
}

pub fn get_font_id(ui: &egui::Ui, text_style: &egui::TextStyle) -> Option<egui::FontId> {
    ui.style().text_styles.get(text_style).cloned()
}

pub fn get_text_size(ui: &egui::Ui, text_style: &egui::TextStyle) -> Option<f32> {
    get_font_id(ui, text_style).map(|font_id| font_id.size)
}

pub fn get_body_text_size(ui: &egui::Ui) -> f32 {
    get_body_font_id(ui).size
}

pub fn get_body_font_id(ui: &egui::Ui) -> egui::FontId {
    get_font_id(ui, &egui::TextStyle::Body).unwrap()
}

pub fn get_button_height(ui: &egui::Ui) -> f32 {
    ui.style().spacing.button_padding.y + get_text_size(ui, &egui::TextStyle::Button).unwrap()
}

pub fn setup_loggers(log_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    use simplelog::{Config, LevelFilter, WriteLogger};
    use std::fs::File;

    #[cfg(debug_assertions)]
    {
        use simplelog::{CombinedLogger, SimpleLogger};
        CombinedLogger::init(vec![
            SimpleLogger::new(LevelFilter::Info, Config::default()),
            WriteLogger::new(
                LevelFilter::Warn,
                Config::default(),
                File::create(
                    std::env::current_exe()?
                        .parent()
                        .unwrap()
                        .join(log_filename),
                )?,
            ),
        ])?;
    }

    #[cfg(not(debug_assertions))]
    WriteLogger::init(
        LevelFilter::Warn,
        Config::default(),
        File::create(
            std::env::current_exe()?
                .parent()
                .unwrap()
                .join(log_filename),
        )?,
    )?;

    Ok(())
}

pub fn open_in_explorer(path: &str) {
    // https://github.com/tauri-apps/plugins-workspace/issues/999
    #[allow(clippy::zombie_processes)]
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .args(["/select,", path])
        .spawn()
        .unwrap();

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .args(["-R", path])
        .spawn()
        .unwrap();
}

#[inline]
pub fn window_resize(ui: &mut egui::Ui, size: egui::Vec2) {
    ui.ctx()
        .send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
}

#[inline]
pub fn calculate_fit_scale(available_size: egui::Vec2, image_size: egui::Vec2) -> f32 {
    let width_scale = available_size.x / image_size.x;
    let height_scale = available_size.y / image_size.y;
    width_scale.min(height_scale)
}

#[inline]
pub fn window_resize_by_fit_scale(ui: &mut egui::Ui, image_size: egui::Vec2) {
    let fit_scale = calculate_fit_scale(ui.ctx().screen_rect().size(), image_size);
    window_resize(ui, image_size * fit_scale);
    ui.ctx().request_repaint();
}
