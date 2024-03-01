//! Contains animation related utils

use eframe::egui::Color32;

pub fn color_lerp(color1: Color32, color2: Color32, factor: f32) -> Color32 {
    fn u8_lerp(v1: u8, v2: u8, factor: f32) -> u8 {
        (v1 as f32 * (1.0 - factor) + v2 as f32 * factor) as u8
    }

    Color32::from_rgba_premultiplied(
        u8_lerp(color1.r(), color2.r(), factor),
        u8_lerp(color1.g(), color2.g(), factor),
        u8_lerp(color1.b(), color2.b(), factor),
        u8_lerp(color1.a(), color2.a(), factor),
    )
}
