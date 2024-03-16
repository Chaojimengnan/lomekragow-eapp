use std::process::Command;

use crate::{app::END_REACHED_LIST, mpv};
use eframe::egui::{self, Vec2b};

impl super::App {
    pub fn ui_chapters_popup(&mut self, ui: &mut egui::Ui) {
        ui.set_height(150.0);
        ui.set_width(300.0);
        egui::ScrollArea::both()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if self.player.state().chapters.is_empty() {
                    let _ = ui.selectable_label(false, "None");
                    return;
                }

                let mut clicked = None;
                for (desc, time) in &self.player.state().chapters {
                    if ui
                        .selectable_label(
                            false,
                            format!("{desc}, {}", mpv::make_time_string(*time)),
                        )
                        .clicked()
                    {
                        clicked = Some(time);
                    }
                }

                if let Some(clicked_time) = clicked {
                    self.player.seek(*clicked_time, false);
                };
            });
    }

    pub fn ui_setting_popup(&mut self, ui: &mut egui::Ui) {
        use crate::app::SettingType::*;

        ui.set_height(150.0);
        ui.set_width(350.0);
        ui.horizontal(|ui| {
            for (v, str) in [(Play, "Play"), (Color, "Color"), (Danmu, "Danmu")].into_iter() {
                ui.selectable_value(&mut self.state.setting_type, v, str);
            }
        });

        egui::ScrollArea::both()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("setting_popup_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.ui_setting_popup_contents(ui));
            });
    }

    fn ui_setting_popup_contents(&mut self, ui: &mut egui::Ui) {
        use crate::app::SettingType::*;

        macro_rules! simple_slider {
            ($value:ident, $setter:ident, $range:expr) => {
                ui.label(stringify!($value));
                let mut $value = self.player.state().$value;
                if ui.add(egui::Slider::new(&mut $value, $range)).changed() {
                    self.player.$setter($value);
                }
                ui.end_row();
            };
        }

        macro_rules! simple_combo {
            ($name:literal, $value:ident, $setter:ident, $array:expr) => {
                ui.label($name);
                let mut $value = self.player.state().$value;
                if egui::ComboBox::from_id_source(stringify!($name, "combo"))
                    .height(80.0)
                    .show_index(ui, &mut $value, $array.len(), |idx| {
                        if $array.len() == 0 {
                            return "";
                        }
                        AsRef::<str>::as_ref(&$array[idx].0)
                    })
                    .changed()
                {
                    self.player.$setter($value);
                }
                ui.end_row();
            };
        }

        match self.state.setting_type {
            Play => {
                ui.label("subtitle");
                let mut sub_visibility = self.player.state().sub_visibility;
                if eapp_utils::widgets::toggle_ui(ui, &mut sub_visibility).changed() {
                    self.player.set_sub_visibility(sub_visibility);
                }
                ui.end_row();

                ui.label("subtitle delay");
                let mut sub_delay = self.player.state().sub_delay;
                if ui
                    .add(egui::DragValue::new(&mut sub_delay).speed(1))
                    .changed()
                {
                    self.player.set_sub_delay(sub_delay);
                }
                ui.end_row();

                simple_combo!(
                    "audio track",
                    cur_audio_idx,
                    set_cur_audio_idx,
                    self.player.state().audio_tracks
                );

                simple_combo!(
                    "subtitle track",
                    cur_subtitle_idx,
                    set_cur_subtitle_idx,
                    self.player.state().subtitle_tracks
                );

                simple_combo!(
                    "video aspect",
                    video_aspect,
                    set_video_aspect,
                    mpv::player::VIDEO_ASPECT_LIST
                );

                simple_combo!(
                    "video rotate",
                    video_rotate,
                    set_video_rotate,
                    mpv::player::VIDEO_ROTATE_LIST
                );

                ui.label("end reached");
                egui::ComboBox::from_id_source("end_reached_combo")
                    .height(80.0)
                    .selected_text(END_REACHED_LIST[self.state.end_reached as usize].1)
                    .show_ui(ui, |ui| {
                        for (v, str) in END_REACHED_LIST {
                            ui.selectable_value(&mut self.state.end_reached, v, str);
                        }
                    });
                ui.end_row();

                simple_slider!(speed, set_speed, 0.25..=4.0);
            }
            Color => {
                simple_slider!(brightness, set_brightness, -100..=100);
                simple_slider!(contrast, set_contrast, -100..=100);
                simple_slider!(saturation, set_saturation, -100..=100);
                simple_slider!(gamma, set_gamma, -100..=100);
                simple_slider!(hue, set_hue, -100..=100);
                simple_slider!(sharpen, set_sharpen, -4.0..=4.0);
            }
            Danmu => {
                self.ui_setting_popup_contents_danmu(ui);
            }
        }
    }

    fn ui_setting_popup_contents_danmu(&mut self, ui: &mut egui::Ui) {
        ui.label("danmu");
        eapp_utils::widgets::toggle_ui(ui, &mut self.state.enable_danmu);
        ui.end_row();

        ui.label("danmu alpha");
        ui.add(egui::Slider::new(
            &mut self.danmu.state_mut().alpha,
            0..=255,
        ));
        ui.end_row();

        ui.label("danmu lower bound");
        ui.add(egui::Slider::new(
            &mut self.danmu.state_mut().lower_bound,
            0.25..=1.0,
        ));
        ui.end_row();

        ui.label("danmu lifetime");
        ui.add(egui::Slider::new(
            &mut self.danmu.state_mut().lifetime,
            1.0..=10.0,
        ));
        ui.end_row();

        ui.label("danmu rolling speed");
        ui.add(egui::Slider::new(
            &mut self.danmu.state_mut().rolling_speed,
            1.0..=1000.0,
        ));
        ui.end_row();

        ui.label("danmu delay");
        ui.add(
            egui::DragValue::new(&mut self.danmu.state_mut().delay)
                .speed(1.0)
                .suffix("s"),
        );
        ui.end_row();

        ui.label("danmu font size");
        let mut font_size = self.danmu.state().atlas.font_size();
        if ui
            .add(egui::Slider::new(&mut font_size, 16.0..=32.0))
            .changed()
        {
            if self.danmu.state_mut().atlas.set_font_size(font_size) {
                self.danmu.clear_emitted();
            }
        }
        ui.end_row();

        ui.label("danmu stroke size");
        let mut stroke_size = self.danmu.state().atlas.stroke_size();
        if ui
            .add(egui::Slider::new(&mut stroke_size, 0.1..=4.0))
            .changed()
        {
            if self.danmu.state_mut().atlas.set_stroke_size(stroke_size) {
                self.danmu.clear_emitted();
            }
        }
        ui.end_row();
    }

    pub fn ui_long_setting_popup(&mut self, ui: &mut egui::Ui) {
        ui.set_height(150.0);
        ui.set_width(400.0);

        use super::LongSettingType::*;
        ui.horizontal(|ui| {
            for (v, str, hover_text) in [
                (
                    MpvOptions,
                    "Mpv options",
                    "Edit mpv option (effect on the next startup)",
                ),
                (DanmuFonts, "Danmu fonts", "Edit danmu fonts"),
            ]
            .into_iter()
            {
                ui.selectable_value(&mut self.state.long_setting_type, v, str)
                    .on_hover_text(hover_text);
            }
        });

        match self.state.long_setting_type {
            MpvOptions => {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.state.options).desired_rows(8));
                });
            }
            DanmuFonts => {
                egui::ScrollArea::vertical()
                    .auto_shrink(Vec2b::new(false, false))
                    .max_height(80.0)
                    .show(ui, |ui| {
                        for (path, _) in &self.danmu.state().atlas.fonts().0 {
                            ui.label(path);
                        }
                    });

                ui.separator();

                ui.add(
                    egui::TextEdit::singleline(&mut self.state.danmu_font_path)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui.button("Add font").clicked() {
                        self.danmu
                            .state_mut()
                            .atlas
                            .add_font(&self.state.danmu_font_path);
                    }
                    if ui.button("Clear fonts").clicked() {
                        self.danmu.state_mut().atlas.clear_fonts();
                    }
                });
            }
        }
    }

    pub fn ui_playlist_popup(&mut self, ui: &mut egui::Ui) {
        ui.set_width(140.0);
        let Some((list, media)) = self.state.playlist_cur_sel.as_ref() else {
            return;
        };

        if ui.selectable_label(false, "Show in explorer").clicked() {
            // https://github.com/tauri-apps/plugins-workspace/issues/999
            #[cfg(target_os = "windows")]
            Command::new("explorer")
                .args(["/select,", &media])
                .spawn()
                .unwrap();

            #[cfg(target_os = "macos")]
            Command::new("open").args(["-R", &path]).spawn().unwrap();
        }

        ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(189, 21, 21));

        if ui.selectable_label(false, "Delete the list").clicked() {
            self.playlist.remove_list(list);
        }
    }
}
