//! Contains widgets related utils

use crate::animation::color_lerp;
use eframe::egui::{self, Align2, Color32, FontId, Rounding, Sense, Vec2, Widget, WidgetText};

/// Just a button, with plain style
pub struct PlainButton {
    text: WidgetText,
    size: Vec2,
    font_size: f32,
    rounding: Rounding,
    fill: Color32,
    hover: Color32,
}

impl PlainButton {
    pub fn new(size: Vec2, text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            size,
            font_size: 16.0,
            rounding: Default::default(),
            fill: Default::default(),
            hover: Color32::DARK_GRAY,
        }
    }

    #[inline]
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = rounding.into();
        self
    }

    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = fill.into();
        self
    }

    #[inline]
    pub fn hover(mut self, hover: impl Into<Color32>) -> Self {
        self.hover = hover.into();
        self
    }

    #[inline]
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }
}

impl Widget for PlainButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());

        if ui.is_rect_visible(rect) {
            let hovered = response.hovered();
            let factor = ui.ctx().animate_bool(response.id, hovered);

            ui.painter().rect_filled(
                rect,
                self.rounding,
                color_lerp(self.fill, self.hover, factor),
            );

            let text_color = ui.style().visuals.text_color();
            let strong_text_color = ui.style().visuals.strong_text_color();

            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.text.text(),
                FontId::proportional(self.font_size),
                color_lerp(text_color, strong_text_color, factor),
            );
        }

        response
    }
}
