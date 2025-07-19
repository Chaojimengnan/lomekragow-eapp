use eapp_utils::{
    borderless,
    codicons::{
        ICON_DEBUG_START, ICON_DEBUG_STOP, ICON_LAYOUT_SIDEBAR_LEFT, ICON_NEW_FILE, ICON_SAVE,
        ICON_SETTINGS, ICON_TERMINAL,
    },
    get_body_font_id,
    global_hotkey::{Code, GlobalHotkeyHandler, KeyMap, Modifiers},
    ui_font_selector::UiFontSelector,
    widgets::simple_widgets::{
        PlainButton, auto_selectable, frameless_btn, get_theme_button, theme_button,
    },
};
use eframe::egui::{self, Align2, Color32, PopupCloseBehavior, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};

use crate::auto_script::{
    CONSOLE_SYSTEM_LOG_PREFIEX, script_editor::ScriptEditor, script_executor::ScriptExecutor,
    script_manager::ScriptManager,
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
    selector: UiFontSelector,
    show_confirm_modal: bool,
    show_console: bool,
    show_left_panel: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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

        let selector = if let Some(storage) = cc.storage {
            eframe::get_value(storage, UiFontSelector::KEY).unwrap_or_default()
        } else {
            UiFontSelector::default()
        };

        let mut this = Self {
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
            selector,
            show_confirm_modal: false,
            show_console: true,
            show_left_panel: true,
        };

        this.rebuild_fonts(&cc.egui_ctx);
        this.selector.apply_text_style(&cc.egui_ctx);
        this
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.65;
        let max_height = ui.available_height() * 0.65;

        egui::SidePanel::left("left_panel")
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .default_width(200.0)
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.show_left_panel, |ui| self.ui_left_panel(ui));

        egui::TopBottomPanel::bottom("bottom_panel")
            .default_height(300.0)
            .height_range(100.0..=max_height)
            .resizable(true)
            .frame(
                egui::Frame::side_top_panel(ui.style())
                    .inner_margin(4.0)
                    .fill(Color32::TRANSPARENT),
            )
            .show_animated_inside(ui, self.show_console, |ui| self.ui_bottom_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| self.ui_right_panel(ui));
    }

    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.visuals_mut().button_frame = false;

            ui.add_space(8.0);

            if theme_button(ui, get_theme_button(ui)).clicked() {
                self.selector.apply_text_style(ui.ctx());
            }

            if self.selector.ui_and_should_rebuild_fonts(ui) {
                self.rebuild_fonts(ui.ctx());
            }

            if frameless_btn(ui, ICON_NEW_FILE.to_string()).clicked() {
                self.manager.new_script();
            }

            if frameless_btn(ui, ICON_SAVE.to_string()).clicked() {
                match self.manager.save() {
                    Ok(_) => self.script_changed = false,
                    Err(err) => log::error!("Error when save `ScriptManager`: {err}"),
                }
            }

            egui::Popup::menu(&frameless_btn(ui, ICON_SETTINGS.to_string()))
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    self.ui_show_global_hotkeys(ui);
                });

            if ui
                .selectable_label(self.show_console, ICON_TERMINAL.to_string())
                .clicked()
            {
                self.show_console = !self.show_console;
            }

            if ui
                .selectable_label(self.show_left_panel, ICON_LAYOUT_SIDEBAR_LEFT.to_string())
                .clicked()
            {
                self.show_left_panel = !self.show_left_panel;
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
                    let is_executing = self.executor.is_executing();

                    let mut response = ui
                        .add_enabled_ui(!is_executing, |ui| {
                            self.editor
                                .ui(ui, &mut script.content, self.check_error.as_ref())
                        })
                        .inner;

                    if response.changed() {
                        self.script_changed = true;
                        match self.executor.check_script(&script.content) {
                            Ok(_) => self.check_error = None,
                            Err(err) => self.check_error = Some(err),
                        }
                    }

                    if let Some(err) = self.check_error.as_ref() {
                        if !self.editor.is_showing_completion() {
                            response = response.on_hover_text_at_pointer(err);
                        }
                    }

                    let rect = response.interact_rect;
                    let btn_size = egui::vec2(28.0, 28.0);
                    let btn_pos = rect.right_bottom() - btn_size - egui::vec2(4.0, 4.0);

                    ui.scope_builder(
                        UiBuilder::new().max_rect(egui::Rect::from_min_size(btn_pos, btn_size)),
                        |ui| {
                            let executing = self.executor.is_executing();
                            let (icon, hover_text) = if executing {
                                (ICON_DEBUG_STOP.to_string(), "Stop")
                            } else {
                                (ICON_DEBUG_START.to_string(), "Start")
                            };

                            let btn = PlainButton::new(btn_size, icon)
                                .font_size(btn_size.y)
                                .hover(Color32::TRANSPARENT);

                            if ui.add(btn).on_hover_text(hover_text).clicked() {
                                if executing {
                                    self.executor.cancel();
                                } else {
                                    self.executor.execute_script(script.content.clone());
                                }
                            }
                        },
                    );

                    let rect = {
                        let rect = response.rect;
                        let amount = rect.size() * 0.2;
                        rect.shrink2(amount)
                    };

                    if is_executing {
                        Self::draw_running_hint(ui, rect);
                    }
                });
            });
    }

    fn ui_bottom_panel(&mut self, ui: &mut egui::Ui) {
        fn get_bg_color(ui: &egui::Ui) -> Color32 {
            let visuals = ui.visuals();

            let base = if visuals.dark_mode {
                visuals.extreme_bg_color
            } else {
                visuals.window_fill()
            };

            if visuals.dark_mode {
                base.linear_multiply(0.95)
            } else {
                base.linear_multiply(1.05)
            }
        }

        egui::Frame::new()
            .corner_radius(8.0)
            .inner_margin(8.0)
            .fill(get_bg_color(ui))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        for log in self.executor.get_console_logs() {
                            let color = if log.starts_with(CONSOLE_SYSTEM_LOG_PREFIEX) {
                                ui.visuals().warn_fg_color
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(egui::RichText::new(log).color(color));
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

    fn ui_show_confirm_modal(&mut self, ui: &mut egui::Ui) {
        if self.show_confirm_modal {
            egui::Modal::new(egui::Id::new("confirm_close")).show(ui.ctx(), |ui| {
                ui.label("There are unsaved changes, are you sure you want to exit?");

                ui.horizontal(|ui| {
                    if ui.button("yes").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    if ui.button("no").clicked() {
                        self.show_confirm_modal = false;
                    }
                });
            });
        }
    }

    fn draw_running_hint(ui: &mut egui::Ui, rect: egui::Rect) {
        egui::Spinner::new().paint_at(ui, rect);

        let time = ui.input(|i| i.time);
        let dot_count = (time as usize) % 3;
        let text = format!("Running{}", ".".repeat(dot_count + 2));

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            get_body_font_id(ui),
            ui.visuals().strong_text_color(),
        );
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
                        if !self.executor.is_executing() {
                            self.executor.execute_script(script.content.clone());
                        }
                    }
                }
                HotKeyAction::CancelScript => self.executor.cancel(),
            }
        }
    }

    fn process_close_request(&mut self, ui: &mut egui::Ui) {
        if ui.ctx().input(|i| i.viewport().close_requested())
            && self.script_changed
            && !self.show_confirm_modal
        {
            self.show_confirm_modal = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }

    fn rebuild_fonts(&mut self, ctx: &egui::Context) {
        let fonts = self.selector.insert_font(eapp_utils::get_default_fonts());
        ctx.set_fonts(fonts);
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, UiFontSelector::KEY, &self.selector);
        eframe::set_value(storage, eframe::APP_KEY, self.handler.get_key_map());
        if let Err(err) = self.manager.save() {
            log::error!("Error when save `ScriptManager`: {err}");
        }
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            self.poll_global_hotkey_events(ui.ctx());
            self.executor.update();

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

            self.process_close_request(ui);
            self.ui_show_confirm_modal(ui);

            self.ui_show_rename_modal(ui);
            self.ui_show_error_modal(ui);
            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }
}
