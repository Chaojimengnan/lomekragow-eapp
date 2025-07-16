use super::PlaylistType;
use eapp_utils::widgets::simple_widgets::{get_theme_button, theme_button};
use eframe::egui::{self, Color32, CornerRadius, Frame};
use std::path::Path;

use crate::danmu::DanmuPtr;

impl super::App {
    pub fn ui_playlist(&mut self, ui: &mut egui::Ui) {
        let max_width = ui.available_width() * 0.5;

        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .frame(
                Frame::side_top_panel(ui.style()).corner_radius(self.adjust_fullscreen(
                    ui,
                    CornerRadius {
                        nw: 8,
                        sw: 8,
                        ..egui::CornerRadius::ZERO
                    },
                )),
            )
            .width_range(200.0..=max_width)
            .show_animated_inside(ui, self.state.playlist_open, |ui| {
                ui.horizontal(|ui| {
                    if theme_button(ui, get_theme_button(ui)).clicked() {
                        self.selector.apply_text_style(ui.ctx());
                    }

                    if self.selector.ui_and_should_rebuild_fonts(ui) {
                        self.rebuild_fonts(ui.ctx());
                    }

                    #[allow(clippy::single_element_loop)]
                    for (v, str) in [
                        (PlaylistType::Playlist, "Playlist"),
                        (PlaylistType::Danmu, "Danmu"),
                    ]
                    .into_iter()
                    {
                        ui.selectable_value(&mut self.state.playlist_type, v, str);
                    }
                });

                let max_width = ui.available_width();

                match self.state.playlist_type {
                    PlaylistType::Playlist => {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.playlist_key)
                                .desired_width(f32::INFINITY)
                                .hint_text("Search keywords"),
                        );
                        egui::ScrollArea::both()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                self.ui_playlist_playlist(ui, max_width);
                            });
                    }
                    PlaylistType::Danmu => {
                        self.ui_playlist_danmu(ui);
                    }
                }
            });
    }

    fn ui_playlist_playlist(&mut self, ui: &mut egui::Ui, max_width: f32) {
        let key_empty = self.state.playlist_key.is_empty();
        let key = self.state.playlist_key.to_ascii_lowercase();

        let mut current_play = self
            .playlist
            .current_play()
            .map(|(list, media)| (list.to_owned(), media.to_owned()));

        let popup_id: egui::Id = "playlist_popup_id".into();
        let mut popup_res: Option<egui::Response> = None;

        macro_rules! tuple_as_ref {
            ($value:expr) => {
                match &$value {
                    Some((list, media)) => Some((list.as_str(), media.as_str())),
                    None => None,
                }
            };
        }

        for (list_name, list) in self.playlist.inner_map() {
            let (iter, len): (Box<dyn Iterator<Item = &String>>, usize) = if key_empty {
                (Box::new(list.iter()), list.0.len())
            } else {
                let iter = list.iter().filter(|v| {
                    Path::new(v)
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_lowercase()
                        .contains(&key)
                });
                (Box::new(iter.clone()), iter.count())
            };

            let list_filename = Path::new(list_name).file_name().unwrap().to_str().unwrap();

            egui::CollapsingHeader::new(list_filename)
                .default_open(false)
                .show(ui, |ui| {
                    let row_height = ui.text_style_height(&egui::TextStyle::Body);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                    egui::ScrollArea::both()
                        .auto_shrink([false, true])
                        .show_rows(ui, row_height, len, |ui, range| {
                            for media_name in iter.skip(range.start).take(range.len()) {
                                let media_filename =
                                    Path::new(media_name).file_name().unwrap().to_str().unwrap();

                                ui.scope(|ui| {
                                    if tuple_as_ref!(current_play) == Some((list_name, media_name))
                                    {
                                        ui.visuals_mut().override_text_color =
                                            Some(ui.visuals().strong_text_color());
                                    }

                                    let res = ui
                                        .selectable_label(
                                            tuple_as_ref!(self.state.playlist_cur_sel)
                                                == Some((list_name, media_name)),
                                            media_filename,
                                        )
                                        .on_hover_text(media_name);

                                    if res.clicked() {
                                        egui::Popup::close_id(ui.ctx(), popup_id);
                                    }

                                    if res.clicked() || res.secondary_clicked() {
                                        self.state.playlist_cur_sel =
                                            Some((list_name.to_owned(), media_name.to_owned()));
                                    }

                                    if res.triple_clicked() {
                                        current_play =
                                            Some((list_name.to_owned(), media_name.to_owned()));
                                    }

                                    if res.secondary_clicked() {
                                        egui::Popup::open_id(ui.ctx(), popup_id);
                                    }

                                    if tuple_as_ref!(self.state.playlist_cur_sel)
                                        == Some((list_name, media_name))
                                    {
                                        popup_res = Some(res);
                                    }
                                });
                            }
                        });
                });
        }

        if self.playlist.current_play() != tuple_as_ref!(current_play) {
            if let Some((_, media)) = current_play.as_ref() {
                self.set_media(media);
            }
            self.playlist.set_current_play(current_play);
        }

        if let Some(res) = popup_res {
            let max_x = max_width.min(res.rect.right()).max(175.0);
            let rect = res.rect.with_max_x(max_x);
            let res = res.with_new_rect(rect);

            egui::Popup::context_menu(&res)
                .id(popup_id)
                .show(|ui| self.ui_playlist_popup(ui));
        }
    }

    fn ui_playlist_danmu(&mut self, ui: &mut egui::Ui) {
        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style) + 4.0;

        let mut res = ui.add(
            egui::TextEdit::singleline(&mut self.state.danmu_regex_str)
                .desired_width(f32::INFINITY)
                .hint_text("Block words (in regex)"),
        );

        if let Some(err_str) = &self.state.danmu_regex_err_str {
            res =
                res.on_hover_text(egui::RichText::new(err_str).color(ui.visuals().error_fg_color));
        }

        if res.changed() {
            if self.state.danmu_regex_str.is_empty() {
                self.state.danmu_regex = None;
                self.state.danmu_regex_err_str = None;
            } else {
                self.state.danmu_regex = match regex::Regex::new(&self.state.danmu_regex_str) {
                    Ok(v) => {
                        self.state.danmu_regex_err_str = None;
                        Some(v)
                    }
                    Err(err) => {
                        self.state.danmu_regex_err_str = Some(err.to_string());
                        None
                    }
                };
            }
        }

        egui::ScrollArea::both()
            .auto_shrink([false, true])
            .show_rows(ui, row_height, self.danmu.danmu().len(), |ui, row_range| {
                egui::Grid::new("playlist_danmu_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for i in row_range {
                            let danmu = &self.danmu.danmu()[i];
                            ui.label(crate::mpv::make_time_string(
                                self.danmu.danmu()[i].playback_time,
                            ));

                            let mut color =
                                Color32::from_rgb(danmu.color.0, danmu.color.1, danmu.color.2);

                            if color == Color32::WHITE {
                                color = ui.visuals().strong_text_color();
                            }

                            if let Some(regex) = &self.state.danmu_regex {
                                if regex.is_match(&danmu.text) {
                                    color = ui.visuals().weak_text_color();
                                }
                            }

                            let text = egui::RichText::new(&danmu.text).color(color);
                            if ui
                                .selectable_label(
                                    self.danmu.emitted().contains(&DanmuPtr::from(danmu)),
                                    text.clone(),
                                )
                                .on_hover_text(text)
                                .clicked()
                            {
                                self.player.seek(danmu.playback_time, false);
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}
