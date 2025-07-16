//! Contains widgets related utils

use crate::{
    animation::color_lerp,
    codicons::{ICON_GITHUB, ICON_GITHUB_INVERTED},
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, IntoAtoms, Rect, Sense, Vec2, Widget, WidgetText,
    pos2,
};

/// Just a button, with plain style
pub struct PlainButton {
    text: WidgetText,
    size: Vec2,
    font_size: Option<f32>,
    corner_radius: CornerRadius,
    fill: Option<Color32>,
    hover: Option<Color32>,
}

impl PlainButton {
    pub fn new(size: Vec2, text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            size,
            font_size: None,
            corner_radius: Default::default(),
            fill: None,
            hover: None,
        }
    }

    #[inline]
    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = corner_radius.into();
        self
    }

    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    #[inline]
    pub fn hover(mut self, hover: impl Into<Color32>) -> Self {
        self.hover = Some(hover.into());
        self
    }

    #[inline]
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }
}

impl Widget for PlainButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());

        if ui.is_rect_visible(rect) {
            let hovered = response.hovered();
            let factor = ui.ctx().animate_bool(response.id, hovered);

            let (fill, hover) = {
                (
                    self.fill.unwrap_or(Color32::TRANSPARENT),
                    self.hover.unwrap_or(ui.visuals().widgets.hovered.bg_fill),
                )
            };

            ui.painter()
                .rect_filled(rect, self.corner_radius, color_lerp(fill, hover, factor));

            let text_color = ui.style().visuals.text_color();
            let strong_text_color = ui.style().visuals.strong_text_color();
            let mut font_id = crate::get_body_font_id(ui);
            if let Some(font_size) = self.font_size {
                font_id.size = font_size;
            }

            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.text.text(),
                font_id,
                color_lerp(text_color, strong_text_color, factor),
            );
        }

        response
    }
}

/// same as [`egui::popup::popup_above_or_below_widget`]
/// but with animation
#[allow(clippy::too_many_arguments)]
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
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

pub fn text_in_center_bottom_of_rect(ui: &egui::Ui, text: String, rect: &Rect) {
    let color = ui.visuals().strong_text_color();
    let font_size = crate::get_body_text_size(ui);
    let galley = ui
        .painter()
        .layout(text, FontId::proportional(font_size), color, rect.width());
    let pos = {
        let pos = rect.center_bottom();
        pos2(pos.x - galley.size().x / 2.0, pos.y - galley.size().y)
    };
    ui.painter().rect_filled(
        Rect::from_min_max(pos, pos + galley.size()),
        CornerRadius::ZERO,
        ui.visuals().panel_fill.gamma_multiply(0.8),
    );
    ui.painter().galley(pos, galley, color);
}

pub fn get_theme_button_icon(ui: &egui::Ui) -> String {
    if ui.visuals().dark_mode {
        ICON_GITHUB_INVERTED
    } else {
        ICON_GITHUB
    }
    .to_string()
}

pub fn get_theme_button(ui: &egui::Ui) -> egui::Button<'static> {
    egui::Button::new(get_theme_button_icon(ui)).frame(false)
}

pub fn theme_button<Btn: Widget>(ui: &mut egui::Ui, btn: Btn) -> egui::Response {
    let response = ui.add(btn);
    if response.clicked() {
        ui.ctx()
            .set_theme(egui::Theme::from_dark_mode(!ui.visuals().dark_mode));
    }

    response
}

pub fn auto_selectable<Value>(
    ui: &mut egui::Ui,
    current_value: &mut Value,
    selected_value: Value,
    text: &str,
    extra_scroll_cod: bool,
) -> egui::Response
where
    Value: PartialEq,
{
    let cur_select = *current_value == selected_value;
    let res = ui.selectable_value(current_value, selected_value, text);
    if cur_select && extra_scroll_cod {
        res.scroll_to_me(None);
    };

    res
}

pub fn frameless_btn<'a>(ui: &mut egui::Ui, text: impl IntoAtoms<'a>) -> egui::Response {
    ui.selectable_label(false, text)
}
