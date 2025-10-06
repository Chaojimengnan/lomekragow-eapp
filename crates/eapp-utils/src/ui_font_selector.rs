use std::collections::BTreeMap;

use eframe::egui::{
    self, FontData, FontDefinitions, FontFamily, FontId, PopupCloseBehavior, TextStyle,
};
use serde::{Deserialize, Serialize};

use crate::{
    codicons::{
        ICON_CLEAR_ALL, ICON_FOLDER, ICON_REPLY, ICON_SYMBOL_PARAMETER, ICON_TEXT_SIZE,
        ICON_WHOLE_WORD,
    },
    widgets::simple_widgets::frameless_btn,
};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct UiFontSelector {
    pub font_path: String,
    pub text_style: BTreeMap<TextStyle, f32>,
}

impl Default for UiFontSelector {
    fn default() -> Self {
        use crate::egui::TextStyle::*;

        Self {
            font_path: String::default(),
            text_style: BTreeMap::from([
                (Heading, 18.0),
                (Body, 16.0),
                (Monospace, 16.0),
                (Button, 16.0),
                (Small, 12.0),
            ]),
        }
    }
}

impl UiFontSelector {
    pub const KEY: &str = "ui_font_selector_state";

    pub fn insert_font(&self, mut fonts: FontDefinitions) -> FontDefinitions {
        if let Ok(data) = std::fs::read(&self.font_path) {
            let name = "ui_font_selector_font".to_string();

            fonts
                .font_data
                .insert(name.clone(), FontData::from_owned(data).into());

            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, name);
        }

        fonts
    }

    pub fn apply_text_style(&self, ctx: &egui::Context) {
        ctx.style_mut(|style| {
            for (style, font_id) in style.text_styles.iter_mut() {
                if let Some(&size) = self.text_style.get(style) {
                    *font_id = FontId::proportional(size);
                }
            }
        });
    }

    pub fn ui_and_should_rebuild_fonts(&mut self, ui: &mut egui::Ui) -> bool {
        let mut should_rebuild_fonts = false;

        let response = frameless_btn(ui, ICON_WHOLE_WORD.to_string());

        egui::Popup::menu(&response)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                egui::Grid::new("ui_font_selector_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (style, size) in &mut self.text_style {
                            ui.label(style.to_string());
                            ui.add(egui::Slider::new(size, 8.0..=36.0));
                            ui.end_row();
                        }
                    });

                ui.horizontal(|ui| {
                    if frameless_btn(ui, ICON_FOLDER.to_string()).clicked()
                        && let Some(open_path) = rfd::FileDialog::new()
                            .add_filter("*", &["ttf", "otf", "ttc"])
                            .pick_file()
                    {
                        self.font_path = open_path.to_string_lossy().to_string();
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.font_path));
                });

                ui.horizontal(|ui| {
                    ui.visuals_mut().button_frame = true;
                    if ui
                        .button((ICON_CLEAR_ALL.to_string(), "Clear"))
                        .on_hover_text("Clear font path")
                        .clicked()
                    {
                        self.font_path.clear();
                    }

                    if ui
                        .button((ICON_SYMBOL_PARAMETER.to_string(), "Build"))
                        .on_hover_text("Rebuild fonts")
                        .clicked()
                    {
                        should_rebuild_fonts = true;
                    }

                    if ui
                        .button((ICON_TEXT_SIZE.to_string(), "Apply"))
                        .on_hover_text("Apply text style")
                        .clicked()
                    {
                        self.apply_text_style(ui.ctx());
                    }

                    if ui
                        .button((ICON_REPLY.to_string(), "Reset"))
                        .on_hover_text("Reset text style")
                        .clicked()
                    {
                        self.text_style = Self::default().text_style;
                    }
                });
            });

        should_rebuild_fonts
    }
}
