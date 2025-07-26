mod left_panel;
mod right_panel;
mod setting;

use eapp_utils::{
    borderless,
    codicons::{ICON_LAYOUT_SIDEBAR_LEFT, ICON_SETTINGS_GEAR},
    delayed_toggle::DelayedToggle,
    get_body_font_id, get_button_height,
    ui_font_selector::UiFontSelector,
    widgets::simple_widgets::{frameless_btn, get_theme_button, theme_button},
};
use eframe::egui::{self, Color32, PopupCloseBehavior, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};

use crate::chat::{Message, Role, config::ChatConfig, dialogue_manager::DialogueManager};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    pub show_left_panel: bool,
    pub show_summarized: bool,
    pub trigger_request: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            show_left_panel: true,
            show_summarized: true,
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
    scroll_to_top: bool,
    scroll_to_bottom: bool,
    scroll_to_summary: bool,
    toggle: DelayedToggle,
    selector: UiFontSelector,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };
        let manager = DialogueManager::new(cc.egui_ctx.clone());
        let config = manager.data.config.read().unwrap().clone();

        let selector = if let Some(storage) = cc.storage {
            eframe::get_value(storage, UiFontSelector::KEY).unwrap_or_default()
        } else {
            UiFontSelector::default()
        };

        let mut this = Self {
            state,
            manager,
            input: String::new(),
            thinking_content: None,
            role: Role::User,
            config,
            status_msg: String::new(),
            edit_summary: false,
            last_summary: (
                0,
                Message {
                    role: Role::System,
                    ..Default::default()
                },
            ),
            scroll_to_top: false,
            scroll_to_bottom: false,
            scroll_to_summary: false,
            toggle: Default::default(),
            selector,
        };

        this.rebuild_fonts(&cc.egui_ctx);
        this.selector.apply_text_style(&cc.egui_ctx);
        this
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

            if theme_button(ui, get_theme_button(ui)).clicked() {
                self.selector.apply_text_style(ui.ctx());
            }

            if self.selector.ui_and_should_rebuild_fonts(ui) {
                self.rebuild_fonts(ui.ctx());
            }

            egui::Popup::menu(&frameless_btn(ui, ICON_SETTINGS_GEAR.to_string()))
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
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
            .exact_height(get_button_height(ui) + 16.0)
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
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
        self.manager.save();
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            let app_rect = ui.max_rect();

            let title_bar_height = get_button_height(ui) + 16.0;
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
