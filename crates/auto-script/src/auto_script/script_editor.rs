use eapp_utils::widgets::simple_widgets::frameless_btn;
use eframe::egui::{
    self, Color32, Galley, Id, Response, TextEdit, Ui, text::LayoutJob, text_edit::TextEditOutput,
    text_selection::text_cursor_state::byte_index_from_char_index,
};
use egui_extras::syntax_highlighting::{self, CodeTheme};
use regex::Regex;
use std::sync::Arc;

use crate::auto_script::GUI_METHODS;

struct CompletionState {
    byte_offset: usize,
    pos: egui::Pos2,
    suggestions: Vec<&'static (&'static str, &'static str, &'static str)>,
}

#[derive(Default)]
pub struct ScriptEditor {
    completion: Option<CompletionState>,
}

impl ScriptEditor {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        content: &mut String,
        check_error: Option<&String>,
    ) -> Response {
        let mut output = TextEdit::multiline(content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .layouter(&mut |ui, code, wrap_width| {
                Self::highlight(ui, code, wrap_width, check_error)
            })
            .id(Id::new("auto_script_editor"))
            .show(ui);

        self.show_completion(ui, &mut output, content);
        output.response
    }

    fn show_completion(&mut self, ui: &mut Ui, output: &mut TextEditOutput, content: &mut String) {
        let mut reset_completion = false;

        if let Some(state) = self.completion.as_ref() {
            egui::Area::new("show_completion_area".into())
                .fixed_pos(state.pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Self::show_completion_area(ui, |ui| {
                        for (_, sig, doc) in &state.suggestions {
                            if frameless_btn(ui, *sig).on_hover_text(*doc).clicked() {
                                output.response.mark_changed();
                                content.insert_str(state.byte_offset, sig);
                                reset_completion = true;
                            }
                        }
                    });
                });
        }

        if reset_completion {
            self.completion = None;
        }

        let Some(cursor) = output.cursor_range else {
            return;
        };

        let char_index = cursor.primary.ccursor.index;
        let byte_offset = byte_index_from_char_index(output.galley.text(), char_index);

        if byte_offset > content.len() {
            return;
        }

        let Some(prefix_start) = content[..byte_offset].rfind("gui:") else {
            self.completion = None;
            return;
        };
        let rest = &content[prefix_start + 4..];
        let typed = rest.split('\n').next().unwrap_or(rest);

        let suggestions: Vec<_> = GUI_METHODS
            .iter()
            .filter(|(name, _, _)| name.starts_with(typed) && name.len() > typed.len())
            .collect();

        if suggestions.is_empty() {
            self.completion = None;
            return;
        }

        let rect = output.galley.pos_from_cursor(&cursor.primary);
        let global_pos = output.response.rect.min + rect.left_bottom().to_vec2();

        self.completion = Some(CompletionState {
            byte_offset,
            pos: global_pos,
            suggestions: suggestions.clone(),
        });
    }

    fn highlight(
        ui: &egui::Ui,
        code: &str,
        wrap_width: f32,
        error: Option<&String>,
    ) -> Arc<Galley> {
        let ctx = ui.ctx();
        let style = ui.style();
        let theme = CodeTheme::from_style(ui.style());

        let line_number = error.map(|e| Self::extract_error_line(e));

        let mut layout_job = if let Some(error_line) = line_number {
            let mut layout_job = LayoutJob {
                text: code.to_string(),
                ..Default::default()
            };

            let mut byte_offset = 0;
            let mut line_number = 1;

            for line in Self::split_lines_including_newline(code) {
                let mut line_job = syntax_highlighting::highlight(ctx, style, &theme, line, "lua");

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
            syntax_highlighting::highlight(ctx, style, &theme, code, "lua")
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
        let re = Regex::new(r#"\[string ".*?"\]:(\d+):"#).unwrap();
        re.captures(error)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<usize>().ok())
    }

    fn show_completion_area<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
        egui::Frame::popup(ui.style())
            .multiply_with_opacity(0.5)
            .show(ui, |ui| {
                ui.set_max_height(300.0);
                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::LEFT),
                            add_contents,
                        )
                    });
            });
    }
}
