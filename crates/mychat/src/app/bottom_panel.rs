use eapp_utils::{
    codicons::{
        ICON_ARROW_CIRCLE_DOWN, ICON_ARROW_CIRCLE_UP, ICON_ARROW_UP, ICON_CLOUD_UPLOAD,
        ICON_OPEN_PREVIEW, ICON_PREVIEW, ICON_STOP_CIRCLE,
    },
    widgets::simple_widgets::frameless_btn,
};
use eframe::egui::{self, Button, TextEdit, Widget};

use crate::chat::{Message, Role};

impl super::App {
    pub fn ui_bottom_panel(&mut self, ui: &mut egui::Ui) {
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
                .selectable_label(self.state.show_summarized, ICON_PREVIEW.to_string())
                .on_hover_text("Show summarized messages")
                .clicked()
            {
                self.state.show_summarized = !self.state.show_summarized;
            }
            if frameless_btn(ui, ICON_ARROW_CIRCLE_UP.to_string())
                .on_hover_text("Scroll to top")
                .clicked()
            {
                self.scroll_to_top = true;
            }
            if frameless_btn(ui, ICON_ARROW_CIRCLE_DOWN.to_string())
                .on_hover_text("Scroll to bottom")
                .clicked()
            {
                self.scroll_to_bottom = true;
            }
            if frameless_btn(ui, ICON_OPEN_PREVIEW.to_string())
                .on_hover_text("Scroll to summary")
                .clicked()
            {
                self.scroll_to_summary = true;
            }
            ui.selectable_value(&mut self.role, Role::System, "System");
            ui.selectable_value(&mut self.role, Role::Assistant, "Assistant");
            ui.selectable_value(&mut self.role, Role::User, "User");
        });

        let height = ui.available_height();
        ui.horizontal(|ui| {
            let button_size = egui::Vec2::new(36.0, height);
            let scroll_width =
                (ui.available_width() - button_size.x - ui.spacing().item_spacing.x).max(0.0);

            ui.allocate_ui(egui::Vec2::new(scroll_width, height), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            TextEdit::multiline(&mut self.input)
                                .hint_text("Type a message")
                                .desired_width(scroll_width - 8.0)
                                .ui(ui);
                        });
                    });
            });

            let is_idle = self.manager.is_cur_dialogue_idle();
            let icon = if is_idle {
                ICON_ARROW_UP.to_string()
            } else {
                ICON_STOP_CIRCLE.to_string()
            };
            let icon = egui::RichText::new(icon).size(24.0);

            if ui.add_sized(button_size, Button::new(icon)).clicked() {
                if self.manager.is_empty() {
                    self.manager.new_dialogue();
                }

                match (is_idle, self.edit_summary) {
                    (true, true) => {
                        let input = self.input.trim();
                        self.manager.cur_dialogue_mut().summary.message.content = input.to_owned();
                        self.edit_summary = false;
                        self.input.clear();
                    }
                    (true, false) => {
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
                            self.last_summary.0 =
                                self.manager.cur_dialogue().amount_of_message_summarized;
                            self.last_summary.1 =
                                self.manager.cur_dialogue().summary.message.clone();
                            self.manager.trigger_request();
                        }
                    }
                    (false, _) => self.manager.cancel(),
                }
            }
        });
    }
}
