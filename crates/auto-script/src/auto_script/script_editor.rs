use eframe::egui::{
    self, Color32, Galley, Id, Response, TextEdit, Ui, text::LayoutJob, text_edit::TextEditOutput,
    text_selection::text_cursor_state::byte_index_from_char_index,
};
use egui_extras::syntax_highlighting::{self, CodeTheme};
use regex::Regex;
use std::sync::Arc;

use crate::auto_script::{GUI_METHODS, SNIPPETS};

enum CompletionKind {
    Gui((usize, usize)),
    Snippet((usize, usize)),
}

struct CompletionState {
    kind: CompletionKind,
    pos: egui::Pos2,
    suggestions: Vec<&'static (&'static str, &'static str, &'static str)>,
    selected_index: usize,
}

impl CompletionState {
    fn get_replace_range(&self) -> std::ops::Range<usize> {
        match self.kind {
            CompletionKind::Gui((cursor_offset, typed_len)) => {
                cursor_offset - typed_len..cursor_offset
            }
            CompletionKind::Snippet((line_start, line_end)) => line_start..line_end,
        }
    }
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
        let changed = self.input_completion(ui, content);
        let mut output = TextEdit::multiline(content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .layouter(&mut |ui, code, wrap_width| {
                Self::highlight(ui, code.as_str(), wrap_width, check_error)
            })
            .id(Id::new("auto_script_editor"))
            .show(ui);

        if changed {
            output.response.mark_changed();
        }

        self.show_completion(ui, &mut output, content);
        output.response
    }

    pub fn is_showing_completion(&self) -> bool {
        self.completion.is_some()
    }

    fn input_completion(&mut self, ui: &mut Ui, content: &mut String) -> bool {
        let mut reset_completion = false;
        let mut content_changed = false;

        if let Some(state) = self.completion.as_mut() {
            let ctx = ui.ctx();
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                if !state.suggestions.is_empty() {
                    state.selected_index = (state.selected_index + 1) % state.suggestions.len();
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if !state.suggestions.is_empty() {
                    if state.selected_index == 0 {
                        state.selected_index = state.suggestions.len() - 1;
                    } else {
                        state.selected_index -= 1;
                    }
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
                if let Some((_, sig, _)) = state.suggestions.get(state.selected_index) {
                    content_changed = true;
                    content.replace_range(state.get_replace_range(), sig);
                    reset_completion = true;
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                reset_completion = true;
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            }
        };

        if reset_completion {
            self.completion = None;
        }

        content_changed
    }

    fn show_completion(&mut self, ui: &mut Ui, output: &mut TextEditOutput, content: &mut String) {
        let mut reset_completion = false;

        if let Some(state) = self.completion.as_ref() {
            egui::Area::new("show_completion_area".into())
                .fixed_pos(state.pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    Self::show_completion_area(ui, |ui| {
                        for (i, (_, sig, doc)) in state.suggestions.iter().enumerate() {
                            let job = Self::syntax_highlight(ui, sig, "lua");
                            let doc_job = Self::syntax_highlight(ui, doc, "md");
                            let selected = i == state.selected_index;
                            let response =
                                ui.selectable_label(selected, job).on_hover_text(doc_job);

                            if response.clicked() {
                                output.response.mark_changed();
                                content.replace_range(state.get_replace_range(), sig);
                                reset_completion = true;
                            }

                            if i % 2 == 1 {
                                let rect = response.rect;
                                let painter = ui.painter();
                                painter.rect_filled(rect, 8.0, ui.visuals().faint_bg_color);
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

        let char_index = cursor.primary.index;
        let byte_offset = byte_index_from_char_index(output.galley.text(), char_index);

        if byte_offset > content.len() {
            return;
        }

        let line_start = content[..byte_offset]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = content[byte_offset..]
            .find('\n')
            .map(|i| byte_offset + i)
            .unwrap_or(content.len());
        let current_line = &content[line_start..line_end];

        let is_gui_triggered = content[..byte_offset]
            .rfind("gui:")
            .is_some_and(|idx| idx >= line_start);

        let (suggestions, kind) = if is_gui_triggered {
            let prefix_start = content[..byte_offset].rfind("gui:").unwrap();
            let rest = &content[prefix_start + 4..];
            let typed = rest.split('\n').next().unwrap_or(rest);

            (
                GUI_METHODS
                    .iter()
                    .filter(|(name, _, _)| name.starts_with(typed) && name.len() >= typed.len())
                    .collect::<Vec<_>>(),
                CompletionKind::Gui((byte_offset, typed.len())),
            )
        } else {
            let typed = current_line.trim_start();
            (
                SNIPPETS
                    .iter()
                    .filter(|(name, _, _)| {
                        !typed.is_empty() && name.starts_with(typed) && name.len() >= typed.len()
                    })
                    .collect::<Vec<_>>(),
                CompletionKind::Snippet((line_start, line_end)),
            )
        };

        if suggestions.is_empty() {
            self.completion = None;
            return;
        }

        let rect = output.galley.pos_from_cursor(cursor.primary);
        let global_pos = output.response.rect.min + rect.left_bottom().to_vec2();
        let selected_index = self
            .completion
            .as_ref()
            .map(|c| c.selected_index)
            .unwrap_or(0)
            .min(suggestions.len().saturating_sub(1));
        self.completion = Some(CompletionState {
            kind,
            pos: global_pos,
            suggestions,
            selected_index,
        });
    }

    fn highlight(
        ui: &egui::Ui,
        code: &str,
        wrap_width: f32,
        error: Option<&String>,
    ) -> Arc<Galley> {
        let line_number = error.map(|e| Self::extract_error_line(e));

        let mut layout_job = if let Some(error_line) = line_number {
            let mut layout_job = LayoutJob {
                text: code.to_string(),
                ..Default::default()
            };

            let mut byte_offset = 0;
            let mut line_number = 1;

            for line in Self::split_lines_including_newline(code) {
                let mut line_job = Self::syntax_highlight(ui, line, "lua");

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
            Self::syntax_highlight(ui, code, "lua")
        };

        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    }

    fn syntax_highlight(ui: &egui::Ui, code: &str, lang: &str) -> LayoutJob {
        let ctx = ui.ctx();
        let style = ui.style();
        let theme = CodeTheme::from_style(ui.style());
        syntax_highlighting::highlight(ctx, style, &theme, code, lang)
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
            .multiply_with_opacity(0.6)
            .show(ui, |ui| {
                ui.set_max_height(300.0);
                ui.set_max_width(400.0);

                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::LEFT),
                            add_contents,
                        );
                    });
            });
    }
}
