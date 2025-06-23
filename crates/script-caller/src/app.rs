use crate::script::{self, Script};
use eapp_utils::{
    borderless,
    codicons::{ICON_FOLDER, ICON_SETTINGS_GEAR},
    widgets::simple_widgets::{get_theme_button, theme_button},
};
use eframe::egui::{self, Color32, Event, Key, UiBuilder, Vec2, Vec2b};

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
    info_json_path: Option<String>,
    search_query: String,
    load_error: Option<String>,
    cwd: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);

        let info_json_path: Option<String> = if let Some(storage) = cc.storage {
            eframe::get_value(storage, "info_json_path").unwrap_or_default()
        } else {
            None
        };

        let (loader, load_error) = match script::Loader::load(info_json_path.as_deref()) {
            Ok(loader) => (loader, None),
            Err(err) => (script::Loader::default(), Some(err.to_string())),
        };

        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned());

        Self {
            loader,
            cur_sel_tag: None,
            cur_sel_script: 0,
            run_mode: RunMode::Config,
            info_json_path,
            search_query: String::new(),
            load_error,
            cwd,
        }
    }

    fn get_cur_script(&mut self) -> Option<&mut Script> {
        let indices = self.get_filtered_indices();
        if indices.is_empty() {
            return None;
        }

        let script_index = indices.get(self.cur_sel_script)?;
        self.loader.script_list.get_mut(*script_index)
    }

    fn get_cur_script_len(&self) -> usize {
        self.get_filtered_indices().len()
    }

    fn get_filtered_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();

        for (i, script) in self.loader.script_list.iter().enumerate() {
            let tag_match = match self.cur_sel_tag {
                Some(tag_index) => {
                    let cur_tag = &self.loader.tag_list[tag_index];
                    script.tag.contains(cur_tag)
                }
                None => true,
            };

            let search_match = if self.search_query.is_empty() {
                true
            } else {
                let query = self.search_query.to_lowercase();
                script.name.to_lowercase().contains(&query)
                    || script.desc.join(" ").to_lowercase().contains(&query)
                    || script.tag.iter().any(|t| t.to_lowercase().contains(&query))
            };

            if tag_match && search_match {
                indices.push(i);
            }
        }

        indices
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
        let indices = self.get_filtered_indices();
        if indices.is_empty() {
            return false;
        }

        let search_letter = letter.to_ascii_lowercase();

        let start_index = (self.cur_sel_script + 1) % indices.len();
        let mut found = false;

        for i in 0..indices.len() {
            let index = (start_index + i) % indices.len();
            let script_index = indices[index];
            let script = &self.loader.script_list[script_index];
            if let Some(first_char) = script.name.chars().next() {
                if first_char.to_ascii_lowercase() == search_letter {
                    self.cur_sel_script = index;
                    found = true;
                    break;
                }
            }
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
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .default_width(200.0)
            .width_range(200.0..=max_width)
            .show_inside(ui, |ui| self.ui_left_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| self.ui_right_panel(ui));
    }

    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.visuals_mut().button_frame = false;

            ui.add_space(8.0);

            theme_button(ui, get_theme_button(ui));

            ui.menu_button(ICON_SETTINGS_GEAR.to_string(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("info.json");

                    if ui.button(ICON_FOLDER.to_string()).clicked() {
                        if let Some(open_path) = rfd::FileDialog::new()
                            .add_filter("JSON files", &["json"])
                            .set_directory(self.cwd.clone().unwrap_or_default())
                            .pick_file()
                        {
                            self.info_json_path = Some(open_path.to_string_lossy().to_string());
                        }
                    }

                    let mut path_str = self.info_json_path.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut path_str).changed() {
                        self.info_json_path = if path_str.is_empty() {
                            None
                        } else {
                            Some(path_str.clone())
                        };
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close_menu();
                    }

                    if ui.button("Reload").clicked() {
                        (self.loader, self.load_error) =
                            match script::Loader::load(self.info_json_path.as_deref()) {
                                Ok(loader) => (loader, None),
                                Err(err) => (script::Loader::default(), Some(err.to_string())),
                            };

                        self.cur_sel_tag = None;
                        self.cur_sel_script = 0;
                    }
                });
            });

            ui.menu_button(ICON_FOLDER.to_string(), |ui| {
                let msg = match self.cwd.as_ref() {
                    Some(cwd) => cwd,
                    None => "Cannot read current work directory",
                };
                ui.label(msg);
            });

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

        ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search Query")
                .desired_width(f32::INFINITY),
        );

        if self.load_error.is_some() {
            return;
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
                let indices = self.get_filtered_indices();

                if indices.is_empty() {
                    return;
                }

                for (display_index, &script_index) in indices.iter().enumerate() {
                    let script = &self.loader.script_list[script_index];
                    Self::auto_selectable(
                        ui,
                        &mut self.cur_sel_script,
                        display_index,
                        &script.name,
                        s_changed,
                    );
                }
            })
        });
    }

    fn ui_right_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(err) = &self.load_error {
            ui.label(err);
            return;
        }

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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "info_json_path", &self.info_json_path);
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

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
