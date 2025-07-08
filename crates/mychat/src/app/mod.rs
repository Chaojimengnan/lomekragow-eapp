mod left_panel;
mod right_panel;
mod setting;

use eapp_utils::{
    borderless,
    codicons::{ICON_LAYOUT_SIDEBAR_LEFT, ICON_SETTINGS_GEAR},
    get_body_font_id,
    widgets::simple_widgets::{get_theme_button, theme_button},
};
use eframe::egui::{self, Color32, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};

use crate::chat::{Message, Role, config::ChatConfig, dialogue_manager::DialogueManager};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    pub show_left_panel: bool,
    pub show_summary: bool,
    pub trigger_request: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            show_left_panel: true,
            show_summary: true,
            trigger_request: true,
        }
    }
}

pub struct App {
    state: State,
    manager: DialogueManager,
    input: String,
    thinking_content: Option<String>,
    role: Role,
    config: ChatConfig,
    status_msg: String,
    edit_summary: bool,
    last_summary: (usize, Message),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);

        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };
        let manager = DialogueManager::new(cc.egui_ctx.clone());
        let input = String::new();
        let thinking_content = None;
        let role = Role::User;
        let config = manager.data.config.read().unwrap().clone();
        let status_msg = String::new();
        let edit_summary = false;
        let last_summary = (
            0,
            Message {
                role: Role::System,
                ..Default::default()
            },
        );

        Self {
            state,
            manager,
            input,
            thinking_content,
            role,
            config,
            status_msg,
            edit_summary,
            last_summary,
        }
    }
}

impl App {
    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.add_space(8.0);
            ui.visuals_mut().button_frame = false;

            if ui
                .selectable_label(
                    self.state.show_left_panel,
                    ICON_LAYOUT_SIDEBAR_LEFT.to_string(),
                )
                .clicked()
            {
                self.state.show_left_panel = !self.state.show_left_panel;
            }

            theme_button(ui, get_theme_button(ui));

            ui.menu_button(ICON_SETTINGS_GEAR.to_string(), |ui| {
                ui.set_max_height(ui.ctx().screen_rect().height() * 0.65);
                self.ui_setting(ui);
            });

            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                "MYCHAT:)",
                get_body_font_id(ui),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.65;

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(32.0)
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_animated_inside(ui, !self.status_msg.is_empty(), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        self.status_msg.clear();
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_clip_rect(ui.max_rect());
                        ui.label(&self.status_msg);
                    });
                });
            });

        egui::SidePanel::left("left_panel")
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .default_width(200.0)
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.state.show_left_panel, |ui| self.ui_left_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| self.ui_right_panel(ui));
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
        self.manager.save();
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

            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink2(Vec2::new(0.5, 0.5));

            self.manager.update(&mut self.status_msg);

            self.ui_title_bar(ui, title_bar_rect);
            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }
}
