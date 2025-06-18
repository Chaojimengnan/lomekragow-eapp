use eframe::egui;

pub mod animation;
pub mod borderless;
pub mod codicons;
pub mod easy_mark;
pub mod natordset;
pub mod platform;
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
    ctx.style_mut(setup_text_size);
}

pub fn setup_text_size(style: &mut egui::Style) {
    for id in &mut style.text_styles.values_mut() {
        id.size = 16.0;
    }
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
