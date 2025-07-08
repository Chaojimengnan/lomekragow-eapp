use eapp_utils::{
    codicons::{
        ICON_ARROW_UP, ICON_CLEAR_ALL, ICON_CLOUD_UPLOAD, ICON_COPY, ICON_EDIT, ICON_PREVIEW,
        ICON_STOP_CIRCLE,
    },
    get_body_font_id, get_body_text_size,
};
use eframe::egui::{self, Button, CollapsingHeader, Color32, TextEdit, Widget};
use egui_commonmark::CommonMarkViewer;

use crate::chat::{
    Message, Role,
    dialogue::{DialogueState, MessageWithUiData},
};

impl super::App {
    pub fn ui_right_panel(&mut self, ui: &mut egui::Ui) {
        let input_height = 142.0;
        let height = (ui.available_height() - input_height - ui.spacing().item_spacing.y).max(0.0);

        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.set_min_height(height);
                self.ui_show_dialogues(ui);
            });

        self.ui_input(ui, input_height);
    }

    fn ui_show_dialogues(&mut self, ui: &mut egui::Ui) {
        if self.manager.data.dialogues.is_empty() {
            return;
        }

        let dialogue = &mut self.manager.data.dialogues[self.manager.cur_dialogue_idx];

        let is_idle = dialogue.is_idle();
        let mut idx_to_edit = None;
        let mut clear_summary = false;

        let show_summary = self.state.show_summary && !dialogue.summary.message.content.is_empty();

        if show_summary {
            ui_show_summary(
                ui,
                &mut dialogue.summary,
                &mut clear_summary,
                dialogue.amount_of_message_summarized,
            );
        }

        let start_index = if show_summary {
            dialogue.amount_of_message_summarized
        } else {
            0
        };

        for idx in start_index..dialogue.messages.len() {
            let msg = &mut dialogue.messages[idx];
            ui_show_message(ui, msg, is_idle, idx, &mut idx_to_edit);
        }

        if dialogue.state == DialogueState::Summarizing {
            ui.horizontal(|ui| {
                ui.colored_label(ui.visuals().strong_text_color(), "Summarizing...");
                ui.spinner();
            });
        }

        if clear_summary {
            dialogue.clear_summary();
        }

        if let Some(idx) = idx_to_edit {
            let message = &mut dialogue.messages[idx].message;
            self.input = std::mem::take(&mut message.content);
            self.thinking_content = message.thinking_content.take();
            self.role = message.role;
            dialogue.back_to(idx as isize - 1);
        }
    }

    fn ui_input(&mut self, ui: &mut egui::Ui, input_height: f32) {
        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.state.trigger_request, ICON_CLOUD_UPLOAD.to_string())
                .on_hover_text("Trigger request")
                .clicked()
            {
                self.state.trigger_request = !self.state.trigger_request;
            }

            if ui
                .selectable_label(self.state.show_summary, ICON_PREVIEW.to_string())
                .on_hover_text("Show summary")
                .clicked()
            {
                self.state.show_summary = !self.state.show_summary;
            }
            ui.selectable_value(&mut self.role, Role::System, "System");
            ui.selectable_value(&mut self.role, Role::Assistant, "Assistant");
            ui.selectable_value(&mut self.role, Role::User, "User");
        });

        ui.horizontal(|ui| {
            ui.set_height((input_height - 26.0).max(0.0));
            egui::ScrollArea::vertical()
                .auto_shrink([true, false])
                .max_height(f32::INFINITY)
                .show(ui, |ui| {
                    TextEdit::multiline(&mut self.input)
                        .hint_text("Type a message")
                        .desired_rows(7)
                        .desired_width(ui.available_width() - 40.0)
                        .ui(ui);
                });

            let is_idle = self.manager.is_cur_dialogue_idle();
            let icon = if is_idle {
                ICON_ARROW_UP.to_string()
            } else {
                ICON_STOP_CIRCLE.to_string()
            };

            if ui
                .add_sized(ui.available_size(), Button::new(icon))
                .clicked()
            {
                if self.manager.data.dialogues.is_empty() {
                    self.manager.new_dialogue();
                }

                if is_idle {
                    let input = self.input.trim();
                    if !input.is_empty() {
                        let thinking_content = self.thinking_content.take();
                        self.manager.push_message(Message {
                            role: self.role,
                            content: input.to_owned(),
                            thinking_content,
                        });

                        self.input.clear();
                    }

                    if self.state.trigger_request {
                        self.manager.trigger_request();
                    }
                } else {
                    self.manager.cancel();
                }
            }
        });
    }
}

fn ui_show_message(
    ui: &mut egui::Ui,
    message_with_ui_data: &mut MessageWithUiData,
    is_idle: bool,
    idx: usize,
    idx_to_edit: &mut Option<usize>,
) {
    let max_width = ui.available_width() * 0.85;

    let MessageWithUiData { cache, message } = message_with_ui_data;
    let is_user = message.role == Role::User;
    let is_system = message.role == Role::System;

    let (bg_color, layout) = if is_user {
        (
            ui.visuals().faint_bg_color,
            egui::Layout::right_to_left(egui::Align::Min),
        )
    } else {
        (
            Color32::TRANSPARENT,
            egui::Layout::left_to_right(egui::Align::Min),
        )
    };

    if !is_user {
        if let Some(content) = message.thinking_content.as_ref() {
            CollapsingHeader::new("Thinking Content")
                .id_salt(idx)
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(content);
                });
        }
    }

    if is_system {
        ui.heading("System");
    }

    ui.with_layout(layout, |ui| {
        let width = if message.content.len() >= 200 {
            max_width
        } else {
            ui.painter()
                .layout(
                    message.content.clone(),
                    get_body_font_id(ui),
                    Color32::TRANSPARENT,
                    max_width,
                )
                .rect
                .width()
        };

        ui.set_width(width + 24.0);

        egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                CommonMarkViewer::new().show(ui, cache, &message.content);
            });
    });

    ui.with_layout(layout, |ui| {
        ui.horizontal(|ui| {
            ui.visuals_mut().button_frame = false;
            if ui.button(ICON_COPY.to_string()).clicked() {
                ui.output_mut(|o| {
                    o.commands
                        .push(egui::OutputCommand::CopyText(message.content.clone()))
                });
            }

            ui.add_enabled_ui(is_idle, |ui| {
                if ui.button(ICON_EDIT.to_string()).clicked() {
                    *idx_to_edit = Some(idx);
                }
            });
        });
    });

    ui.add_space(get_body_text_size(ui));
}

fn ui_show_summary(
    ui: &mut egui::Ui,
    summary: &mut MessageWithUiData,
    clear_summary: &mut bool,
    amount_of_message_summarized: usize,
) {
    egui::Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.heading(if amount_of_message_summarized > 1 {
                format!("Summary (1 - {amount_of_message_summarized})")
            } else {
                format!("Summary ({amount_of_message_summarized})")
            });

            if let Some(thinking_content) = &summary.message.thinking_content {
                CollapsingHeader::new("Thinking Content")
                    .id_salt("Summary Thinking Content")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(thinking_content);
                    });
            }

            CommonMarkViewer::new().show(ui, &mut summary.cache, &summary.message.content);

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(false, ICON_CLEAR_ALL.to_string())
                    .clicked()
                {
                    *clear_summary = true;
                }

                if ui.selectable_label(false, ICON_COPY.to_string()).clicked() {
                    ui.output_mut(|o| {
                        o.commands.push(egui::OutputCommand::CopyText(
                            summary.message.content.clone(),
                        ))
                    });
                }
            });
        });

    ui.add_space(get_body_text_size(ui));
}
