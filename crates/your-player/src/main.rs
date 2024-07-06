#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod danmu;
pub mod mpv;
pub mod playlist;
pub mod tex_register;

// TODO:
// 5. deal with mpv frash rate
// 6. optim danmu code struct

fn main() {
    eapp_utils::setup_loggers("your-player.log").unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([1024.0, 576.0])
            .with_min_inner_size([640.0, 480.0])
            .with_transparent(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../assets/your-player/icon.png"
                ))
                .unwrap(),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "your-player",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    ) {
        log::error!("run native fails: {err}");
    }
}
