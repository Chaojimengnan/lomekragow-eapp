//! Contains borderless application related utils

use crate::{codicons, widgets::PlainButton};
use eframe::egui::{self, vec2, Color32, CursorIcon, ResizeDirection, UiBuilder, ViewportCommand};

// https://github.com/emilk/egui/pull/3762
pub fn handle_resize(ui: &mut egui::Ui) -> bool {
    let Some(pos) = ui.input(|i| i.pointer.interact_pos()) else {
        return false;
    };

    let screen_rect = ui.ctx().screen_rect();

    // Since this is the outermost layer of the viewport hence we cannot use extend on
    // screen_rect to check if pointer is at an interaction position.
    const SNAP_DIST: f32 = 5.0;

    let east_snap = (screen_rect.right() - pos.x).abs() <= SNAP_DIST;
    let west_snap = !east_snap && (screen_rect.left() - pos.x).abs() <= SNAP_DIST;
    let south_snap = (screen_rect.bottom() - pos.y).abs() <= SNAP_DIST;
    let north_snap = !south_snap && (screen_rect.top() - pos.y).abs() <= SNAP_DIST;

    let possible_resize_direction = match (north_snap, east_snap, west_snap, south_snap) {
        (true, true, false, false) => Some(egui::ResizeDirection::NorthEast),
        (false, true, false, true) => Some(egui::ResizeDirection::SouthEast),
        (true, false, true, false) => Some(egui::ResizeDirection::NorthWest),
        (false, false, true, true) => Some(egui::ResizeDirection::SouthWest),
        (true, false, false, false) => Some(egui::ResizeDirection::North),
        (false, true, false, false) => Some(egui::ResizeDirection::East),
        (false, false, true, false) => Some(egui::ResizeDirection::West),
        (false, false, false, true) => Some(egui::ResizeDirection::South),
        _ => None,
    };

    let Some(resize_direction) = possible_resize_direction else {
        return false;
    };

    fn into_cursor_icon(direction: ResizeDirection) -> CursorIcon {
        match direction {
            ResizeDirection::North => CursorIcon::ResizeNorth,
            ResizeDirection::South => CursorIcon::ResizeSouth,
            ResizeDirection::West => CursorIcon::ResizeWest,
            ResizeDirection::East => CursorIcon::ResizeEast,
            ResizeDirection::NorthEast => CursorIcon::ResizeNorthEast,
            ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
            ResizeDirection::NorthWest => CursorIcon::ResizeNorthWest,
            ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
        }
    }

    ui.output_mut(|o| o.cursor_icon = into_cursor_icon(resize_direction));

    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if !is_maximized && ui.input(|i| i.pointer.primary_pressed()) {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::BeginResize(resize_direction));
        true
    } else {
        false
    }
}

pub fn window_frame(
    ctx: &egui::Context,
    fill: Option<Color32>,
) -> egui::containers::panel::CentralPanel {
    let rounding = if !ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        8.0.into()
    } else {
        0.0.into()
    };

    let frame = egui::Frame {
        fill: fill.unwrap_or(ctx.style().visuals.window_fill()),
        rounding,
        stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        outer_margin: 0.5.into(),
        ..Default::default()
    };

    egui::containers::panel::CentralPanel::default().frame(frame)
}

pub fn title_bar_behavior(ui: &egui::Ui, title_bar_rect: eframe::epaint::Rect) {
    use egui::*;

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar_behavior"),
        Sense::click(),
    );

    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.is_pointer_button_down_on() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

pub fn title_bar(
    ui: &mut egui::Ui,
    title_bar_rect: eframe::epaint::Rect,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    use egui::*;

    title_bar_behavior(ui, title_bar_rect);

    let painter = ui.painter();

    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    ui.allocate_new_ui(UiBuilder::new().max_rect(title_bar_rect), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close_maximize_minimize(
                ui,
                120.0,
                title_bar_rect.height() - 1.0,
                Color32::TRANSPARENT,
            );

            ui.set_clip_rect(title_bar_rect.with_max_x(title_bar_rect.right() - 120.0));

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                add_contents(ui)
            });
        });
    });
}

/// Ignore all everything, just check pointer if in this rect
pub fn rect_contains_pointer(ui: &egui::Ui, rect: eframe::epaint::Rect) -> bool {
    let ptr_pos = ui.input(|i| i.pointer.interact_pos());
    let Some(ptr_pos) = ptr_pos else {
        return false;
    };
    rect.contains(ptr_pos)
}

pub fn title_bar_animated(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect) {
    title_bar_behavior(ui, title_bar_rect);

    let width = 120.0;
    let height = title_bar_rect.height();

    let interact_rect = {
        let mut rect = title_bar_rect;
        rect.set_left(rect.right() - width * 3.0);
        rect.set_bottom(rect.top() + height * 8.0);
        rect
    };

    // cmm : shortcut for close_maximize_minimize
    let opacity = ui.ctx().animate_bool(
        egui::Id::new("cmm_btns_hover_area"),
        rect_contains_pointer(ui, interact_rect),
    );

    if opacity == 0.0 {
        return;
    }

    ui.allocate_new_ui(UiBuilder::new().max_rect(title_bar_rect), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.set_opacity(opacity);
            close_maximize_minimize(ui, width, height, Color32::from_rgb(40, 40, 40));
        });
    });
}

pub fn close_maximize_minimize(
    ui: &mut egui::Ui,
    width_total: f32,
    height: f32,
    fill_color: impl Into<Color32>,
) {
    if ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }

    let width = width_total / 3.0;
    let f_col: Color32 = fill_color.into();
    let new_button = |str| PlainButton::new(vec2(width, height), str);

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        let frame_rect = {
            let mut start = ui.cursor();
            start.set_left(start.right() - width_total);
            start.set_bottom(start.top() + height);
            start
        };

        ui.painter().rect_filled(
            frame_rect,
            egui::Rounding {
                ne: 8.0,
                ..egui::Rounding::ZERO
            },
            f_col,
        );

        if ui
            .add(
                new_button(codicons::ICON_CHROME_CLOSE.to_string())
                    .rounding(egui::Rounding {
                        ne: 8.0,
                        ..egui::Rounding::ZERO
                    })
                    .hover(Color32::from_rgb(200, 5, 5)),
            )
            .clicked()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let text = if is_maximized {
            codicons::ICON_CHROME_RESTORE
        } else {
            codicons::ICON_CHROME_MAXIMIZE
        };

        if ui.add(new_button(text.to_string())).clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
        }

        if ui
            .add(new_button(codicons::ICON_CHROME_MINIMIZE.to_string()))
            .clicked()
        {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
    });
}
