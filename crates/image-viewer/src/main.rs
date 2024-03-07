#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod img_finder;
pub mod lifo;
pub mod tex_loader;

fn main() {
    eapp_utils::setup_loggers("image-viewer.log").unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([480.0, 480.0])
            .with_min_inner_size([480.0, 480.0])
            .with_transparent(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/image-viewer/icon.png"
                ))
                .unwrap(),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "image-viewer",
        options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    ) {
        log::error!("run native fails: {err}");
    }
}
