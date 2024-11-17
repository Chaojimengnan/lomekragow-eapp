use crate::{img_finder::ImgFinder, tex_loader::TexLoader};
use eapp_utils::widgets::PlainButton;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontId, Frame, Id, Rect, Rounding, UiBuilder, Vec2b,
};
use serde::{Deserialize, Serialize};
pub struct App {
    state: State,
    img_finder: ImgFinder,
    tex_loader: TexLoader,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    left_panel_open: bool,
    #[serde(skip)]
    last_cur_dir: Option<usize>,
    search_key: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            left_panel_open: true,
            last_cur_dir: None,
            search_key: String::default(),
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

        let img_path = std::env::args().nth(1);

        let mut img_finder = ImgFinder::new();
        img_finder = Self::search_from_cwd(img_finder, img_path.as_deref());
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
            .frame(Frame::side_top_panel(ui.style()).rounding(Rounding {
                nw: 8.0,
                sw: 8.0,
                ne: 0.0,
                se: 0.0,
            }))
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.state.left_panel_open, |ui| {
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
                        let prefix = self
                            .img_finder
                            .cur_dir_name()
                            .map(|str| str.len() + 1)
                            .unwrap_or(0);
                        let mut cur_dir = self.img_finder.cur_dir();
                        let mut cur_image = self.img_finder.cur_image();
                        const ACTIVE_COL: Color32 =
                            Color32::from_rgba_premultiplied(80, 138, 214, 160);

                        let dir_changed = if self.state.last_cur_dir != cur_dir {
                            self.state.last_cur_dir = cur_dir;
                            true
                        } else {
                            false
                        };

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
                            let (str, open) = if is_cur_dir {
                                (
                                    egui::RichText::new(dir_str).color(ACTIVE_COL),
                                    if dir_changed { Some(true) } else { None },
                                )
                            } else {
                                (egui::RichText::new(dir_str), Some(false))
                            };

                            if egui::CollapsingHeader::new(str)
                                .open(open)
                                .show(ui, |ui| {
                                    for (img, img_name) in
                                        self.img_finder.cur_image_set().iter().enumerate()
                                    {
                                        if ui
                                            .selectable_label(
                                                cur_image == Some(img),
                                                &img_name[prefix..],
                                            )
                                            .clicked()
                                        {
                                            cur_image = Some(img.to_owned());
                                        }
                                    }
                                })
                                .header_response
                                .on_hover_text(dir_str)
                                .clicked()
                            {
                                cur_dir = Some(dir);
                            };
                        }

                        if let Some(dir) = cur_dir {
                            self.img_finder.set_cur_dir_idx(dir);
                        }

                        if let Some(image) = cur_image {
                            self.img_finder.set_cur_image_idx(image);
                        }
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
                eapp_utils::borderless::title_bar_animated(ui, title_bar_rect);

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
        if let Some(cur_image_name) = self.img_finder.cur_image_name() {
            self.tex_loader.load(cur_image_name);

            if let Some(texture) = self.tex_loader.textures().get(cur_image_name).unwrap() {
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
            eapp_utils::borderless::rect_contains_pointer(ui, sense_rect),
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
                    .rounding(Rounding::same(9.0)),
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
            eapp_utils::borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.set_opacity(opacity);

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
            Color32::from_black_alpha(180),
        );

        let mut name = String::from("None");
        let mut page = String::from("None");
        let mut size = String::from("? x ?");

        if let Some(img) = self.img_finder.cur_image() {
            let prefix = self.img_finder.search_dir().unwrap().len() + 1;
            let img_name = self.img_finder.cur_image_name().unwrap();
            name = img_name[prefix..].to_owned();

            let len = self.img_finder.cur_image_set().0.len();
            page = format!("{} / {}", img + 1, len);

            if let Some(texture) = self.tex_loader.textures().get(img_name).unwrap() {
                use crate::tex_loader::Texture::*;
                let size_number = match texture {
                    Static(v) => v.size(),
                    Animated(v) => v.frames[v.current].0.size(),
                };
                size = format!("{} x {}", size_number[0], size_number[1]);
            }
        }

        ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("info_grid")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .max_col_width(rect.width())
                    .show(ui, |ui| {
                        ui.visuals_mut().override_text_color = Some(Color32::WHITE);
                        ui.label("Name");
                        ui.label(name);
                        ui.end_row();
                        ui.label("Page");
                        ui.label(page);
                        ui.end_row();
                        ui.label("Size");
                        ui.label(size);
                        ui.end_row();

                        ui.label("Action");
                        if ui.button("Spawn from this").clicked() {
                            self.spawn();
                        }
                        ui.end_row();
                        ui.visuals_mut().override_text_color = None;
                    })
            });
        });
    }

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if ui.memory(|mem| mem.focused().is_none()) {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.img_finder.prev_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.img_finder.next_dir();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
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

            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
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
        eapp_utils::borderless::window_frame(ctx, None).show(ctx, |ui| {
            eapp_utils::borderless::handle_resize(ui);

            self.tex_loader
                .update(ctx, self.img_finder.cur_image_name());

            self.ui_left_panel(ui);
            self.ui_contents(ui);
        });
    }
}
