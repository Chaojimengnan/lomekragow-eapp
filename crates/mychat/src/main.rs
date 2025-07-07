#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod chat;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    eapp_utils::setup_loggers("mychat.log").unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([520.0, 200.0])
            .with_transparent(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/mychat/icon.png"
                ))
                .unwrap(),
            ),
        ..Default::default()
    };

    rt.block_on(async {
        if let Err(err) = eframe::run_native(
            "mychat",
            options,
            Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
        ) {
            log::error!("run native fails: {err}");
        }
    });
}
