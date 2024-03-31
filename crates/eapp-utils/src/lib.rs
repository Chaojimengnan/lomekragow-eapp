use eframe::egui;

pub mod animation;
pub mod borderless;
pub mod codicons;
pub mod easy_mark;
pub mod platform;
pub mod widgets;

#[macro_export]
macro_rules! capture_error {
    ($i:ident, $handler:block, $block_to_capture:block) => {
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

    fonts
        .font_data
        .insert("unifont".to_owned(), egui::FontData::from_static(&UNIFONT));
    fonts
        .font_data
        .insert("codicon".to_owned(), egui::FontData::from_static(&CODICON));

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .append(&mut vec!["unifont".to_owned(), "codicon".to_owned()]);

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .append(&mut vec!["unifont".to_owned(), "codicon".to_owned()]);

    ctx.set_fonts(fonts);

    let mut style = ctx.style().as_ref().clone();
    for (_, id) in &mut style.text_styles {
        id.size = 16.0;
    }
    ctx.set_style(style);
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
                File::create(std::env::current_exe()?.join(format!("../{log_filename}")))?,
            ),
        ])?;
    }

    #[cfg(not(debug_assertions))]
    WriteLogger::init(
        LevelFilter::Warn,
        Config::default(),
        File::create(std::env::current_exe()?.join(format!("../{log_filename}")))?,
    )?;

    Ok(())
}
