use crate::{
    img_finder::ImgFinder,
    img_utils::{ImgTranslation, InitialScalingMode, LastImageInfo},
    tex_loader::TexLoader,
};
use eapp_utils::{
    borderless,
    codicons::{
        ICON_COFFEE, ICON_FOLDER, ICON_GO_TO_FILE, ICON_INSPECT, ICON_NEW_FILE, ICON_REFRESH,
        ICON_SCREEN_FULL, ICON_SCREEN_NORMAL, ICON_TRIANGLE_LEFT, ICON_TRIANGLE_RIGHT,
    },
    get_body_font_id, get_body_text_size, get_button_height,
    task::Task,
    ui_font_selector::UiFontSelector,
    waker::{WakeType, Waker},
    widgets::{
        progress_bar::{ProgressBar, draw_progress_bar_background, value_from_x},
        simple_widgets::{
            PlainButton, get_theme_button, text_in_center_bottom_of_rect, theme_button,
        },
    },
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Id, Layout, Rect, UiBuilder, Widget as _, pos2,
    vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct State {
    search_key: String,
    left_panel_open: bool,
    initial_scaling_mode: InitialScalingMode,
    #[serde(skip)]
    last_image_info: Option<LastImageInfo>,
    #[serde(skip)]
    is_cur_image_loading: bool,
    #[serde(skip)]
    last_cur_dir: Option<usize>,
    #[serde(skip)]
    last_image_name: Option<String>,
    #[serde(skip)]
    last_window_size: egui::Vec2,
    #[serde(skip)]
    pointer_in_info_rect: bool,
    #[serde(skip)]
    last_time_pointer_in_info_rect: f64,
    #[serde(skip)]
    scroll_to_current: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            search_key: String::default(),
            left_panel_open: true,
            initial_scaling_mode: InitialScalingMode::default(),
            last_image_info: None,
            is_cur_image_loading: true,
            last_cur_dir: None,
            last_image_name: None,
            last_window_size: egui::Vec2::ZERO,
            pointer_in_info_rect: false,
            last_time_pointer_in_info_rect: 0.0,
            scroll_to_current: false,
        }
    }
}

