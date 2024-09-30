use crate::save_manager::SaveManager;
use eframe::egui::{self, collapsing_header::CollapsingState, Color32, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct App {
    manager: SaveManager,

    #[serde(skip)]
    msg: String,

    #[serde(skip)]
    cur_sel_dir: String,

    #[serde(skip)]
    input_dir: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);

        let mut this = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            App::default()
        };

        if !this.manager.main_save_dir.is_empty() {
            if let Err(err) = this.manager.load_main_save_dir() {
                this.msg = err.to_string();
            }
        }

        this
    }
}

impl App {
    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        eapp_utils::borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                "save-manager",
                egui::FontId::proportional(16.0),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect());

        let rounding = egui::Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: 8.0,
            se: 8.0,
        };

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(32.0)
            .frame(egui::Frame::default().rounding(rounding))
            .show_animated_inside(ui, !self.msg.is_empty(), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    if ui.button("Clear").clicked() {
                        self.msg.clear();
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_clip_rect(ui.max_rect());
                        ui.add_space(8.0);
                        ui.label(&self.msg);
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).rounding(rounding))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("regex");

                    let mut res = egui::TextEdit::singleline(&mut self.manager.regex_str)
                        .desired_width(f32::INFINITY)
                        .show(ui)
                        .response;

                    if let Some(err_str) = &self.manager.regex_err_str {
                        res = res.on_hover_text_at_pointer(
                            egui::RichText::new(err_str).color(Color32::DARK_RED),
                        );
                    }

                    if res.changed() {
                        self.manager.build_regex_from_str();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("save directory");
                    if ui.button("...").clicked() {
                        if let Some(dir_path) = rfd::FileDialog::new().pick_folder() {
                            self.manager.main_save_dir = dir_path.to_string_lossy().into_owned();
                        }
                    }
                    egui::TextEdit::singleline(&mut self.manager.main_save_dir)
                        .desired_width(f32::INFINITY)
                        .show(ui);
                });

                ui.columns(4, |ui| {
                    macro_rules! btn {
                        ($i:literal, $name:literal, $expr:expr) => {
                            ui[$i].vertical_centered_justified(|ui| {
                                if ui.button($name).clicked() {
                                    $expr;
                                }
                            });
                        };
                    }

                    btn!(0, "load", {
                        if let Err(err) = self.manager.load_main_save_dir() {
                            self.msg = err.to_string();
                        }
                    });

                    btn!(1, "backup", {
                        if let Err(err) = self.manager.backup(&self.cur_sel_dir) {
                            self.msg = err.to_string();
                        }
                    });

                    btn!(2, "restore", {
                        if let Err(err) = self.manager.restore(&self.cur_sel_dir) {
                            self.msg = err.to_string();
                        }
                    });

                    btn!(3, "save regex", {
                        if let Err(err) = self.manager.save_regex() {
                            self.msg = err.to_string();
                        }
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("add").clicked() && !self.input_dir.is_empty() {
                        if let Err(err) = self.manager.add(self.input_dir.clone()) {
                            self.msg = err.to_string();
                        }
                    }
                    if ui.button("remove").clicked() {
                        if let Err(err) = self.manager.remove(&self.input_dir) {
                            self.msg = err.to_string();
                        }
                    }

                    egui::TextEdit::singleline(&mut self.input_dir)
                        .desired_width(f32::INFINITY)
                        .show(ui);
                });

                ui.columns(2, |ui| {
                    egui::ScrollArea::both()
                        .id_salt("scroll_left")
                        .auto_shrink([false, false])
                        .show(&mut ui[0], |ui| {
                            for (dir, items) in self.manager.save_dirs.iter() {
                                let id = ui.make_persistent_id(dir);
                                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                    .show_header(ui, |ui| {
                                        if ui
                                            .selectable_label(self.cur_sel_dir == *dir, dir)
                                            .clicked()
                                        {
                                            self.cur_sel_dir = dir.to_string();
                                        }
                                    })
                                    .body(|ui| {
                                        let row = ui.text_style_height(&egui::TextStyle::Body);
                                        egui::ScrollArea::both()
                                            .auto_shrink([false, true])
                                            .show_rows(ui, row, items.len(), |ui, range| {
                                                for i in range {
                                                    ui.label(&items[i]);
                                                }
                                            })
                                    });
                            }
                        });

                    let row_height = ui[1].text_style_height(&egui::TextStyle::Body);
                    egui::ScrollArea::vertical()
                        .id_salt("scroll_right")
                        .auto_shrink([false, false])
                        .show_rows(
                            &mut ui[1],
                            row_height,
                            self.manager.main_save_dir_items.len(),
                            |ui, range| {
                                for i in range {
                                    let mut text = egui::RichText::new(
                                        self.manager.main_save_dir_items[i].clone(),
                                    );

                                    if let Some(reg) = self.manager.regex.as_ref() {
                                        if !reg.is_match(&self.manager.main_save_dir_items[i]) {
                                            text = text.color(egui::Color32::from_gray(60));
                                        }
                                    }

                                    ui.label(text)
                                        .on_hover_text(&self.manager.main_save_dir_items[i]);
                                }
                            },
                        );
                });
            });
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eapp_utils::borderless::window_frame(ctx, None).show(ctx, |ui| {
            eapp_utils::borderless::handle_resize(ui);

            let app_rect = ui.max_rect();

            let title_bar_height = 32.0;
            let title_bar_rect = {
                let mut rect = app_rect;
                rect.max.y = rect.min.y + title_bar_height;
                rect
            };

            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink2(Vec2::new(1.5, 1.0));

            self.ui_title_bar(ui, title_bar_rect);

            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
