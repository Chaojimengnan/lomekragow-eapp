use eframe::egui::{self, Color32};
use std::sync::Arc;

#[derive(Default)]
struct Highlighter;

impl egui::util::cache::ComputerMut<&str, egui::text::LayoutJob> for Highlighter {
    fn compute(&mut self, mut opts: &str) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();

        let font_id = egui::FontId::monospace(16.0);
        let comment_format = egui::TextFormat::simple(font_id.clone(), Color32::from_gray(120));
        let key_format =
            egui::TextFormat::simple(font_id.clone(), Color32::from_rgb(225, 120, 164));
        let value_format =
            egui::TextFormat::simple(font_id.clone(), Color32::from_rgb(80, 80, 160));
        let spliter_format =
            egui::TextFormat::simple(font_id.clone(), Color32::from_rgb(229, 227, 65));

        while !opts.is_empty() {
            if opts.starts_with('\n') {
                job.append("\n", 0.0, value_format.clone());
                opts = &opts[1..];
                continue;
            }

            let line_end = opts.find('\n').unwrap_or(opts.len());

            if opts.trim_start().starts_with('#') {
                job.append(&opts[..line_end], 0.0, comment_format.clone());
                opts = &opts[line_end..];
                continue;
            }

            let line = &opts[..line_end];

            let (has_spliter, key, value) = if let Some((key, value)) = line.split_once('=') {
                (true, key, value)
            } else {
                (false, line, "")
            };

            job.append(key, 0.0, key_format.clone());
            if has_spliter {
                job.append("=", 0.0, spliter_format.clone());
                job.append(value, 0.0, value_format.clone());
            }

            opts = &opts[line_end..];
        }

        job
    }
}

pub fn highlight(ui: &egui::Ui, opts: &str, wrap_width: f32) -> Arc<egui::Galley> {
    type HighlightCache = egui::util::cache::FrameCache<egui::text::LayoutJob, Highlighter>;

    let mut layout_job = ui.memory_mut(|mem| mem.caches.cache::<HighlightCache>().get(opts));
    layout_job.wrap.max_width = wrap_width;

    ui.fonts(|f| f.layout_job(layout_job))
}
