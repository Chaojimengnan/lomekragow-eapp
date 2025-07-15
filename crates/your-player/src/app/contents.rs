use crate::mpv::{self, player::PlayState};
use eapp_utils::{
    borderless,
    codicons::{ICON_TRIANGLE_LEFT, ICON_TRIANGLE_RIGHT},
    get_body_font_id,
    widgets::{
        progress_bar::{ProgressBar, draw_progress_bar_background, value_from_x},
        simple_widgets::{PlainButton, popup_animated, text_in_center_bottom_of_rect},
    },
};
use eframe::egui::{
    self, Align2, CornerRadius, Frame, Id, Rect, UiBuilder, ViewportCommand, Widget as _,
    load::SizedTexture, pos2, vec2,
};

impl super::App {
    pub fn ui_contents(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show_inside(ui, |ui| {
                let app_rect = ui.max_rect();
                self.state.content_rect = app_rect;

                let title_bar_height = 28.0;
                let title_bar_rect = {
                    let mut rect = app_rect;
                    rect.max.y = rect.min.y + title_bar_height;
                    rect
                };
                borderless::title_bar_animated(ui, title_bar_rect);

                let size = 20.0;
                let playlist_button_rect = Rect::from_center_size(
                    pos2(app_rect.left() + size / 2.0 + 8.0, app_rect.left_center().y),
                    vec2(size, size),
                );
                let playlist_button_sense_rect = {
                    let mut rect = app_rect;
                    rect.set_right(rect.left() + size * 10.0);
                    rect.set_top(app_rect.center().y - size * 10.0);
                    rect.set_bottom(app_rect.center().y + size * 10.0);
                    rect
                };
                self.ui_playlist_button(ui, playlist_button_rect, playlist_button_sense_rect);

                let progress_bar_total_rect = {
                    let mut rect = app_rect;
                    rect.set_top(rect.bottom() - 100.0);
                    rect.shrink2(vec2(32.0, 0.0))
                };
                let progress_bar_total_sense_rect = {
                    let mut rect = app_rect;
                    rect.set_top(rect.bottom() - rect.height() * 0.5);
                    rect.translate(vec2(0.5, 0.0))
                };
                self.ui_progress_bar(ui, progress_bar_total_rect, progress_bar_total_sense_rect);
            });
    }

    fn ui_playlist_button(
        &mut self,
        ui: &mut egui::Ui,
        btn_rect: eframe::epaint::Rect,
        sense_rect: eframe::epaint::Rect,
    ) {
        let btn_text = if self.state.playlist_open {
            ICON_TRIANGLE_LEFT
        } else {
            ICON_TRIANGLE_RIGHT
        };

        let opacity = ui.ctx().animate_bool(
            Id::new("playlist_button_hover_area"),
            borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }
        ui.scope_builder(UiBuilder::new().max_rect(btn_rect), |ui| {
            ui.set_opacity(opacity);
            if ui
                .add(
                    PlainButton::new(
                        vec2(btn_rect.width(), btn_rect.height()),
                        btn_text.to_string(),
                    )
                    .corner_radius(CornerRadius::same(9)),
                )
                .clicked()
            {
                self.state.playlist_open = !self.state.playlist_open;
            }
        });
    }

    fn ui_progress_bar(
        &mut self,
        ui: &mut egui::Ui,
        rect: eframe::epaint::Rect,
        sense_rect: eframe::epaint::Rect,
    ) {
        let opacity = ui.ctx().animate_bool(
            Id::new("progress_bar_hover_area"),
            borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.set_opacity(opacity);

        let bg_rect = {
            let mut rect = sense_rect;
            rect.set_top(rect.bottom() - 190.0);
            rect
        };

        let corner_radius = self.adjust_fullscreen(
            ui,
            self.adjust(CornerRadius {
                se: 8,
                sw: 8,
                ..egui::CornerRadius::ZERO
            }),
        );

        draw_progress_bar_background(ui, bg_rect, ui.visuals().extreme_bg_color, corner_radius);

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.visuals_mut().override_text_color = Some(ui.visuals().strong_text_color());

            let duration = self.player.state().duration;
            let playback_time = self.player.state().playback_time;

            ui.add(
                egui::Label::new(&self.player.state().media_title)
                    .wrap_mode(egui::TextWrapMode::Truncate),
            );
            ui.advance_cursor_after_rect(Rect::from_min_max(
                pos2(rect.left(), rect.top()),
                pos2(rect.right(), rect.top() + 28.0),
            ));

            let response = ProgressBar::new(playback_time, duration)
                .preview(|ui, hover_time| {
                    if self.player.state().play_state != PlayState::Stop {
                        let size = self.preview.size();
                        let size = vec2(size.0 as _, size.1 as _);
                        let (_, rect) = ui.allocate_space(size);
                        if !self.player.state().is_audio {
                            if let Some(tex) = self.preview.get(hover_time) {
                                if let Some(tex_id) = self.tex_register.get(*tex) {
                                    egui::Image::from_texture(SizedTexture::new(tex_id, size))
                                        .corner_radius(4)
                                        .paint_at(ui, rect);
                                }
                            }
                        }
                        let text = mpv::make_time_string(hover_time);
                        text_in_center_bottom_of_rect(ui, text, &rect);
                    }
                })
                .ui(ui);

            let progress_bar_rect = response.rect;

            if response.dragged() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let new_playback_time =
                        value_from_x(duration, progress_bar_rect, pointer.x as _);

                    let seek_threshold = duration * 0.001;
                    if (playback_time - new_playback_time).abs() < seek_threshold.max(0.05) {
                        self.player.set_play_state(PlayState::Pause);
                    } else {
                        self.player.seek(new_playback_time, false);
                    }
                }
            }

