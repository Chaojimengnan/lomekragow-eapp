use eframe::egui::{self, Color32, Galley, text::LayoutJob};
use egui_extras::syntax_highlighting::{CodeTheme, highlight};
use std::sync::Arc;

pub fn lua_highlight(
    ui: &egui::Ui,
    code: &str,
    wrap_width: f32,
    error: Option<&String>,
) -> Arc<Galley> {
    let ctx = ui.ctx();
    let style = ui.style();
    let theme = CodeTheme::from_style(ui.style());

    let line_number = error.map(|e| extract_error_line(e));

    let mut layout_job = if let Some(error_line) = line_number {
        let mut layout_job = LayoutJob {
            text: code.to_string(),
            ..Default::default()
        };

        let mut byte_offset = 0;
        let mut line_number = 1;

        for line in split_lines_including_newline(code) {
            let mut line_job = highlight(ctx, style, &theme, line, "lua");

            for section in line_job.sections.iter_mut() {
                section.byte_range.start += byte_offset;
                section.byte_range.end += byte_offset;

                if Some(line_number) == error_line {
                    section.format.background = Color32::DARK_RED;
                }

                layout_job.sections.push(section.clone());
            }

            byte_offset += line.len();
            line_number += 1;
        }

        layout_job
    } else {
        highlight(ctx, style, &theme, code, "lua")
    };

    layout_job.wrap.max_width = wrap_width;
    ui.fonts(|f| f.layout_job(layout_job))
}

fn split_lines_including_newline(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            result.push(&text[start..=i]);
            start = i + 1;
        }
    }
    if start < text.len() {
        result.push(&text[start..]);
    }
    result
}

fn extract_error_line(error: &str) -> Option<usize> {
    let re = regex::Regex::new(r#"\[string ".*?"\]:(\d+):"#).unwrap();
    re.captures(error)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}
