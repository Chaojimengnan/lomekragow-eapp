#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub(crate) mod app;
pub(crate) mod sync;

fn main() {
    eapp_utils::setup_loggers("syncer.log").unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([520.0, 200.0])
            .with_transparent(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/syncer/icon.png"
                ))
                .unwrap(),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "syncer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    ) {
        log::error!("run native fails: {err}");
    }
}
