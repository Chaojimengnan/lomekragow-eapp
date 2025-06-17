use crate::{img_finder::ImgFinder, img_translation::ImgTranslation, tex_loader::TexLoader};
use eapp_utils::{
    borderless,
    codicons::{
        ICON_INSPECT, ICON_NEW_FILE, ICON_REFRESH, ICON_TRIANGLE_LEFT, ICON_TRIANGLE_RIGHT,
    },
    widgets::{
        progress_bar::{ProgressBar, draw_progress_bar_background, value_from_x},
        simple_widgets::{
            PlainButton, get_theme_button, text_in_center_bottom_of_rect, theme_button,
        },
    },
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Rect, UiBuilder, Vec2b,
    Widget as _, pos2, vec2,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct State {
    search_key: String,
    left_panel_open: bool,
    #[serde(skip)]
    is_cur_image_loading: bool,
    #[serde(skip)]
    last_cur_dir: Option<usize>,
    #[serde(skip)]
    last_image_name: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            left_panel_open: true,
            is_cur_image_loading: true,
            last_cur_dir: None,
            search_key: String::default(),
            last_image_name: None,
        }
    }
}

pub struct App {
    state: State,
    img_finder: ImgFinder,
    tex_loader: TexLoader,
    translation: ImgTranslation,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.style_mut(|style| style.animation_time = 0.11);

        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };

        let img_path = std::env::args().nth(1);

        let mut img_finder = ImgFinder::new();
        img_finder = Self::search_from_cwd(img_finder, img_path.as_deref());
        img_finder.consume_dir_changed_flag();

        let tex_loader = TexLoader::new(&cc.egui_ctx);
        let translation = ImgTranslation::default();

        Self {
            state,
            img_finder,
            tex_loader,
            translation,
        }
    }

    fn search_from_cwd(img_finder: ImgFinder, image_path: Option<&str>) -> ImgFinder {
        match img_finder.search_from_cwd(image_path) {
            Ok(v) => v,
            Err(err) => {
                log::error!("load from cmd with arg '{image_path:?}' fails: {err}");
                ImgFinder::new()
            }
        }
    }

    fn adjust_corner_radius_match_left_panel(&self, corner_radius: CornerRadius) -> CornerRadius {
        let mut corner_radius = corner_radius;
        if self.state.left_panel_open {
            corner_radius.nw = 0;
            corner_radius.sw = 0;
        }
        corner_radius
    }

    fn spawn(&self) {
        eapp_utils::capture_error!(err => log::error!("spawn error: {err}"), {
            let mut cmd = std::process::Command::new(std::env::current_exe()?);
            if let Some(cur_image_name) = self.img_finder.cur_image_name() {
                cmd.arg(cur_image_name);
            }
            cmd.spawn()?;
        });
    }

    fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.5;

        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .frame(
                Frame::side_top_panel(ui.style()).corner_radius(CornerRadius {
                    nw: 8,
                    sw: 8,
                    ..egui::CornerRadius::ZERO
                }),
            )
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.state.left_panel_open, |ui| {
                theme_button(ui, get_theme_button(ui));

                ui.add(
                    egui::TextEdit::singleline(&mut self.state.search_key)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search keywords"),
                );

                egui::ScrollArea::both()
                    .auto_shrink(Vec2b::new(false, true))
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                        let dir_prefix = self
                            .img_finder
                            .search_dir()
                            .map(|str| str.len() + 1)
                            .unwrap_or(0);
                        let mut cur_dir = self.img_finder.cur_dir();

                        self.state.last_cur_dir = cur_dir;

                        for (dir, dir_name) in self.img_finder.cur_dir_set().iter().enumerate() {
                            let dir_str = if dir_name.len() != dir_prefix - 1 {
                                &dir_name[dir_prefix..]
                            } else {
                                "current directory"
                            };

                            if !self.state.search_key.is_empty()
                                && !dir_str
                                    .to_ascii_lowercase()
                                    .contains(&self.state.search_key)
                            {
                                continue;
                            }

                            let is_cur_dir = cur_dir == Some(dir);
                            let str = if is_cur_dir {
                                egui::RichText::new(dir_str).color(ui.visuals().strong_text_color())
                            } else {
                                egui::RichText::new(dir_str)
                            };

                            if ui
                                .selectable_label(is_cur_dir, str)
                                .on_hover_text(dir_str)
                                .clicked()
                            {
                                cur_dir = Some(dir);
                            };
                        }

                        if let Some(dir) = cur_dir {
                            self.img_finder.set_cur_dir_idx(dir);
                        }
                    });
            });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show_inside(ui, |ui| {
                let app_rect = ui.max_rect();

                let rect_contains = borderless::rect_contains_pointer(ui, app_rect);
                let no_focuse = ui.memory(|m| m.focused().is_none());
                if rect_contains && no_focuse {
                    self.handle_scroll_and_drag(ui);
                }

                self.process_inputs(ui);
                self.ui_show_cur_image(ui, app_rect);

                let title_bar_height = 28.0;
                let title_bar_rect = {
                    let mut rect = app_rect;
                    rect.max.y = rect.min.y + title_bar_height;
                    rect
                };
                borderless::title_bar_animated(ui, title_bar_rect);

                let size = 20.0;
                let left_panel_button_rect = Rect::from_center_size(
                    pos2(app_rect.left() + size / 2.0 + 8.0, app_rect.left_center().y),
                    vec2(size, size),
                );
                let left_panel_button_sense_rect = {
                    let mut rect = app_rect;
                    rect.set_right(rect.left() + size * 10.0);
                    rect.set_top(app_rect.center().y - size * 10.0);
                    rect.set_bottom(app_rect.center().y + size * 10.0);
                    rect
                };
                self.ui_left_panel_button(ui, left_panel_button_rect, left_panel_button_sense_rect);

                let info_total_rect = {
                    let mut rect = app_rect;
                    rect.set_top(rect.bottom() - 120.0);
                    rect.shrink2(vec2(32.0, 0.0))
                };
                let info_total_sense_rect = {
                    let mut rect = app_rect;
                    rect.set_top(rect.bottom() - rect.height() * 0.35);

                    if self.state.left_panel_open {
                        rect.translate(vec2(0.5, 0.0))
                    } else {
                        rect.expand2(vec2(0.5, 0.0))
                    }
                };
                self.ui_info(ui, info_total_rect, info_total_sense_rect);
            });
    }

    fn ui_show_cur_image(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let show_center_text = |text| {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(16.0),
                ui.visuals().text_color(),
            )
        };
        let opacity = ui.ctx().animate_bool(
            Id::new("state.is_cur_image_loading"),
            !self.state.is_cur_image_loading,
        );

        if let Some(cur_image_name) = self.img_finder.cur_image_name() {
            self.tex_loader.load(cur_image_name);

            if let Some(texture) = self.tex_loader.textures().get(cur_image_name).unwrap() {
                self.state.is_cur_image_loading = false;

                use crate::tex_loader::Texture::*;
                let handle = match texture {
                    Static(handle) => handle,
                    Animated(animated) => &animated.frames[animated.current].0,
                };

                let image_size = handle.size_vec2();
                let available_size = rect.size();

                let fit_scale = {
                    let width_scale = available_size.x / image_size.x;
                    let height_scale = available_size.y / image_size.y;
                    width_scale.min(height_scale)
                };
                self.translation.min_scale = fit_scale.min(1.0);
                self.translation.scale = self.translation.scale.max(self.translation.min_scale);

                if self.translation.image_fit_space_size {
                    self.translation.scale = fit_scale;
                    self.translation.image_fit_space_size = false;
                }

                let scaled_size = image_size * self.translation.scale;

                self.translation.image_exceeds_space = (
                    scaled_size.x > available_size.x,
                    scaled_size.y > available_size.y,
                );

                let current_offset = self.translation.image_offset;

                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if let Some(scale_old) = self.translation.scale_old_for_calculate {
                        let image_pos =
                            rect.center() - (image_size * scale_old) * 0.5 + current_offset;
                        let mouse_in_image = (mouse_pos - image_pos) / (image_size * scale_old);
                        let image_pos_new = rect.center() - scaled_size * 0.5 + current_offset;
                        let target_pos = image_pos_new + mouse_in_image * scaled_size;
                        let offset_delta = mouse_pos - target_pos;

                        self.translation.image_offset = current_offset + offset_delta;
                        self.translation.scale_old_for_calculate = None;
                    }
                }

                self.translation.max_offset =
                    ((scaled_size - available_size) * 0.5).max(egui::Vec2::ZERO);
                self.translation.image_offset =
                    self.translation.clamp_offset(self.translation.image_offset);

                let tex = egui::Image::from_texture(handle)
                    .show_loading_spinner(false)
                    .maintain_aspect_ratio(true)
                    .fit_to_exact_size(scaled_size);

                let image_pos = rect.center() - scaled_size * 0.5 + self.translation.image_offset;
                let image_rect = Rect::from_min_size(image_pos, scaled_size);

                let diff = available_size - scaled_size;
                let corner_radius = if diff.x <= 16.0 && diff.y <= 16.0 {
                    CornerRadius::same(8)
                } else {
                    CornerRadius::same(0)
                };

                tex.corner_radius(self.adjust_corner_radius_match_left_panel(corner_radius))
                    .tint(Color32::WHITE.linear_multiply(opacity))
                    .paint_at(ui, image_rect);

                if self.translation.is_dragging {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            } else {
                self.state.is_cur_image_loading = true;
                show_center_text("Maiden in Prayer...");
            }
        } else {
            show_center_text("manga-reader :)");
        }
    }

    fn ui_left_panel_button(
        &mut self,
        ui: &mut egui::Ui,
        btn_rect: eframe::epaint::Rect,
        sense_rect: eframe::epaint::Rect,
    ) {
        let btn_text = if self.state.left_panel_open {
            ICON_TRIANGLE_LEFT
        } else {
            ICON_TRIANGLE_RIGHT
        };

        let opacity = ui.ctx().animate_bool(
            Id::new("left_panel_button_hover_area"),
            borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.allocate_new_ui(UiBuilder::new().max_rect(btn_rect), |ui| {
            ui.set_opacity(opacity);
            if ui
                .add(
                    PlainButton::new(
                        vec2(btn_rect.width(), btn_rect.height()),
                        btn_text.to_string(),
                    )
                    .corner_radius(CornerRadius::same(9)),
                )
                .clicked()
            {
                self.state.left_panel_open = !self.state.left_panel_open;
            }
        });
    }

    fn ui_info(
        &mut self,
        ui: &mut egui::Ui,
        rect: eframe::epaint::Rect,
        sense_rect: eframe::epaint::Rect,
    ) {
        let opacity = ui.ctx().animate_bool(
            Id::new("info_hover_area"),
            borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.set_opacity(opacity);

        let bg_rect = {
            let mut rect = sense_rect;
            rect.set_top(rect.bottom() - 190.0);
            rect
        };

        let corner_radius = self.adjust_corner_radius_match_left_panel(CornerRadius {
            se: 8,
            sw: 8,
            ..egui::CornerRadius::ZERO
        });

        draw_progress_bar_background(ui, bg_rect, ui.visuals().extreme_bg_color, corner_radius);

        let mut name = "None".to_owned();
        let mut page_info = "None".to_owned();
        let mut size_info = "? x ?".to_owned();
        let total_pages = self.img_finder.cur_image_set().0.len();
        let current_page = self.img_finder.cur_image().unwrap_or(0);

        if let Some(img) = self.img_finder.cur_image() {
            let prefix = self.img_finder.search_dir().unwrap().len() + 1;
            let img_name = self.img_finder.cur_image_name().unwrap();
            name = img_name[prefix..].to_owned();

            page_info = format!("{} / {}", img + 1, total_pages);

            if let Some(texture) = self.tex_loader.textures().get(img_name).unwrap() {
                use crate::tex_loader::Texture::*;
                let size_number = match texture {
                    Static(v) => v.size(),
                    Animated(v) => v.frames[v.current].0.size(),
                };
                size_info = format!(
                    "{} x {} ({:.0}%)",
                    size_number[0],
                    size_number[1],
                    self.translation.scale * 100.0
                );
            }
        }

        ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.visuals_mut().override_text_color = Some(ui.visuals().strong_text_color());

            ui.advance_cursor_after_rect(Rect::from_min_max(
                pos2(rect.left(), rect.top()),
                pos2(rect.right(), rect.bottom() - 92.0),
            ));

            ui.style_mut().spacing.item_spacing = vec2(6.0, 12.0);

            ui.add(egui::Label::new(name).wrap_mode(egui::TextWrapMode::Truncate));

            let response = ProgressBar::new((current_page + 1) as f64, total_pages as f64)
                .height(16.0)
                .knob_radius(7.0)
                .preview(|ui, hover_img| {
                    let new_page = (hover_img as usize).min(total_pages.saturating_sub(1));
                    let size = vec2(256.0, 256.0);
                    let (_, rect) = ui.allocate_space(size);

                    if let Some(img_name) = self.img_finder.image_at(new_page) {
                        if let Some(Some(texture)) = self.tex_loader.textures().get(img_name) {
                            use crate::tex_loader::Texture::*;
                            let handle = match texture {
                                Static(handle) => handle,
                                Animated(animated) => &animated.frames[animated.current].0,
                            };

                            let image = egui::Image::from_texture(handle)
                                .max_size(vec2(256.0, 256.0))
                                .corner_radius(4);
                            let image_size = image.calc_size(size, image.size());
                            let center = pos2(
                                rect.center().x,
                                rect.center().y + (256.0 - image_size.y) / 2.0,
                            );
                            image.paint_at(ui, Rect::from_center_size(center, image_size));
                        } else {
                            self.tex_loader.load(img_name);
                        }
                    }

                    let text = format!("{} / {}", new_page + 1, total_pages);
                    text_in_center_bottom_of_rect(ui, text, &rect);
                })
                .ui(ui);

            let progress_bar_rect = response.rect;

            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(page_info);
                });

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(size_info);
                });
            });

            let btn_size = vec2(32.0, 32.0);
            let rect_size = vec2(
                btn_size.x * 3.0 + ui.style_mut().spacing.item_spacing.x * 2.0,
                btn_size.y,
            );

            let rect =
                Rect::from_center_size(pos2(rect.center().x, rect.bottom() - 22.0), rect_size);

            ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal(|ui| {
                    let hover_color = ui.visuals().selection.bg_fill;

                    if PlainButton::new(btn_size, ICON_NEW_FILE.to_string())
                        .corner_radius(CornerRadius::same(2))
                        .hover(hover_color)
                        .ui(ui)
                        .on_hover_text("Spawn from this")
                        .clicked()
                    {
                        self.spawn();
                    }

                    if PlainButton::new(btn_size, ICON_REFRESH.to_string())
                        .corner_radius(CornerRadius::same(2))
                        .hover(hover_color)
                        .ui(ui)
                        .on_hover_text("Reset image translation")
                        .clicked()
                    {
                        self.translation.scale = 1.0;
                        self.translation.image_offset = egui::Vec2::ZERO;
                        ui.ctx().request_repaint();
                    }

                    if PlainButton::new(btn_size, ICON_INSPECT.to_string())
                        .corner_radius(CornerRadius::same(2))
                        .hover(hover_color)
                        .ui(ui)
                        .on_hover_text("Fit the image size with the available space size")
                        .clicked()
                    {
                        self.translation.image_fit_space_size = true;
                        ui.ctx().request_repaint();
                    }
                });
            });

            if response.dragged() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let new_page =
                        value_from_x(total_pages as f64, progress_bar_rect, pointer.x as f64)
                            as usize;

                    let new_page = new_page.min(total_pages.saturating_sub(1));
                    self.img_finder.set_cur_image_idx(new_page);

                    for page in new_page.saturating_sub(3)..=new_page.saturating_add(3) {
                        if page < total_pages {
                            if let Some(img_name) = self.img_finder.image_at(page) {
                                self.tex_loader.load(img_name);
                            }
                        }
                    }
                }
            }
        });
    }

    fn handle_scroll_and_drag(&mut self, ui: &mut egui::Ui) {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);

        let zoom_delta = if scroll_delta != 0.0 {
            scroll_delta * 0.005
        } else {
            0.0
        };

        let no_need_to_zoom_out = self.translation.image_fully_contained()
            && zoom_delta < 0.0
            && self.translation.scale < 1.0;

        if zoom_delta != 0.0 && !no_need_to_zoom_out {
            self.translation.scale_old_for_calculate = Some(self.translation.scale);
            self.translation.scale =
                (self.translation.scale + zoom_delta).clamp(self.translation.min_scale, 5.0);
            ui.ctx().request_repaint();
        }

        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let is_over_image = self.img_finder.cur_image_name().is_some();

            if ui.input(|i| i.pointer.primary_pressed()) && is_over_image {
                let can_drag_x = self.translation.image_exceeds_space.0;
                let can_drag_y = self.translation.image_exceeds_space.1;

                if can_drag_x || can_drag_y {
                    self.translation.is_dragging = true;
                    self.translation.drag_start_offset = self.translation.image_offset;
                }
            }

            if self.translation.is_dragging {
                if let Some(click_pos) = ui.input(|i| i.pointer.press_origin()) {
                    let mut delta = pos - click_pos;

                    if !self.translation.image_exceeds_space.0 {
                        delta.x = 0.0;
                    }
                    if !self.translation.image_exceeds_space.1 {
                        delta.y = 0.0;
                    }

                    self.translation.image_offset = self
                        .translation
                        .clamp_offset(self.translation.drag_start_offset + delta);
                    ui.ctx().request_repaint();
                }

                if ui.input(|i| i.pointer.primary_released()) {
                    self.translation.is_dragging = false;
                }
            }
        } else if self.translation.is_dragging {
            self.translation.is_dragging = false;
        }
    }

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if ui.memory(|mem| mem.focused().is_none()) {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.img_finder.prev_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.img_finder.next_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.img_finder.prev_image();

                if let Some(cur_image) = self.img_finder.cur_image() {
                    for item in self
                        .img_finder
                        .image_iter()
                        .skip(cur_image.saturating_sub(3))
                        .take(3)
                    {
                        self.tex_loader.load(item);
                    }
                }
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.img_finder.next_image();
                if let Some(cur_image) = self.img_finder.cur_image() {
                    for item in self
                        .img_finder
                        .image_iter()
                        .skip(cur_image + 1)
                        .take(3)
                        .rev()
                    {
                        self.tex_loader.load(item);
                    }
                }
            }
        }

        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(path) = &i.raw.dropped_files.first().unwrap().path {
                    let cwd = if path.is_dir() {
                        path
                    } else {
                        path.parent().unwrap()
                    };

                    if let Err(err) = std::env::set_current_dir(cwd) {
                        log::error!("set current dir '{cwd:?}' fails: {err}");
                    }

                    self.img_finder = Self::search_from_cwd(
                        std::mem::take(&mut self.img_finder),
                        Some(path.to_string_lossy().as_ref()),
                    );
                }
            }
        });

        if self.img_finder.consume_dir_changed_flag() {
            self.tex_loader.forget_all();
        }

        if let Some(cur_image) = self.img_finder.cur_image_name() {
            if self.state.last_image_name.as_deref() != Some(cur_image) {
                self.state.last_image_name = Some(cur_image.to_string());
                self.translation.reset_translation();
            }
        } else {
            self.state.last_image_name = None;
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.extreme_bg_color)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            self.tex_loader
                .update(ctx, self.img_finder.cur_image_name());

            self.ui_left_panel(ui);
            self.ui_contents(ui);
        });
    }
}