            if response.drag_stopped() {
                self.player.set_play_state(PlayState::Play);
            }

            self.ui_progress_bar_items(ui, rect, opacity);
        });
    }

    fn ui_progress_bar_items(
        &mut self,
        ui: &mut egui::Ui,
        rect: eframe::epaint::Rect,
        parent_opacity: f32,
    ) {
        let painter = ui.painter();
        let btn_size = 32.0;

        let playback_time = mpv::make_time_string(self.player.state().playback_time);
        let duration = mpv::make_time_string(self.player.state().duration);
        painter.text(
            pos2(rect.left(), rect.bottom() - btn_size),
            Align2::LEFT_CENTER,
            format!("{playback_time} / {duration}"),
            get_body_font_id(ui),
            ui.visuals().text_color(),
        );

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let hover_color = ui.visuals().selection.bg_fill;

            let new_button = |font_size, str| {
                PlainButton::new(vec2(btn_size, btn_size), str)
                    .font_size(font_size)
                    .corner_radius(CornerRadius::same(2))
                    .hover(hover_color)
            };
            let center_btns_rect = Rect::from_center_size(
                pos2(rect.center().x, rect.bottom() - btn_size),
                vec2(btn_size * 5.0, btn_size),
            );

            use eapp_utils::codicons::*;

            ui.scope_builder(UiBuilder::new().max_rect(center_btns_rect), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(new_button(24.0, ICON_STOP_CIRCLE.to_string()))
                        .clicked()
                    {
                        self.player.set_play_state(PlayState::Stop);
                    }

                    if ui
                        .add(new_button(24.0, ICON_ARROW_CIRCLE_LEFT.to_string()))
                        .clicked()
                    {
                        if let Some(new_media) = self.playlist.prev_item() {
                            self.set_media(&new_media);
                        }
                    }

                    let is_pause = !self.player.state().play_state.is_playing();
                    let icon = if is_pause {
                        ICON_PLAY_CIRCLE
                    } else {
                        ICON_DEBUG_PAUSE
                    }
                    .to_string();
                    if ui.add(new_button(24.0, icon)).clicked() {
                        self.player.set_play_state(if is_pause {
                            PlayState::Play
                        } else {
                            PlayState::Pause
                        });
                    }

                    if ui
                        .add(new_button(24.0, ICON_ARROW_CIRCLE_RIGHT.to_string()))
                        .clicked()
                    {
                        if let Some(new_media) = self.playlist.next_item() {
                            self.set_media(&new_media);
                        }
                    }

                    let mute = self.player.state().mute;
                    let icon = if mute { ICON_MUTE } else { ICON_UNMUTE }.to_string();
                    let volume_res = ui.add(new_button(24.0, icon.clone()));
                    if volume_res.clicked() {
                        self.state.volume_popup_open = !self.state.volume_popup_open;
                    }
                    self.state.volume_popup_open = popup_animated(
                        ui,
                        self.state.volume_popup_open,
                        parent_opacity,
                        ui.make_persistent_id("volume_popup_open"),
                        &volume_res,
                        egui::AboveOrBelow::Above,
                        Align2::LEFT_BOTTOM,
                        |ui| {
                            ui.horizontal(|ui| {
                                let mut volume = self.player.state().volume;
                                if ui.add(new_button(18.0, icon)).clicked() {
                                    self.player.set_mute(!mute);
                                }
                                if ui.add(egui::Slider::new(&mut volume, 0..=130)).changed() {
                                    self.player.set_volume(volume);
                                }
                            });
                        },
                    );
                });
            });

            let right_btns_rect = {
                let width = btn_size * 5.0;
                Rect::from_center_size(
                    pos2(rect.right() - width / 2.0, rect.bottom() - btn_size),
                    vec2(width, btn_size),
                )
            };

            ui.scope_builder(UiBuilder::new().max_rect(right_btns_rect), |ui| {
                ui.horizontal(|ui| {
                    macro_rules! simple_popup {
                        ($icon:expr, $state:ident, $popup_fn:ident) => {
                            let res = ui.add(new_button(16.0, $icon));
                            if res.clicked() {
                                self.state.$state = !self.state.$state;
                            }
                            self.state.$state = popup_animated(
                                ui,
                                self.state.$state,
                                parent_opacity,
                                ui.make_persistent_id(stringify!($state)),
                                &res,
                                egui::AboveOrBelow::Above,
                                Align2::RIGHT_BOTTOM,
                                |ui| {
                                    self.$popup_fn(ui);
                                },
                            );
                        };
                    }

                    simple_popup!(
                        ICON_REFERENCES.to_string(),
                        chapters_popup_open,
                        ui_chapters_popup
                    );

                    simple_popup!(
                        ICON_SETTINGS_GEAR.to_string(),
                        setting_popup_open,
                        ui_setting_popup
                    );

                    simple_popup!(
                        ICON_NOTE.to_string(),
                        long_setting_popup_open,
                        ui_long_setting_popup
                    );

                    if ui.add(new_button(16.0, ICON_INSPECT.to_string())).clicked()
                        && self.tex_register.get(*self.player.texture()).is_some()
                    {
                        let size = self.player.state().media_size;
                        let size = vec2(size.0 as _, size.1 as _);
                        eapp_utils::window_resize_by_fit_scale(ui, size);
                    }

                    let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    let icon = if is_fullscreen {
                        ICON_SCREEN_NORMAL
                    } else {
                        ICON_SCREEN_FULL
                    }
                    .to_string();
                    if ui.add(new_button(16.0, icon)).clicked() {
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::Fullscreen(!is_fullscreen));
                    }
                });
            });
        });
    }
}
