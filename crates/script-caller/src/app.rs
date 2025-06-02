use crate::script::{self, Script};
use eframe::egui::{self, Event, Key, Margin, UiBuilder, Vec2, Vec2b};

#[derive(PartialEq, Eq)]
enum RunMode {
    Config,
    Normal,
    Admin,
}

pub struct App {
    loader: script::Loader,
    cur_sel_tag: Option<usize>,
    cur_sel_script: usize,
    run_mode: RunMode,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, loader: script::Loader) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);
        Self {
            loader,
            cur_sel_tag: None,
            cur_sel_script: 0,
            run_mode: RunMode::Config,
        }
    }

    fn get_cur_script(&mut self) -> Option<&mut Script> {
        let mut iter = self.loader.script_list.iter_mut();

        if self.cur_sel_tag.is_some() {
            let cur_tag = &self.loader.tag_list[self.cur_sel_tag.unwrap()];
            iter.filter(|script| script.tag.contains(cur_tag))
                .nth(self.cur_sel_script)
        } else {
            iter.nth(self.cur_sel_script)
        }
    }

    fn get_cur_script_len(&self) -> usize {
        if self.cur_sel_tag.is_some() {
            let cur_tag = &self.loader.tag_list[self.cur_sel_tag.unwrap()];
            self.loader
                .script_list
                .iter()
                .filter(|script| script.tag.contains(cur_tag))
                .count()
        } else {
            self.loader.script_list.len()
        }
    }

    fn next_script(&mut self) {
        let len = self.get_cur_script_len().max(1);
        self.cur_sel_script = (self.cur_sel_script + 1).clamp(0, len - 1);
    }

    fn prev_script(&mut self) {
        let len = self.get_cur_script_len().max(1);
        self.cur_sel_script =
            (self.cur_sel_script as isize - 1).clamp(0, len as isize - 1) as usize;
    }

    fn select_script_by_letter(&mut self, letter: char) -> bool {
        let mut found = false;
        let iter = self.loader.script_list.iter();
        let cur_sel_script = self.cur_sel_script;

        let mut do_select = |iter: &mut dyn Iterator<Item = (usize, &Script)>| {
            for (i, script) in &mut *iter {
                if i == cur_sel_script {
                    break;
                }

                if !script.name.is_empty() {
                    let script_letter = script.name.chars().next().unwrap().to_ascii_lowercase();
                    if script_letter == letter {
                        self.cur_sel_script = i;
                        found = true;
                        break;
                    }
                }
            }
        };

        if self.cur_sel_tag.is_some() {
            let cur_tag = &self.loader.tag_list[self.cur_sel_tag.unwrap()];
            do_select(
                &mut iter
                    .filter(|script| script.tag.contains(cur_tag))
                    .enumerate()
                    .cycle()
                    .skip(cur_sel_script + 1),
            );
        } else {
            do_select(&mut iter.enumerate().cycle().skip(cur_sel_script + 1));
        }

        found
    }

    fn prev_tag(&mut self) {
        if let Some(ref mut i) = self.cur_sel_tag {
            if *i == 0 {
                self.cur_sel_tag = None;
            } else {
                *i -= 1
            }
        }
    }

    fn next_tag(&mut self) {
        match self.cur_sel_tag {
            Some(ref mut i) => {
                if *i != self.loader.tag_list.len() - 1 {
                    *i += 1
                }
            }
            None => self.cur_sel_tag = Some(0),
        }
    }

    fn auto_selectable<Value>(
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

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.65;

        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .width_range(200.0..=max_width)
            .show_inside(ui, |ui| self.ui_left_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(Margin::symmetric(8, 2)))
            .show_inside(ui, |ui| self.ui_right_panel(ui));
    }

    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        eapp_utils::borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                "script-caller",
                egui::FontId::proportional(16.0),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        let mut t_changed = false; // tag
        let mut s_changed = false; // script

        if ui.memory(|mem| mem.focused().is_none()) {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                t_changed = true;
                self.prev_tag();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                t_changed = true;
                self.next_tag();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                s_changed = true;
                self.prev_script();
            }

            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                s_changed = true;
                self.next_script();
            }

            ui.input(|i| {
                if let Some(Event::Key { key, .. }) = i.events.iter().find(|event| {
                    matches!(
                        event,
                        Event::Key { key, pressed: true, .. }
                        if *key >= Key::A && *key <= Key::Z
                    )
                }) {
                    let i = *key as u8 - Key::A as u8;
                    let letter = (b'a' + i) as char;
                    s_changed = s_changed || self.select_script_by_letter(letter)
                }
            });
        }

        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let cur_sel_tag = &mut self.cur_sel_tag;
                Self::auto_selectable(ui, cur_sel_tag, None, "ALL", t_changed);
                for (i, tag) in self.loader.tag_list.iter().enumerate() {
                    Self::auto_selectable(ui, cur_sel_tag, Some(i), tag, t_changed);
                }
            });
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                let iter = self.loader.script_list.iter();
                let show_selectable = |(i, script): (usize, &Script)| {
                    Self::auto_selectable(ui, &mut self.cur_sel_script, i, &script.name, s_changed);
                };
                if self.cur_sel_tag.is_some() {
                    let cur_tag = &self.loader.tag_list[self.cur_sel_tag.unwrap()];
                    iter.filter(|script| script.tag.contains(cur_tag))
                        .enumerate()
                        .for_each(show_selectable);
                } else {
                    iter.enumerate().for_each(show_selectable);
                }
            })
        });
    }

    fn ui_right_panel(&mut self, ui: &mut egui::Ui) {
        let script = self.get_cur_script();
        if script.is_none() {
            ui.heading("No script selected");
            return;
        }

        let script = script.unwrap();

        let res = ui.label(&script.name);
        let args_string = script.generate_args_string().trim().to_owned();
        if !args_string.is_empty() {
            res.on_hover_text(args_string);
        }

        ui.label(format!(
            "tag: {}",
            script
                .tag
                .clone()
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        ));

        ui.separator();

        let height = ui.available_height() - 48.0 - ui.spacing().item_spacing.y * 2.0;
        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink(Vec2b::new(false, true))
            .show(ui, |ui| {
                ui.set_min_height(height);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    script.show_ui(ui);
                })
            });

        ui.columns(3, |ui| {
            for (i, (run_mode, name)) in [
                (RunMode::Config, "Config"),
                (RunMode::Normal, "Normal"),
                (RunMode::Admin, "Admin"),
            ]
            .into_iter()
            .enumerate()
            {
                ui[i].vertical_centered(|ui| ui.radio_value(&mut self.run_mode, run_mode, name));
            }
        });

        ui.add_space(2.0);

        if ui
            .add_sized(ui.available_size(), egui::Button::new("Run this script"))
            .clicked()
        {
            let script_base_path = self.loader.script_path.clone();
            eapp_utils::capture_error!(error => log::error!("error when run script: {error}"), {
                if let Some(script) = self.get_cur_script() {
                    let args = script.generate_args_string();
                    let script_path = format!("{}/{}", script_base_path, script.name);
                    let require_admin = script.require_admin;
                    match self.run_mode {
                        RunMode::Config => {
                            if require_admin {
                                script::runas_admin(&script_path, &args)?
                            } else {
                                script::runas_normal(&script_path, &args)?
                            }
                        }
                        RunMode::Normal => script::runas_normal(&script_path, &args)?,
                        RunMode::Admin => script::runas_admin(&script_path, &args)?,
                    }
                }
            });
        }
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

            self.ui_title_bar(ui, title_bar_rect);

            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink2(Vec2::new(0.5, 6.0));

            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }
}
