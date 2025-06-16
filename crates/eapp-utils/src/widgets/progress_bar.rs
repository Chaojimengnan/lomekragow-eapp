//! ProgressBar widget impl

use eframe::egui::{
    Align2, Area, Color32, CornerRadius, Order, Rect, Response, Sense, Stroke, Ui, Widget, pos2,
    vec2,
};

/// A customizable progress bar widget for egui
///
/// Features:
/// - Smooth progress visualization
/// - Interactive dragging for value adjustment
/// - Hover preview with custom content
/// - Customizable colors and styling
///
/// # Example
/// ```
/// ProgressBar::new(current_time, duration)
///     .height(24.0)
///     .background_color(Color32::DARK_GRAY)
///     .fill_color(Color32::LIGHT_GREEN)
///     .active_color(Color32::YELLOW)
///     .preview(|ui, value| {
///         ui.label(format!("Preview: {:.1}s", value));
///     })
///     .ui(ui);
/// ```
pub struct ProgressBar<'a> {
    /// Current value of the progress bar
    value: f64,

    /// Maximum value (represents 100% progress)
    max: f64,

    /// Height of the progress bar track
    height: f32,

    /// Background color of the track
    background_color: Color32,

    /// Color of the filled portion
    fill_color: Color32,

    /// Color used when actively dragging
    active_color: Color32,

    /// Radius of the draggable knob
    knob_radius: f32,

    /// Whether to show the draggable knob
    show_knob: bool,

    /// Preview callback function (shown on hover)
    preview: Option<PreviewCallback<'a>>,
}

type PreviewCallback<'a> = Box<dyn FnMut(&mut Ui, f64) + 'a>;

impl<'a> ProgressBar<'a> {
    /// Creates a new progress bar with default styling
    ///
    /// # Arguments
    /// * `value` - Current progress value
    /// * `max` - Maximum value (100% progress)
    pub fn new(value: f64, max: f64) -> Self {
        Self {
            value,
            max,
            height: 16.0,
            background_color: Color32::from_rgba_premultiplied(100, 100, 100, 106),
            fill_color: Color32::LIGHT_BLUE,
            active_color: Color32::LIGHT_BLUE,
            knob_radius: 7.0,
            show_knob: true,
            preview: None,
        }
    }

    /// Sets the height of the progress bar track
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets the background color of the track
    pub fn background_color(mut self, color: impl Into<Color32>) -> Self {
        self.background_color = color.into();
        self
    }

    /// Sets the color of the filled progress portion
    pub fn fill_color(mut self, color: impl Into<Color32>) -> Self {
        self.fill_color = color.into();
        self
    }

    /// Sets the color used when actively dragging the progress bar
    pub fn active_color(mut self, color: impl Into<Color32>) -> Self {
        self.active_color = color.into();
        self
    }

    /// Sets the radius of the draggable knob
    pub fn knob_radius(mut self, radius: f32) -> Self {
        self.knob_radius = radius;
        self
    }

    /// Controls whether the draggable knob is visible
    pub fn show_knob(mut self, show: bool) -> Self {
        self.show_knob = show;
        self
    }

    /// Sets a preview callback that shows when hovering over the progress bar
    ///
    /// The callback receives:
    /// - `&mut Ui` for drawing content
    /// - `f64` representing the value at the hover position
    pub fn preview<F: FnMut(&mut Ui, f64) + 'a>(mut self, callback: F) -> Self {
        self.preview = Some(Box::new(callback));
        self
    }
}

impl<'a> Widget for ProgressBar<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // Allocate space for the progress bar
        let desired_size = vec2(ui.available_width(), self.height);
        let (_, response) = ui.allocate_exact_size(desired_size, Sense::drag());
        let rect = response.rect;

        // Only draw if visible
        if ui.is_rect_visible(rect) {
            let is_active = response.dragged();
            let fill_color = if is_active {
                self.active_color
            } else {
                self.fill_color
            };

            // Draw background track
            ui.painter().line_segment(
                [rect.left_center(), rect.right_center()],
                Stroke::new(3.0, self.background_color),
            );

            // Draw filled portion if we have valid values
            if self.max > 0.0 {
                let progress_fraction = (self.value / self.max) as f32;
                let fill_width = progress_fraction * rect.width();
                let fill_end = rect.left() + fill_width;

                // Draw filled progress
                ui.painter().line_segment(
                    [rect.left_center(), pos2(fill_end, rect.center().y)],
                    Stroke::new(3.0, fill_color),
                );

                // Draw draggable knob
                if self.show_knob {
                    ui.painter().circle_filled(
                        pos2(fill_end, rect.center().y),
                        self.knob_radius,
                        fill_color,
                    );
                }
            }

            // Show preview tooltip on hover
            if let Some(ref mut preview_callback) = self.preview {
                if let Some(pointer) = response.hover_pos() {
                    let value = value_from_x(self.max, rect, pointer.x as _);
                    let preview_pos = pos2(pointer.x, rect.top() - 10.0);

                    Area::new("progress_bar_preview_area".into())
                        .order(Order::Tooltip)
                        .fixed_pos(preview_pos)
                        .pivot(Align2::CENTER_BOTTOM)
                        .show(ui.ctx(), |ui| {
                            preview_callback(ui, value);
                        });
                }
            }
        }

        response
    }
}

/// Converts a horizontal position within the progress bar to a value
///
/// Useful for custom interactions where you need to translate
/// pointer positions to progress values
pub fn value_from_x(max: f64, rect: Rect, x: f64) -> f64 {
    if rect.width() <= 0.0 {
        return 0.0;
    }

    let fraction = ((x - rect.left() as f64) / rect.width() as f64).clamp(0.0, 1.0);
    fraction * max
}

/// Draws the progress bar background with gradient effect
///
/// This includes:
/// - Bottom panel with rounded corners
/// - Top gradient overlay
///
/// # Arguments
/// * `ui` - Egui UI context
/// * `rect` - Rectangle to draw in
/// * `background_color` - Color for the bottom panel
/// * `corner_radius` - Radius for the bottom panel corners
pub fn draw_progress_bar_background(
    ui: &mut Ui,
    rect: Rect,
    background_color: Color32,
    corner_radius: CornerRadius,
) {
    // Draw bottom background panel
    let painter = ui.painter();
    painter.rect_filled(
        {
            let mut rect = rect;
            rect.set_top(rect.bottom() - 16.0);
            rect
        },
        corner_radius,
        background_color,
    );

    // Prepare gradient area
    let mesh_rect = {
        let mut r = rect;
        r.set_bottom(r.bottom() - 16.0);
        r
    };

    // Define gradient colors
    let mesh_top_color = Color32::TRANSPARENT;
    let mesh_bottom_color = background_color;

    // Create and add gradient mesh
    let mut mesh = eframe::egui::Mesh::default();
    mesh.colored_vertex(mesh_rect.left_top(), mesh_top_color);
    mesh.colored_vertex(mesh_rect.right_top(), mesh_top_color);
    mesh.colored_vertex(mesh_rect.left_bottom(), mesh_bottom_color);
    mesh.colored_vertex(mesh_rect.right_bottom(), mesh_bottom_color);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}
