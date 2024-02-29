use eframe::egui::{self, Color32, CursorIcon, ResizeDirection, Vec2, ViewportCommand};

pub mod codicons;
pub mod easy_mark;

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

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    include_flate::flate!(static UNIFONT: [u8] from "../../assets/unifont-15.1.04.otf");
    include_flate::flate!(static CODICON: [u8] from "../../assets/codicon.ttf");

    fonts
        .font_data
        .insert("unifont".to_owned(), egui::FontData::from_static(&UNIFONT));
    fonts
        .font_data
        .insert("codicon".to_owned(), egui::FontData::from_static(&CODICON));

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .append(&mut vec!["unifont".to_owned(), "codicon".to_owned()]);

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .append(&mut vec!["unifont".to_owned(), "codicon".to_owned()]);

    ctx.set_fonts(fonts);

    let mut style = ctx.style().as_ref().clone();
    for (_, id) in &mut style.text_styles {
        id.size = 16.0;
    }
    ctx.set_style(style);
}

pub fn window_frame(ctx: &egui::Context) -> egui::containers::panel::CentralPanel {
    let frame = egui::Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 8.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        outer_margin: 0.5.into(),
        ..Default::default()
    };

    egui::containers::panel::CentralPanel::default().frame(frame)
}

pub fn title_bar_behavior(ui: &egui::Ui, title_bar_rect: eframe::epaint::Rect) {
    use egui::*;

    let title_bar_response = ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click());
    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.is_pointer_button_down_on() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

pub fn title_bar(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    use egui::*;

    title_bar_behavior(ui, title_bar_rect);

    let painter = ui.painter();
    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(16.0),
        ui.style().visuals.text_color(),
    );

    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.visuals_mut().button_frame = false;

            ui.add_space(4.0);
            close_maximize_minimize(ui);
        });
    });
}

pub fn close_maximize_minimize(ui: &mut egui::Ui) {
    use egui::{Button, RichText};

    let mut temp_col = Color32::RED;
    std::mem::swap(
        &mut temp_col,
        &mut ui.style_mut().visuals.widgets.hovered.fg_stroke.color,
    );

    let close_response = ui
        .add(
            Button::new(RichText::new(codicons::ICON_CHROME_CLOSE).size(16.0))
                .min_size(Vec2::new(40.0, 32.0)),
        )
        .on_hover_text("Close the window");
    if close_response.clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }

    std::mem::swap(
        &mut temp_col,
        &mut ui.style_mut().visuals.widgets.hovered.fg_stroke.color,
    );

    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let text = if is_maximized {
        ("Restore window", codicons::ICON_CHROME_RESTORE)
    } else {
        ("Maximize window", codicons::ICON_CHROME_MAXIMIZE)
    };

    let maximized_response = ui
        .add(Button::new(RichText::new(text.1).size(16.0)).min_size(Vec2::new(40.0, 32.0)))
        .on_hover_text(text.0);
    if maximized_response.clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    let minimized_response = ui
        .add(
            Button::new(RichText::new(codicons::ICON_CHROME_MINIMIZE).size(16.0))
                .min_size(Vec2::new(40.0, 32.0)),
        )
        .on_hover_text("Minimize the window");
    if minimized_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}