pub struct App {
    state: State,
    waker: Waker,
    img_finder: ImgFinder,
    tex_loader: TexLoader,
    translation: ImgTranslation,
    search_task: Option<Task<Option<ImgFinder>>>,
    search_list: VecDeque<String>,
    selector: UiFontSelector,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|style| style.animation_time = 0.11);

        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };
        let waker = Waker::new(cc.egui_ctx.clone(), WakeType::WakeOnLongestDeadLine);
        let img_finder = ImgFinder::new();
        let tex_loader = TexLoader::new(&cc.egui_ctx);
        let translation = ImgTranslation::default();
        let search_task = None;
        let search_list: VecDeque<_> = std::env::args().skip(1).collect();

        let selector = if let Some(storage) = cc.storage {
            eframe::get_value(storage, UiFontSelector::KEY).unwrap_or_default()
        } else {
            UiFontSelector::default()
        };

        let mut this = Self {
            state,
            waker,
            img_finder,
            tex_loader,
            translation,
            search_task,
            search_list,
            selector,
        };

        this.rebuild_fonts(&cc.egui_ctx);
        this.selector.apply_text_style(&cc.egui_ctx);
        this
    }

    fn start_search(&mut self, path: String) {
        if self.is_searching() {
            return;
        }

        if let Ok(canonicalized_path) = std::path::Path::new(&path).canonicalize() {
            if self.img_finder.is_subpath(&canonicalized_path) {
                self.img_finder.set_path(&canonicalized_path);
                return;
            }

            let (cancel_sender, cancel_receiver) = std::sync::mpsc::channel();
            let task = Task::new(cancel_sender, move || {
                match ImgFinder::from_search(&canonicalized_path, cancel_receiver) {
                    Ok(finder) => Some(finder),
                    Err(err) => {
                        log::error!("load from path '{path}' fails: {err}");
                        None
                    }
                }
            });

            self.search_task = Some(task);
        }
    }

    fn is_searching(&self) -> bool {
        self.search_task.is_some()
    }

    fn try_get_search_result(&mut self) {
        if !self.is_searching() && !self.search_list.is_empty() {
            let path = self.search_list.pop_front().unwrap();
            self.start_search(path);
        }

        if !self.is_searching() || !self.search_task.as_ref().unwrap().is_finished() {
            return;
        }

        match self.search_task.take().unwrap().get_result() {
            Ok(Some(finder)) => self.img_finder = finder,
            Err(_) => log::error!("Search thread panicked"),
            _ => (),
        }
        self.state.last_image_info = None;
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
            if let Some(search_dir) = self.img_finder.search_dir() {
                cmd.arg(search_dir);
            }
            if let Some(cur_image_name) = self.img_finder.cur_image_name() {
                cmd.arg(cur_image_name);
            }
            cmd.spawn()?;
        });
    }

    fn ui_show_searching_modal(&mut self, ui: &mut egui::Ui) {
        if self.is_searching() {
            egui::Modal::new(egui::Id::new("Searching")).show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Searching directory...");
                    if ui.button("Cancel").clicked() {
                        self.search_task.as_ref().unwrap().cancel();
                    }
                });
            });
        }
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
                ui.horizontal(|ui| {
                    ui.visuals_mut().button_frame = false;

                    if theme_button(ui, get_theme_button(ui)).clicked() {
                        self.selector.apply_text_style(ui.ctx());
                    }

                    if self.selector.ui_and_should_rebuild_fonts(ui) {
                        self.rebuild_fonts(ui.ctx());
                    }

                    if ui
                        .button(ICON_FOLDER.to_string())
                        .on_hover_text("Load from current work directory")
                        .clicked()
                    {
                        if let Ok(dir) = std::env::current_dir() {
                            self.search_list
                                .push_back(dir.to_string_lossy().into_owned());
                        }
                    }

                    ui.selectable_value(
                        &mut self.state.initial_scaling_mode,
                        InitialScalingMode::KeepScale,
                        ICON_COFFEE.to_string(),
                    )
                    .on_hover_text("Do nothing with scale or offset, just keep it");
                    ui.selectable_value(
                        &mut self.state.initial_scaling_mode,
                        InitialScalingMode::OriginalSize,
                        ICON_SCREEN_NORMAL.to_string(),
                    )
                    .on_hover_text("Display in the original size of the image");
                    ui.selectable_value(
                        &mut self.state.initial_scaling_mode,
                        InitialScalingMode::FitToSpace,
                        ICON_SCREEN_FULL.to_string(),
                    )
                    .on_hover_text("Fit the image size with the available space size");
                });

                ui.add(
                    egui::TextEdit::singleline(&mut self.state.search_key)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search keywords"),
                );

                egui::ScrollArea::both()
                    .auto_shrink([false, true])
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

                            let res = ui.selectable_label(is_cur_dir, str).on_hover_text(dir_str);

                            if res.clicked() {
                                cur_dir = Some(dir);
                            };

                            if self.state.scroll_to_current && is_cur_dir {
                                self.state.scroll_to_current = false;
                                res.scroll_to_me(None);
                            }
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

                let title_bar_height = get_button_height(ui) + 12.0;
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
                get_body_font_id(ui),
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

                let handle = texture.get_cur_handle();
                let image_size = handle.size_vec2();
                let available_size = rect.size();

                let keep_min_scale = matches!(
                    self.state.initial_scaling_mode,
                    InitialScalingMode::KeepScale
                ) && self.translation.min_scale == self.translation.scale;

                let fit_scale = eapp_utils::calculate_fit_scale(available_size, image_size);
                self.translation.min_scale = fit_scale.min(1.0);
                self.translation.scale = self.translation.scale.max(self.translation.min_scale);

                if keep_min_scale {
                    self.translation.scale = self.translation.min_scale;
                }

                if self.translation.image_fit_space_size {
                    self.translation.image_fit_space_size = false;
                    self.translation.scale = fit_scale;
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

                let tex = egui::Image::from_texture(handle).show_loading_spinner(false);

                let image_pos = rect.center() - scaled_size * 0.5 + self.translation.image_offset;
                let image_rect = Rect::from_min_size(image_pos, scaled_size);

                self.state.last_image_info = Some(LastImageInfo {
                    average_color: texture.get_cur_average_color(),
                    rect: image_rect,
                });

                let diff = available_size - scaled_size;
                let font_size = get_body_text_size(ui);
                let corner_radius = if diff.x <= font_size && diff.y <= font_size {
                    8
                } else {
                    0
                };

                tex.corner_radius(self.adjust_corner_radius_match_left_panel(corner_radius.into()))
                    .tint(Color32::WHITE.gamma_multiply(opacity))
                    .paint_at(ui, image_rect);
            } else {
                self.state.is_cur_image_loading = true;
                if let Some(info) = self.state.last_image_info.as_ref() {
                    let fill_color = info.average_color.gamma_multiply(opacity);
                    ui.painter()
                        .rect_filled(info.rect, CornerRadius::ZERO, fill_color);
                }

                if opacity == 0.0 {
                    show_center_text("Maiden in Prayer...");
                }
            }
        } else if self.img_finder.cur_dir_set().0.is_empty()
            && self.img_finder.cur_image_set().0.is_empty()
        {
            show_center_text("Drop file or directory here");
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

        ui.scope_builder(UiBuilder::new().max_rect(btn_rect), |ui| {
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
        let current_time = ui.input(|i| i.time);

        if borderless::rect_contains_pointer(ui, sense_rect) {
            self.state.pointer_in_info_rect = true;
            self.state.last_time_pointer_in_info_rect = current_time;
            self.waker.request_repaint_after_secs(2.5);
        }

        if current_time - self.state.last_time_pointer_in_info_rect >= 2.0 {
            self.state.pointer_in_info_rect = false;
        }

        let opacity = ui
            .ctx()
            .animate_bool(Id::new("info_hover_area"), self.state.pointer_in_info_rect);

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

            page_info = format!("PAGE ({} / {})", img + 1, total_pages);

            if let Some(texture) = self.tex_loader.textures().get(img_name).unwrap() {
                let size = texture.get_cur_handle().size();
                size_info = format!(
                    "{} x {} ({:.0}%)",
                    size[0],
                    size[1],
                    self.translation.scale * 100.0
                );
            }
        }

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.visuals_mut().override_text_color = Some(ui.visuals().strong_text_color());

            ui.advance_cursor_after_rect(Rect::from_min_max(
                pos2(rect.left(), rect.top()),
                pos2(rect.right(), rect.bottom() - 92.0),
            ));

            ui.style_mut().spacing.item_spacing = vec2(0.0, 12.0);

            ui.add(egui::Label::new(name).wrap_mode(egui::TextWrapMode::Truncate));

            let response = ProgressBar::new((current_page + 1) as f64, total_pages as f64)
                .preview(|ui, hover_img| {
                    let new_page = (hover_img as usize).min(total_pages.saturating_sub(1));
                    let size = vec2(256.0, 256.0);
                    let (_, rect) = ui.allocate_space(size);

                    if let Some(img_name) = self.img_finder.image_at(new_page) {
                        if let Some(Some(texture)) = self.tex_loader.textures().get(img_name) {
                            let handle = texture.get_cur_handle();

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
            let rect_size = vec2(btn_size.x * 5.0, btn_size.y);

            let rect =
                Rect::from_center_size(pos2(rect.center().x, rect.bottom() - 22.0), rect_size);

            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal(|ui| {
                    let hover_color = ui.visuals().selection.bg_fill;

                    macro_rules! btn_clicked {
                        ($icon:expr, $hover_text:expr) => {
                            PlainButton::new(btn_size, $icon.to_string())
                                .corner_radius(CornerRadius::same(2))
                                .hover(hover_color)
                                .ui(ui)
                                .on_hover_text($hover_text)
                                .clicked()
                        };
                    }

                    if btn_clicked!(ICON_NEW_FILE, "Spawn from this") {
                        self.spawn();
                    }

                    if btn_clicked!(ICON_INSPECT, "Change Window size to fit image aspect ratio") {
                        if let Some(cur_img_name) = self.img_finder.cur_image_name() {
                            if let Some(texture) =
                                self.tex_loader.textures().get(cur_img_name).unwrap()
                            {
                                let size = texture.get_cur_handle().size_vec2();
                                eapp_utils::window_resize_by_fit_scale(ui, size);
                            }
                        }
                    }

                    if btn_clicked!(ICON_REFRESH, "Reset image translation") {
                        self.translation.scale = 1.0;
                        self.translation.image_offset = egui::Vec2::ZERO;
                        ui.ctx().request_repaint();
                    }

                    if btn_clicked!(
                        ICON_SCREEN_FULL,
                        "Fit the image size with the available space size"
                    ) {
                        self.translation.image_fit_space_size = true;
                        ui.ctx().request_repaint();
                    }

                    if btn_clicked!(ICON_GO_TO_FILE, "Open in explorer") {
                        if let Some(cur_img) = self.img_finder.cur_image_name() {
                            eapp_utils::open_in_explorer(cur_img);
                        }
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

                    self.search_list
                        .push_back(path.to_string_lossy().into_owned());
                }
            }
        });

        if let Some(current_rect) = ui.ctx().input(|i| i.viewport().inner_rect) {
            let current_size = current_rect.size();
            if self.state.last_window_size != current_size {
                self.state.last_window_size = current_size;
                self.translation
                    .fit_space_if_need(self.state.initial_scaling_mode);
            }
        }

        if self.img_finder.consume_dir_changed_flag() {
            self.state.scroll_to_current = true;
            self.tex_loader.forget_all();
            for item in self.img_finder.image_iter().take(3).rev() {
                self.tex_loader.load(item);
            }
        }

        if let Some(cur_image) = self.img_finder.cur_image_name() {
            if self.state.last_image_name.as_deref() != Some(cur_image) {
                self.state.last_image_name = Some(cur_image.to_string());
                self.translation
                    .reset_translation(self.state.initial_scaling_mode);
                self.translation
                    .fit_space_if_need(self.state.initial_scaling_mode);
            }
        } else {
            self.state.last_image_name = None;
        }
    }

    fn rebuild_fonts(&mut self, ctx: &egui::Context) {
        let fonts = self.selector.insert_font(eapp_utils::get_default_fonts());
        ctx.set_fonts(fonts);
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, UiFontSelector::KEY, &self.selector);
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.extreme_bg_color)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            self.try_get_search_result();
            self.tex_loader
                .update(ctx, self.img_finder.cur_image_name());

            self.ui_show_searching_modal(ui);

            ui.add_enabled_ui(!self.is_searching(), |ui| {
                self.ui_left_panel(ui);
                self.ui_contents(ui);
            });
        });
    }
}
