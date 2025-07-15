use eapp_utils::{
    borderless,
    codicons::{ICON_NEW_FILE, ICON_PLAY_CIRCLE, ICON_SAVE, ICON_SETTINGS, ICON_STOP_CIRCLE},
    get_body_font_id,
    global_hotkey::{Code, GlobalHotkeyHandler, KeyMap, Modifiers},
    widgets::simple_widgets::{auto_selectable, frameless_btn, get_theme_button, theme_button},
};
use eframe::egui::{self, Color32, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};

use crate::auto_script::{
    script_editor::ScriptEditor, script_executor::ScriptExecutor, script_manager::ScriptManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
pub enum HotKeyAction {
    #[default]
    RunScript,
    CancelScript,
}

pub struct App {
    editor: ScriptEditor,
    executor: ScriptExecutor,
    manager: ScriptManager,
    search_query: String,
    cur_sel: usize,
    cur_rename: Option<usize>,
    check_error: Option<String>,
    error: Option<String>,
    handler: GlobalHotkeyHandler<HotKeyAction>,
    script_changed: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);

        let manager = match ScriptManager::load() {
            Ok(manager) => manager,
            Err(err) => {
                log::error!("Error when load `ScriptManager`: {err}");
                ScriptManager::default()
            }
        };

        let mut error = None;

        let handler = eapp_utils::capture_error!(err => {
            log::error!("Error when load `GlobalHotkeyHandler`: {err}");
            error = Some(err.to_string());
            Default::default()
        },
        {
            let mut handler = GlobalHotkeyHandler::<HotKeyAction>::default();
            handler.create_manager()?;

            let key_map = if let Some(storage) = cc.storage {
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            } else {
                KeyMap::<HotKeyAction>::default()
            };

            if  key_map.is_empty() {
                handler.register_hotkey(HotKeyAction::RunScript, Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyB)?;
                handler.register_hotkey(HotKeyAction::CancelScript, Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE)?;
            } else {
                for (hotkey, action) in key_map.values() {
                    handler.register_hotkey(*action, Some(hotkey.mods), hotkey.key)?;
                }
            }

            handler
        });

        Self {
            editor: ScriptEditor::default(),
            executor: ScriptExecutor::new(),
            manager,
            search_query: String::new(),
            cur_sel: 0,
            cur_rename: None,
            check_error: None,
            error,
            handler,
            script_changed: false,
        }
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

            if frameless_btn(ui, ICON_NEW_FILE.to_string()).clicked() {
                self.manager.new_script();
            }

            if frameless_btn(ui, ICON_SAVE.to_string()).clicked() {
                match self.manager.save() {
                    Ok(_) => self.script_changed = false,
                    Err(err) => log::error!("Error when save `ScriptManager`: {err}"),
                }
            }

            ui.menu_button(ICON_SETTINGS.to_string(), |ui| {
                self.ui_show_global_hotkeys(ui);
            });

            let executing = self.executor.is_executing();
            let text = if executing {
                ICON_STOP_CIRCLE.to_string()
            } else {
                ICON_PLAY_CIRCLE.to_string()
            };

            if frameless_btn(ui, text).clicked() {
                if executing {
                    self.executor.cancel();
                } else if let Some(script) = self.manager.scripts.get_mut(self.cur_sel) {
                    self.executor.execute_script(script.content.clone());
                }
            }

            if executing {
                ui.spinner();
            }

            let title = if self.script_changed {
                "auto-script (unsaved)"
            } else {
                "auto-script"
            };

            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                title,
                get_body_font_id(ui),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search Query")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(3.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                if self.manager.is_empty() {
                    return;
                }

                let mut script_to_delete = None;
                let query = self.search_query.to_ascii_lowercase();

                for (idx, script) in self.manager.iter().enumerate() {
                    if !self.search_query.is_empty()
                        && !script.name.to_ascii_lowercase().contains(&query)
                    {
                        continue;
                    }

                    auto_selectable(ui, &mut self.cur_sel, idx, &script.name, false).context_menu(
                        |ui| {
                            if frameless_btn(ui, "Rename").clicked() {
                                self.cur_rename = Some(idx);
                                ui.close();
                            }

                            if frameless_btn(
                                ui,
                                egui::RichText::new("Delete").color(Color32::LIGHT_RED),
                            )
                            .clicked()
                            {
                                script_to_delete = Some(idx);
                                ui.close();
                            }
                        },
                    );
                }

                if let Some(idx) = script_to_delete {
                    self.manager.remove_script(idx);
                }
            })
        });
    }

    fn ui_right_panel(&mut self, ui: &mut egui::Ui) {
        let Some(script) = self.manager.scripts.get_mut(self.cur_sel) else {
            ui.label("No script selected...");
            return;
        };
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                let layout = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
                ui.with_layout(layout, |ui| {
                    let response =
                        self.editor
                            .ui(ui, &mut script.content, self.check_error.as_ref());

                    if response.changed() {
                        self.script_changed = true;
                        match self.executor.check_script(&script.content) {
                            Ok(_) => self.check_error = None,
                            Err(err) => self.check_error = Some(err),
                        }
                    }

                    if let Some(err) = self.check_error.as_ref() {
                        response.on_hover_text_at_pointer(err);
                    }
                });
            });
    }

    fn ui_show_rename_modal(&mut self, ui: &mut egui::Ui) {
        if let Some(idx) = self.cur_rename.take() {
            egui::Modal::new(egui::Id::new("Rename")).show(ui.ctx(), |ui| {
                let Some(script) = self.manager.scripts.get_mut(idx) else {
                    return;
                };

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut script.name);
                    if !ui.button("OK").clicked() {
                        self.cur_rename = Some(idx);
                    }
                });
            });
        }
    }

    fn ui_show_error_modal(&mut self, ui: &mut egui::Ui) {
        if let Some(msg) = self.error.take() {
            egui::Modal::new(egui::Id::new("Error")).show(ui.ctx(), |ui| {
                ui.label(egui::RichText::new(&msg).color(Color32::LIGHT_RED));

                ui.vertical_centered(|ui| {
                    if ui.button("OK").clicked() {
                        self.error = None;
                    } else {
                        self.error = Some(msg);
                    }
                });
            });
        }
    }

    fn ui_show_global_hotkeys(&mut self, ui: &mut egui::Ui) {
        if !self.handler.is_ok() {
            ui.label("HotKeys unable to work");
            return;
        }

        ui.vertical_centered(|ui| ui.heading("HotKeys"));
        if let Err(err) = self.handler.ui(ui) {
            self.error = Some(err.to_string());
        }
    }

    fn poll_global_hotkey_events(&mut self, ctx: &egui::Context) {
        if !self.handler.is_ok() {
            return;
        }

        ctx.request_repaint_after_secs(1.0);
        for action in self.handler.poll_events() {
            match action {
                HotKeyAction::RunScript => {
                    if let Some(script) = self.manager.scripts.get(self.cur_sel) {
                        self.executor.execute_script(script.content.clone());
                    }
                }
                HotKeyAction::CancelScript => self.executor.cancel(),
            }
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self.handler.get_key_map());
        if let Err(err) = self.manager.save() {
            log::error!("Error when save `ScriptManager`: {err}");
        }
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            self.poll_global_hotkey_events(ui.ctx());

            if let Some(Err(e)) = self.executor.try_get_execute_result() {
                self.error = Some(e);
            }

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
            .shrink2(Vec2::new(0.5, 0.5));

            self.ui_show_rename_modal(ui);
            self.ui_show_error_modal(ui);
            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }
}
