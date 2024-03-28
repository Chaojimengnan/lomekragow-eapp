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

/// same as [`egui::popup::popup_above_or_below_widget`]
/// but with animation
pub fn popup_animated(
    ui: &mut egui::Ui,
    mut open: bool,
    parent_opacity: f32,
    popup_id: egui::Id,
    widget_response: &egui::Response,
    above_or_below: egui::AboveOrBelow,
    pivot: Align2,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> bool {
    let opacity = ui.ctx().animate_bool(popup_id, open).min(parent_opacity);

    if opacity != 0.0 {
        let pos = match above_or_below {
            egui::AboveOrBelow::Above => widget_response.rect.left_top(),
            egui::AboveOrBelow::Below => widget_response.rect.left_bottom(),
        };

        let area = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .constrain(true)
            .fixed_pos(pos)
            .pivot(pivot)
            .show(ui.ctx(), |ui| {
                ui.set_opacity(opacity);
                let frame = egui::Frame::popup(ui.style());
                frame.show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        add_contents(ui)
                    })
                })
            })
            .response;

        if ui.input(|i| i.key_pressed(egui::Key::Escape))
            || (widget_response.clicked_elsewhere() && area.clicked_elsewhere())
        {
            open = false;
        }
    }

    open
}

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}
