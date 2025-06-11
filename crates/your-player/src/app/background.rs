use crate::mpv::player::PlayState;
use eframe::egui::{self, Align2, CornerRadius, FontId, Rect, load::SizedTexture, vec2};

impl super::App {
    pub fn ui_background(&mut self, ui: &mut egui::Ui) {
        self.ui_show_cur_video_frame(ui, self.state.content_rect);
        if self.state.enable_danmu {
            self.ui_show_danmu(ui, self.state.content_rect);
        }
    }

    fn ui_show_cur_video_frame(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        let state = self.player.state();
        let playing_no_cover_audio = state.is_audio && state.media_size == (0, 0);

        let welcome_text = if playing_no_cover_audio {
            state.media_title.as_str()
        } else {
            "your player :)"
        };

        if self.player.state().play_state == PlayState::Stop || playing_no_cover_audio {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                welcome_text,
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
            let mut corner_radius = CornerRadius::same(0);
            if diff.x <= 16.0 && diff.y <= 16.0 {
                corner_radius = CornerRadius::same(8);
            }

            tex = tex.corner_radius(self.adjust_fullscreen(ui, self.adjust(corner_radius)));
            tex.paint_at(ui, Rect::from_center_size(rect.center(), size));
        }
    }

    fn ui_show_danmu(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if let Some(tex) = self.tex_register.get(*self.danmu.texture()) {
            let playback_time = self.player.state().playback_time;

            let elapsed_time = playback_time - self.state.last_playback_time;

            self.state.last_playback_time = playback_time;
            self.danmu.render(ui, tex, rect, elapsed_time);
        }
    }
}
