use crate::{img_finder::ImgFinder, tex_loader::TexLoader};
use eapp_utils::widgets::PlainButton;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontId, Frame, Id, Rect, RichText, Rounding, Vec2b,
};
use serde::{Deserialize, Serialize};
use std::ops::Bound;

pub struct App {
    state: State,
    img_finder: ImgFinder,
    tex_loader: TexLoader,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    left_panel_open: bool,
    show: Show,
}

#[derive(Deserialize, Serialize, PartialEq)]
enum Show {
    Image,
    Dir,
}

impl Default for State {
    fn default() -> Self {
        Self {
            left_panel_open: true,
            show: Show::Image,
        }
    }
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

        let mut img_finder = ImgFinder::new();
        img_finder = Self::search_from_cwd(img_finder, None);
        img_finder.consume_dir_changed_flag();

        let tex_loader = TexLoader::new(&cc.egui_ctx);

        Self {
            state,
            img_finder,
            tex_loader,
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

    fn adjust_rounding_match_left_panel(&self, rounding: Rounding) -> Rounding {
        let mut rounding = rounding;
        if self.state.left_panel_open {
            rounding.nw = 0.0;
            rounding.sw = 0.0;
        }
        rounding
    }

    fn ui_title_bar(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect) {
        eapp_utils::borderless::title_bar_behavior(ui, title_bar_rect);

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
            Id::new("cmm_btns_hover_area"),
            ui.rect_contains_pointer(interact_rect),
        );

        ui.allocate_ui_at_rect(title_bar_rect, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                eapp_utils::borderless::close_maximize_minimize(
                    ui,
                    width,
                    height,
                    Color32::from_rgb(40, 40, 40),
                    opacity,
                );
            });
        });
    }

    fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.5;

        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .frame(Frame::side_top_panel(ui.style()).rounding(Rounding {
                nw: 8.0,
                sw: 8.0,
                ne: 0.0,
                se: 0.0,
            }))
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.state.left_panel_open, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.state.show, Show::Image, "Image");
                    ui.selectable_value(&mut self.state.show, Show::Dir, "Dir");
                });

                egui::ScrollArea::vertical()
                    .auto_shrink(Vec2b::new(false, true))
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            let (mut value, list, prefix) = match self.state.show {
                                Show::Image => (
                                    self.img_finder.cur_image().cloned(),
                                    self.img_finder.cur_image_set(),
                                    self.img_finder
                                        .cur_dir()
                                        .map(|str| str.len() + 1)
                                        .unwrap_or(0),
                                ),
                                Show::Dir => (
                                    self.img_finder.cur_dir().cloned(),
                                    self.img_finder.cur_dir_set(),
                                    self.img_finder
                                        .search_dir()
                                        .map(|str| str.len() + 1)
                                        .unwrap_or(0),
                                ),
                            };

                            for item in list {
                                let item_str = if item.len() != prefix - 1 {
                                    &item[prefix..]
                                } else {
                                    "current directory"
                                };
                                ui.selectable_value(&mut value, Some(item.to_owned()), item_str);
                            }

                            if let Some(v) = value {
                                match self.state.show {
                                    Show::Image => self.img_finder.set_cur_image(&v),
                                    Show::Dir => self.img_finder.set_cur_dir(&v),
                                }
                            }
                        })
                    });
            });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        let rounding = self.adjust_rounding_match_left_panel(Rounding::same(8.0));

        egui::CentralPanel::default()
            .frame(Frame::default().rounding(rounding).fill(Color32::BLACK))
            .show_inside(ui, |ui| {
                let app_rect = ui.max_rect();

                self.process_inputs(ui);
                self.ui_show_cur_image(ui, app_rect);

                let title_bar_height = 28.0;
                let title_bar_rect = {
                    let mut rect = app_rect;
                    rect.max.y = rect.min.y + title_bar_height;
                    rect
                };
                Self::ui_title_bar(ui, title_bar_rect);

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
        if let Some(cur_image) = self.img_finder.cur_image() {
            self.tex_loader.load(cur_image);

            if let Some(texture) = self.tex_loader.textures().get(cur_image).unwrap() {
                use crate::tex_loader::Texture::*;
                let handle = match texture {
                    Static(handle) => handle,
                    Animated(animated) => &animated.frames[animated.current].0,
                };

                let mut tex = egui::Image::from_texture(handle)
                    .show_loading_spinner(false)
                    .maintain_aspect_ratio(true)
                    .fit_to_fraction(vec2(1.0, 1.0));

                let size = tex.calc_size(rect.size(), Some(handle.size_vec2()));
                let diff = rect.size() - size;
                let mut rounding = Rounding::same(0.0);
                if diff.x <= 16.0 && diff.y <= 16.0 {
                    rounding = Rounding::same(8.0);
                }

                tex = tex.rounding(self.adjust_rounding_match_left_panel(rounding));
                tex.paint_at(ui, Rect::from_center_size(rect.center(), size));
            } else {
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
            eapp_utils::codicons::ICON_TRIANGLE_LEFT
        } else {
            eapp_utils::codicons::ICON_TRIANGLE_RIGHT
        };

        let opacity = ui.ctx().animate_bool(
            Id::new("left_panel_button_hover_area"),
            ui.rect_contains_pointer(sense_rect),
        );

        ui.allocate_ui_at_rect(btn_rect, |ui| {
            if ui
                .add(
                    PlainButton::new(
                        vec2(btn_rect.width(), btn_rect.height()),
                        btn_text.to_string(),
                    )
                    .rounding(Rounding::same(9.0))
                    .opacity(opacity),
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
            ui.rect_contains_pointer(sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.painter().rect_filled(
            {
                let mut rect = sense_rect;
                rect.set_top(rect.bottom() - 130.0);
                rect
            },
            self.adjust_rounding_match_left_panel(Rounding {
                se: 8.0,
                sw: 8.0,
                nw: 0.0,
                ne: 0.0,
            }),
            Color32::from_black_alpha(180).gamma_multiply(opacity),
        );

        let text_color = Color32::WHITE.gamma_multiply(opacity);
        let opacity_text = |str| RichText::new(str).color(text_color);

        let mut name = String::from("None");
        let mut page = String::from("None");
        let mut size = String::from("? x ?");

        if let Some(img) = self.img_finder.cur_image() {
            let prefix = self.img_finder.search_dir().unwrap().len() + 1;
            name = img[prefix..].to_owned();
            let i = self
                .img_finder
                .cur_image_set()
                .range((Bound::Unbounded, Bound::Excluded(img.clone())))
                .count()
                + 1;
            let len = self.img_finder.cur_image_set().len();
            page = format!("{} / {}", i, len);

            if let Some(texture) = self.tex_loader.textures().get(img).unwrap() {
                use crate::tex_loader::Texture::*;
                let size_number = match texture {
                    Static(v) => v.size(),
                    Animated(v) => v.frames[v.current].0.size(),
                };
                size = format!("{} x {}", size_number[0], size_number[1]);
            }
        }

        ui.allocate_ui_at_rect(rect, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("info_grid")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .max_col_width(rect.width())
                    .show(ui, |ui| {
                        ui.label(opacity_text(String::from("Name")));
                        ui.label(opacity_text(name));
                        ui.end_row();
                        ui.label(opacity_text(String::from("Page")));
                        ui.label(opacity_text(page));
                        ui.end_row();
                        ui.label(opacity_text(String::from("Size")));
                        ui.label(opacity_text(size));
                        ui.end_row();
                    })
            });
        });
    }

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if ui.memory(|mem| mem.focus().is_none()) {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.img_finder.prev_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.img_finder.next_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if let Some(range) = self.img_finder.prev_image() {
                    for item in range.rev().take(3) {
                        self.tex_loader.load(item);
                    }
                }
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                if let Some(range) = self.img_finder.next_image() {
                    for item in range.take(3) {
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
                        self.img_finder.clone(),
                        Some(path.to_string_lossy().as_ref()),
                    );
                }
            }
        });

        if self.img_finder.consume_dir_changed_flag() {
            self.tex_loader.forget_all();
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
        eapp_utils::borderless::window_frame(ctx).show(ctx, |ui| {
            eapp_utils::borderless::handle_resize(ui);

            self.tex_loader.update(ctx, self.img_finder.cur_image());

            self.ui_left_panel(ui);
            self.ui_contents(ui);
        });
    }
}
