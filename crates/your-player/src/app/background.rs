use crate::mpv::player::PlayState;
use eframe::egui::{self, load::SizedTexture, vec2, Align2, FontId, Rect, Rounding};

impl super::App {
    pub fn ui_background(&mut self, ui: &mut egui::Ui) {
        self.ui_show_cur_video_frame(ui, self.state.content_rect);
        if self.state.enable_danmu {
            self.ui_show_danmu(ui, self.state.content_rect);
        }
    }

    fn ui_show_cur_video_frame(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        if self.player.state().play_state == PlayState::Stop {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "your player :)",
                FontId::proportional(16.0),
                ui.visuals().text_color(),
            );
            return;
        }

        if let Some(tex_id) = self.tex_register.get(*self.player.texture()) {
            let size = self.player.state().media_size;
            let mut tex = egui::Image::from_texture(SizedTexture::new(
                tex_id,
                vec2(size.0 as _, size.1 as _),
            ))
            .show_loading_spinner(false)
            .maintain_aspect_ratio(true)
            .fit_to_fraction(vec2(1.0, 1.0));

            let size = tex.calc_size(rect.size(), Some(tex.size().unwrap()));
            let diff = rect.size() - size;
            let mut rounding = Rounding::same(0.0);
            if diff.x <= 16.0 && diff.y <= 16.0 {
                rounding = Rounding::same(8.0);
            }

            tex = tex.rounding(self.adjust_fullscreen(ui, self.adjust(rounding)));
            tex.paint_at(ui, Rect::from_center_size(rect.center(), size));
        }
    }

    fn ui_show_danmu(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if let Some(tex) = self.tex_register.get(*self.danmu.texture()) {
            let playback_time = self.player.state().playback_time;
            let diff = playback_time - self.state.last_playback_time;

            let elapsed_time = if self.player.state().play_state.is_playing()
                && diff.abs() <= 0.05
                && playback_time >= 0.05
            {
                self.state.last_instant.elapsed().as_secs_f64()
                    * self.player.state().speed
                    * self.state.factor
            } else {
                diff
            };

            use std::cmp::Ordering;
            match diff.partial_cmp(&0.0).unwrap() {
                Ordering::Less => self.state.factor -= self.state.factor_increment,
                Ordering::Greater => self.state.factor += self.state.factor_increment,
                Ordering::Equal => (),
            }

            self.danmu.render(ui, tex, rect, elapsed_time);

            self.state.last_playback_time += elapsed_time;
            self.state.last_instant = std::time::Instant::now();
        }
    }
}
