use crate::mpv::{self, player::PlayState};
use eapp_utils::widgets::PlainButton;
use eframe::egui::{
    self, load::SizedTexture, pos2, vec2, Align2, Color32, FontId, Frame, Id, Rect, Rounding,
    Sense, Stroke, ViewportCommand,
};

impl super::App {
    pub fn ui_contents(&mut self, ui: &mut egui::Ui) {
        let rounding = self.adjust_fullscreen(ui, self.adjust(Rounding::same(8.0)));

        egui::CentralPanel::default()
            .frame(
                Frame::default()
                    .rounding(rounding)
                    .fill(Color32::TRANSPARENT),
            )
            .show_inside(ui, |ui| {
                let app_rect = ui.max_rect();
                self.state.content_rect = app_rect;

                let title_bar_height = 28.0;
                let title_bar_rect = {
                    let mut rect = app_rect;
                    rect.max.y = rect.min.y + title_bar_height;
                    rect
                };
                eapp_utils::borderless::title_bar_animated(ui, title_bar_rect);

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
            eapp_utils::codicons::ICON_TRIANGLE_LEFT
        } else {
            eapp_utils::codicons::ICON_TRIANGLE_RIGHT
        };

        let opacity = ui.ctx().animate_bool(
            Id::new("playlist_button_hover_area"),
            eapp_utils::borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.allocate_ui_at_rect(btn_rect, |ui| {
            ui.set_opacity(opacity);
            if ui
                .add(
                    PlainButton::new(
                        vec2(btn_rect.width(), btn_rect.height()),
                        btn_text.to_string(),
                    )
                    .rounding(Rounding::same(9.0)),
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
            eapp_utils::borderless::rect_contains_pointer(ui, sense_rect),
        );

        if opacity == 0.0 {
            return;
        }

        ui.set_opacity(opacity);

        // draw background for progress bar
        let mesh_rect = {
            let mut rect = sense_rect;
            rect.set_top(rect.bottom() - 160.0);
            rect.set_bottom(rect.bottom() - 16.0);
            rect
        };

        let mesh_top_color = Color32::TRANSPARENT;
        let mesh_bottom_color = Color32::from_black_alpha(140);

        let painter = ui.painter();

        painter.rect_filled(
            {
                let mut rect = sense_rect;
                rect.set_top(rect.bottom() - 15.5);
                rect
            },
            self.adjust_fullscreen(
                ui,
                self.adjust(Rounding {
                    se: 8.0,
                    sw: 8.0,
                    nw: 0.0,
                    ne: 0.0,
                }),
            ),
            mesh_bottom_color,
        );

        let mut mesh = eframe::egui::Mesh::default();
        mesh.colored_vertex(mesh_rect.left_top(), mesh_top_color);
        mesh.colored_vertex(mesh_rect.right_top(), mesh_top_color);
        mesh.colored_vertex(mesh_rect.left_bottom(), mesh_bottom_color);
        mesh.colored_vertex(mesh_rect.right_bottom(), mesh_bottom_color);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        painter.add(mesh);

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.visuals_mut().override_text_color = Some(Color32::WHITE);

            let progress_bar_rect = {
                let mut rect_new = rect;
                rect_new.set_bottom(rect.top() + 16.0);
                rect_new.translate(vec2(0.0, 32.0))
            };
            let response = ui.interact(progress_bar_rect, Id::new("progress_bar"), Sense::drag());
            let mut progress_bar_color = Self::INACTIVE_COL;

            let duration = self.player.state().duration;
            let mut playback_time = self.player.state().playback_time;

            let map_into_playback_time = |pointer_position| {
                egui::emath::remap_clamp(pointer_position, progress_bar_rect.x_range(), 0.0..=1.0)
                    as f64
                    * duration
            };

            if let Some(pointer) = response.hover_pos() {
                if self.player.state().play_state != PlayState::Stop {
                    let hover_playback_time = map_into_playback_time(pointer.x);
                    if let Some(tex) = self.preview.get(hover_playback_time) {
                        if let Some(tex_id) = self.tex_register.get(*tex) {
                            let size = self.preview.size();

                            egui::Area::new("preview_area".into())
                                .order(egui::Order::Tooltip)
                                .constrain(true)
                                .fixed_pos(pointer)
                                .pivot(Align2::CENTER_BOTTOM)
                                .show(ui.ctx(), |ui| {
                                    let pos = ui.cursor().min;
                                    let galley = ui.painter().layout(
                                        mpv::make_time_string(hover_playback_time),
                                        FontId::proportional(16.0),
                                        Color32::WHITE,
                                        size.0 as _,
                                    );
                                    ui.add(
                                        egui::Image::from_texture(SizedTexture::new(
                                            tex_id,
                                            vec2(size.0 as _, size.1 as _),
                                        ))
                                        .rounding(4.0),
                                    );

                                    ui.painter().rect_filled(
                                        Rect::from_min_max(pos, pos + galley.size()),
                                        Rounding {
                                            nw: 4.0,
                                            ne: 0.0,
                                            sw: 0.0,
                                            se: 0.0,
                                        },
                                        Color32::from_black_alpha(160),
                                    );
                                    ui.painter().galley(pos, galley, Color32::WHITE);
                                });
                        }
                    }
                }
            }

            if let Some(pointer) = response.interact_pointer_pos() {
                progress_bar_color = Self::ACTIVE_COL;

                playback_time = map_into_playback_time(pointer.x);
                if (self.player.state().playback_time - playback_time).abs() < 0.05 {
                    self.player.set_play_state(PlayState::Pause);
                } else {
                    self.player.seek(playback_time, false);
                }
            }

            if response.drag_stopped() {
                self.player.set_play_state(PlayState::Play);
            }

            if ui.is_rect_visible(rect) {
                let painter = ui.painter().with_clip_rect(rect);

                painter.text(
                    pos2(rect.left(), rect.top() + 1.0),
                    Align2::LEFT_TOP,
                    &self.player.state().media_title,
                    FontId::proportional(16.0),
                    ui.visuals().text_color(),
                );

                let painter = ui.painter();

                painter.line_segment(
                    [
                        progress_bar_rect.left_center(),
                        progress_bar_rect.right_center(),
                    ],
                    Stroke::new(3.0, Color32::from_rgba_premultiplied(100, 100, 100, 106)),
                );

                if duration != 0.0 {
                    let position_in_progress_bar = {
                        let mut pos = progress_bar_rect.left_center();
                        pos.x = progress_bar_rect.width() * (playback_time / duration) as f32;
                        pos.x += progress_bar_rect.left();
                        pos
                    };

                    painter.line_segment(
                        [progress_bar_rect.left_center(), position_in_progress_bar],
                        Stroke::new(3.0, progress_bar_color),
                    );

                    painter.circle_filled(position_in_progress_bar, 7.0, progress_bar_color);
                }
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
        let items_y = rect.top() + 32.0 + 12.0 + 16.0 + 8.0;
        let painter = ui.painter();
        let btn_size = 32.0;

        let playback_time = mpv::make_time_string(self.player.state().playback_time);
        let duration = mpv::make_time_string(self.player.state().duration);
        painter.text(
            pos2(rect.left(), items_y - 1.5),
            Align2::LEFT_CENTER,
            format!("{} / {}", playback_time, duration),
            FontId::proportional(16.0),
            ui.visuals().text_color(),
        );

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let new_button = |font_size, str| {
                PlainButton::new(vec2(btn_size, btn_size), str)
                    .font_size(font_size)
                    .rounding(Rounding::same(2.0))
                    .hover(Self::ACTIVE_COL)
            };

            let center_btns_rect = Rect::from_center_size(
                pos2(rect.center().x, items_y),
                vec2(btn_size * 5.0, btn_size),
            );

            use eapp_utils::codicons::*;

            ui.allocate_ui_at_rect(center_btns_rect, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(new_button(28.0, ICON_STOP_CIRCLE.to_string()))
                        .clicked()
                    {
                        self.player.set_play_state(PlayState::Stop);
                    }

                    if ui
                        .add(new_button(28.0, ICON_ARROW_CIRCLE_LEFT.to_string()))
                        .clicked()
                    {
                        if let Some(new_media) = self.playlist.prev() {
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
                    if ui.add(new_button(28.0, icon)).clicked() {
                        self.player.set_play_state(if is_pause {
                            PlayState::Play
                        } else {
                            PlayState::Pause
                        });
                    }

                    if ui
                        .add(new_button(28.0, ICON_ARROW_CIRCLE_RIGHT.to_string()))
                        .clicked()
                    {
                        if let Some(new_media) = self.playlist.next() {
                            self.set_media(&new_media);
                        }
                    }

                    let mute = self.player.state().mute;
                    let icon = if mute { ICON_MUTE } else { ICON_UNMUTE }.to_string();
                    let volume_res = ui.add(new_button(28.0, icon.clone()));
                    if volume_res.clicked() {
                        self.state.volume_popup_open = !self.state.volume_popup_open;
                    }
                    self.state.volume_popup_open = eapp_utils::widgets::popup_animated(
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
                })
            });

            let right_btns_rect = {
                let width = btn_size * 4.0;
                Rect::from_center_size(
                    pos2(rect.right() - width / 2.0, items_y - 2.0),
                    vec2(width, btn_size),
                )
            };

            ui.allocate_ui_at_rect(right_btns_rect, |ui| {
                ui.horizontal(|ui| {
                    macro_rules! simple_popup {
                        ($icon:expr, $state:ident, $popup_fn:ident) => {
                            let res = ui.add(new_button(16.0, $icon));
                            if res.clicked() {
                                self.state.$state = !self.state.$state;
                            }
                            self.state.$state = eapp_utils::widgets::popup_animated(
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
                })
            });
        });
    }
}
