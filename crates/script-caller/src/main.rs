#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub(crate) mod app;
pub(crate) mod script;

fn main() {
    eapp_utils::setup_loggers("script-caller.log").unwrap();

    eapp_utils::capture_error!(err => log::error!("run native fails: {err}"), {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_inner_size([720.0, 480.0])
                .with_min_inner_size([640.0, 320.0])
                .with_transparent(true)
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!(
                        "../../../assets/script-caller/icon.png"
                    ))
                    .unwrap(),
                ),
            ..Default::default()
        };

        eframe::run_native(
            "script-caller",
            options,
            Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
        )?;
    });
}
