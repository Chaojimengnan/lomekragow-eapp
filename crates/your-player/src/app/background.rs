use crate::mpv::player::PlayState;
use eapp_utils::{get_body_font_id, get_body_text_size};
use eframe::egui::{self, Align2, Rect, load::SizedTexture, vec2};

impl super::App {
    pub fn ui_background(&mut self, ui: &mut egui::Ui) {
        self.ui_show_cur_video_frame(ui, self.state.content_rect);
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
                get_body_font_id(ui),
                ui.visuals().text_color(),
            );
            return;
        }

        if let Some(tex_id) = self.tex_register.get(*self.player.texture()) {
            let size = self.player.state().media_size;
            let size = vec2(size.0 as _, size.1 as _);
            let mut tex = egui::Image::from_texture(SizedTexture::new(tex_id, size))
                .show_loading_spinner(false);

            let fit_scale = eapp_utils::calculate_fit_scale(rect.size(), size);
            let scaled_size = size * fit_scale;

            let font_size = get_body_text_size(ui);
            let diff = rect.size() - scaled_size;
            let corner_radius = if diff.x <= font_size && diff.y <= font_size {
                8
            } else {
                0
            };

            tex = tex.corner_radius(self.adjust_fullscreen(ui, self.adjust(corner_radius.into())));
            tex.paint_at(ui, Rect::from_center_size(rect.center(), scaled_size));
        }
    }
}
