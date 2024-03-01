//! Contains widgets related utils

use crate::animation::color_lerp;
use eframe::egui::{self, Color32};

/// Just a button, with plain style
pub fn plain_button(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    height: f32,
    rounding: impl Into<egui::Rounding>,
    fill_color: impl Into<Color32>,
    hover_color: impl Into<Color32>,
) -> egui::Response {
    use egui::*;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let f_col: Color32 = fill_color.into();
    let h_col: Color32 = hover_color.into();

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let factor = ui.ctx().animate_bool(response.id, hovered);

        ui.painter()
            .rect_filled(rect, rounding, color_lerp(f_col, h_col, factor));

        let text_color = ui.style().visuals.text_color();
        let strong_text_color = ui.style().visuals.strong_text_color();

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(16.0),
            color_lerp(text_color, strong_text_color, factor),
        );
    }

    response
}
