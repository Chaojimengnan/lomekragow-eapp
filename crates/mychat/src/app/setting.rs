use eapp_utils::{get_body_text_size, get_button_height};
use eframe::egui::{self, Color32, TextEdit};

use crate::chat::config::ChatParam;

impl super::App {
    pub fn ui_setting(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut manager = self.manager.data.manager.write().unwrap();
            let current_index = manager.current_profile_index;
            ui.label("Current Profile:");
            let response = egui::ComboBox::from_id_salt("profile_selector")
                .selected_text(manager.cur_name().clone())
                .show_ui(ui, |ui| {
                    let mut new_index = current_index;
                    for (idx, profile) in manager.profiles.iter().enumerate() {
                        if ui
                            .selectable_label(idx == current_index, &profile.name)
                            .clicked()
                        {
                            new_index = idx;
                        }
                    }
                    new_index
                });

            if let Some(new_index) = response.inner {
                if new_index != current_index {
                    manager.current_profile_index = new_index;
                    self.config = manager.cur_config().clone();
                }
            }

            if ui.button("Add Profile").clicked() {
                let new_name = format!("Profile {}", manager.profiles.len() + 1);
                manager.add_profile(&new_name);
                manager.current_profile_index = manager.profiles.len() - 1;
                self.config = manager.cur_config().clone();
            }

            if ui.button("Remove Profile").clicked() && manager.profiles.len() > 1 {
                let remove_index = manager.current_profile_index;
                manager.remove_profile(remove_index);
                self.config = manager.cur_config().clone();
            }
        });

        ui.horizontal(|ui| {
            let mut manager = self.manager.data.manager.write().unwrap();
            ui.label("Rename Profile:");
            ui.text_edit_singleline(manager.cur_name_mut());
        });

        ui.add_space(4.0);

        let height = ui.available_height()
            - (get_button_height(ui)
                + get_body_text_size(ui)
                + ui.style().spacing.item_spacing.y * 2.0);

        egui::ScrollArea::vertical()
            .max_height(height)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Context Window:");
                    ui.add(egui::DragValue::new(&mut self.config.n_ctx).speed(1));
                });

                ui.horizontal(|ui| {
                    ui.label("Compression Threshold:");
                    ui.add(
                        egui::DragValue::new(&mut self.config.compression_threshold)
                            .speed(0.01)
                            .range(0.1..=1.0),
                    );
                });

                egui::CollapsingHeader::new("Assistant Parameters")
                    .default_open(true)
                    .show(ui, |ui| {
                        Self::ui_param(ui, &mut self.config.assistant_param);
                    });
                egui::CollapsingHeader::new("User Parameters")
                    .default_open(true)
                    .show(ui, |ui| {
                        Self::ui_param(ui, &mut self.config.user_param);
                    });
                egui::CollapsingHeader::new("Summary Parameters")
                    .default_open(true)
                    .show(ui, |ui| {
                        Self::ui_param(ui, &mut self.config.summary_param);
                    });
            });

        let config_changed = self.config != *self.manager.data.manager.read().unwrap().cur_config();

        if config_changed {
            ui.colored_label(Color32::YELLOW, "Configure Unsaved");
        } else {
            ui.colored_label(Color32::DARK_GREEN, "Configure Saved");
        };

        ui.horizontal(|ui| {
            ui.add_enabled_ui(config_changed && self.manager.is_idle(), |ui| {
                if ui.button("Save").clicked() {
                    let mut manager = self.manager.data.manager.write().unwrap();
                    *manager.cur_config_mut() = self.config.clone();
                }
            });

            if ui.button("Reset").clicked() {
                let manager = self.manager.data.manager.read().unwrap();
                self.config = manager.cur_config().clone();
            }
        });
    }

    fn ui_param(ui: &mut egui::Ui, param: &mut ChatParam) {
        ui.horizontal(|ui| {
            ui.label("API URL:");
            ui.text_edit_singleline(&mut param.api_url);
        });

        ui.horizontal(|ui| {
            ui.label("API Key:");
            ui.add(TextEdit::singleline(&mut param.api_key).password(true));
        });

        ui.horizontal(|ui| {
            ui.label("Model:");
            ui.text_edit_singleline(&mut param.model);
        });

        ui.horizontal(|ui| {
            ui.label("Max Tokens:");
            ui.add(egui::DragValue::new(&mut param.max_tokens).speed(1));
        });

        ui.horizontal(|ui| {
            ui.label("Temperature:");
            ui.add(
                egui::DragValue::new(&mut param.temperature)
                    .speed(0.01)
                    .range(0.0..=2.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Top P:");
            ui.add(
                egui::DragValue::new(&mut param.top_p)
                    .speed(0.01)
                    .range(0.0..=1.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Top K:");
            ui.add(egui::DragValue::new(&mut param.top_k).speed(1));
        });

        ui.horizontal(|ui| {
            ui.label("Min P:");
            ui.add(
                egui::DragValue::new(&mut param.min_p)
                    .speed(0.01)
                    .range(0.0..=1.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Frequency Penalty:");
            ui.add(
                egui::DragValue::new(&mut param.frequency_penalty)
                    .speed(0.01)
                    .range(-2.0..=2.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Presence Penalty:");
            ui.add(
                egui::DragValue::new(&mut param.presence_penalty)
                    .speed(0.01)
                    .range(-2.0..=2.0),
            );
        });

        ui.vertical(|ui| {
            ui.label("System Message:");
            ui.add(
                egui::TextEdit::multiline(&mut param.system_message)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
        });
    }
}
