#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod codec;

// TODO: 1. impl search

fn main() {
    eapp_utils::setup_loggers("lonote.log").unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([520.0, 200.0])
            .with_transparent(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/lonote/icon.png"
                ))
                .unwrap(),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "lonote",
        options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    ) {
        log::error!("run native fails: {err}");
    }
}
