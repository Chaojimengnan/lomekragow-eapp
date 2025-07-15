use eapp_utils::widgets::simple_widgets::frameless_btn;
use eframe::egui::{self};

impl super::App {
    pub fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        if ui
            .add_sized([ui.available_width(), 26.0], egui::Button::new("New Chat"))
            .clicked()
        {
            self.manager.new_dialogue();
        }

        ui.add_space(4.0);

        let row_height = ui.spacing().interact_size.y;
        let total_rows = self.manager.len();

        let mut idx_to_remove = None;
        egui::ScrollArea::both()
            .auto_shrink([false, true])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                    for idx in row_range {
                        let is_current = idx == self.manager.cur_dialogue_idx;
                        let dialogue = self.manager.dialogue_mut(idx);
                        let title = if dialogue.messages.is_empty() {
                            "New Chat".to_string()
                        } else {
                            dialogue
                                .messages
                                .front()
                                .map(|m| m.message.content.chars().take(20).collect())
                                .unwrap()
                        };

                        let response = ui.selectable_label(is_current, &title).on_hover_text(title);
                        if response.clicked() {
                            self.manager.cur_dialogue_idx = idx;
                        }

                        response.context_menu(|ui| {
                            if ui
                                .add_enabled_ui(self.manager.is_dialogue_idle(idx), |ui| {
                                    frameless_btn(ui, "Delete this chat")
                                })
                                .inner
                                .clicked()
                            {
                                idx_to_remove = Some(idx);
                                ui.close();
                            }
                        });
                    }
                })
            });

        if let Some(idx) = idx_to_remove {
            self.manager.remove_dialogue(idx);
        }
    }
}
