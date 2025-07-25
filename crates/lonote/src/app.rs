use crate::codec;
use chardetng::EncodingDetector;
use eapp_utils::{
    borderless,
    codicons::{ICON_TRIANGLE_DOWN, ICON_TRIANGLE_UP},
    get_body_font_id, get_button_height,
    ui_font_selector::UiFontSelector,
    widgets::simple_widgets::{get_theme_button, theme_button},
};
use eframe::egui::{
    self, Color32, Margin, Rect, UiBuilder, Vec2,
    text::{CCursor, CCursorRange},
    text_edit::TextEditOutput,
    text_selection::text_cursor_state::{byte_index_from_char_index, cursor_rect},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

type DialogCb = Option<(String, Box<dyn FnOnce(bool) -> Result<()>>)>;

pub struct App {
    note: Rc<RefCell<Note>>,
    dialog_cb: DialogCb,
    show_search_box: bool,
    case_sense: bool,
    search_words: String,
    search_down: Option<bool>,
    selector: UiFontSelector,
}

struct Note {
    pub codec_idx: usize,
    pub contents: String,
    pub state_msg: String,
    pub title: String,
    pub modified: bool,
    pub cur_file: Option<File>,
    pub allow_to_close: bool,
}

impl Note {
    pub fn update_title(&mut self) {
        let modified = self.modified;
        let modified = if modified { "* " } else { "" };
        let name = match &self.cur_file {
            Some(f) => f.path.file_name().unwrap().to_string_lossy().to_string(),
            None => "(Untitled)".to_string(),
        };
        self.title = format!("{modified}{name} - lonote");
    }

    pub fn read_from_file<P>(path: P, codec_idx: Option<usize>) -> Result<(String, usize)>
    where
        P: AsRef<std::path::Path>,
    {
        let data = std::fs::read(path.as_ref())?;

        let encoding = match codec_idx {
            Some(idx) => codec::supported_encodings()[idx],
            None => {
                let mut detector = EncodingDetector::new();
                detector.feed(&data, true);
                detector.guess(None, true)
            }
        };

        let codec_list = codec::supported_encodings();
        let codec_idx = codec_list.iter().position(|&e| e == encoding).unwrap_or(0); // Default to UTF-8

        let contents = if codec_idx == 0 {
            String::from_utf8(data).map_err(|e| e.utf8_error())?
        } else {
            codec::decode_to_utf8(encoding, &data)
        };

        Ok((contents, codec_idx))
    }

    pub fn write_to_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<std::path::Path>,
    {
        if self.codec_idx == 0 {
            return Ok(std::fs::write(path, &self.contents)?);
        }

        let encoding = codec::supported_encodings()[self.codec_idx];
        Ok(std::fs::write(
            path,
            codec::encode_from_utf8(encoding, &self.contents),
        )?)
    }

    pub fn get_modified_time<P>(path: P) -> Result<Duration>
    where
        P: AsRef<std::path::Path>,
    {
        Ok(std::fs::metadata(path)?
            .modified()?
            .duration_since(UNIX_EPOCH)?)
    }

    pub fn get_path(&self) -> Option<&Path> {
        self.cur_file.as_ref().map(|file| file.path.as_path())
    }
}

impl Default for Note {
    fn default() -> Self {
        Self {
            codec_idx: 0,
            contents: Default::default(),
            state_msg: Default::default(),
            title: "lonote".to_owned(),
            modified: false,
            cur_file: None,
            allow_to_close: false,
        }
    }
}

struct File {
    pub path: PathBuf,
    pub last_modified_time: Duration,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let selector = if let Some(storage) = cc.storage {
            eframe::get_value(storage, UiFontSelector::KEY).unwrap_or_default()
        } else {
            UiFontSelector::default()
        };

        let mut this = Self {
            note: Rc::new(RefCell::new(Note::default())),
            dialog_cb: None,
            show_search_box: false,
            case_sense: true,
            search_words: String::default(),
            search_down: None,
            selector,
        };

        if let Some(file) = std::env::args().nth(1) {
            this.open(Some(file.into()));
        }

        this.rebuild_fonts(&cc.egui_ctx);
        this.selector.apply_text_style(&cc.egui_ctx);
        this
    }

    const NEW: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::N);

    const OPEN: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O);

    const REOPEN: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R);

    const SAVE: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

    const SAVE_AS: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::S);

    const SEARCH: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::F);

    const SEARCH_DOWN: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Num1);

    const SEARCH_UP: egui::KeyboardShortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Num2);

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if self.dialog_cb.is_none() {
            if ui.input_mut(|i| i.consume_shortcut(&Self::NEW)) {
                self.new_note();
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::OPEN)) {
                self.open(None);
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::REOPEN)) {
                self.reopen();
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::SAVE)) {
                self.save();
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::SAVE_AS)) {
                if let Err(err) = self.save_as() {
                    self.note.borrow_mut().state_msg = err.to_string();
                }
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::SEARCH)) {
                self.show_search_box = true;
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::SEARCH_DOWN)) {
                self.search_down = Some(true);
            }

            if ui.input_mut(|i| i.consume_shortcut(&Self::SEARCH_UP)) {
                self.search_down = Some(false);
            }
        }
    }

    fn process_close_request(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        if ctx.input(|i| i.viewport().close_requested())
            && self.note.borrow().modified
            && !self.note.borrow().allow_to_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.set_confirm_dialog(Self::FILE_UNSAVED.to_owned(), {
                let note = self.note.clone();
                let ctx = ctx.clone();
                move |yes| {
                    if yes {
                        note.borrow_mut().allow_to_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Ok(())
                }
            });
        }
    }

    fn set_confirm_dialog<F: FnOnce(bool) -> Result<()> + 'static>(&mut self, msg: String, cb: F) {
        assert!(self.dialog_cb.is_none());
        self.dialog_cb = Some((msg, Box::new(cb)));
    }

    fn try_search(&mut self, ui: &mut egui::Ui, id: egui::Id, mut output: TextEditOutput) {
        if let Some(down) = self.search_down.take() {
            if !self.search_words.is_empty() {
                let range = output
                    .cursor_range
                    .unwrap_or_default()
                    .as_sorted_char_range();

                let search_result = {
                    let contents = &self.note.borrow().contents;
                    let down_offset = byte_index_from_char_index(contents, range.end);
                    let contents = if down {
                        &contents[down_offset..]
                    } else {
                        &contents[..byte_index_from_char_index(contents, range.start)]
                    };
                    let contents = if self.case_sense {
                        Cow::Borrowed(contents)
                    } else {
                        Cow::Owned(contents.to_ascii_lowercase())
                    };
                    if down {
                        contents.find(&self.search_words).map(|v| v + down_offset)
                    } else {
                        contents.rfind(&self.search_words)
                    }
                };

                match search_result {
                    Some(new_bi) => {
                        let mut new_ci = None;
                        for (ci, (bi, _)) in self.note.borrow().contents.char_indices().enumerate()
                        {
                            if new_bi == bi {
                                new_ci = Some(ci);
                                break;
                            }
                        }

                        if let Some(new_ci_start) = new_ci {
                            let new_ci_end = new_ci_start + self.search_words.chars().count();
                            output.state.cursor.set_char_range(Some(CCursorRange::two(
                                CCursor::new(new_ci_start),
                                CCursor::new(new_ci_end),
                            )));
                            let primary_cursor_rect = cursor_rect(
                                &output.galley,
                                &output.state.cursor.range(&output.galley).unwrap().primary,
                                ui.fonts(|f| f.row_height(&get_body_font_id(ui))),
                            );

                            ui.scroll_to_rect(
                                egui::Rect::from_center_size(
                                    primary_cursor_rect.center() + output.galley_pos.to_vec2(),
                                    primary_cursor_rect.size(),
                                ),
                                None,
                            );
                            ui.ctx().request_repaint();
                            output.state.store(ui.ctx(), id);

                            self.note.borrow_mut().state_msg = "Found".to_owned();
                        }
                    }
                    None => {
                        self.note.borrow_mut().state_msg = "Search finished".to_owned();
                    }
                }
            }
        }
    }
}

impl App {
    fn ui_show_confirm_dialog(&mut self, ui: &mut egui::Ui) {
        if self.dialog_cb.is_some() {
            egui::Modal::new(egui::Id::new("Warning")).show(ui.ctx(), |ui| {
                let str = &self.dialog_cb.as_ref().unwrap().0;
                ui.label(str);
                ui.horizontal(|ui| {
                    let no_res = ui.button("No");
                    let yes_res = ui.button("Yes");
                    if no_res.clicked() || yes_res.clicked() {
                        let (.., cb) = self.dialog_cb.take().unwrap();
                        if let Err(err) = cb(yes_res.clicked()) {
                            self.note.borrow_mut().state_msg = err.to_string();
                        }
                    }
                });
            });
        }
    }

    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.add_space(8.0);
            ui.visuals_mut().button_frame = false;

            ui.menu_button("File", |ui| {
                macro_rules! btn {
                    ($name:literal, $shortcut:expr, $stmt:stmt) => {
                        let btn = egui::Button::new($name)
                            .shortcut_text(ui.ctx().format_shortcut($shortcut));
                        if ui.add(btn).clicked() {
                            $stmt
                            ui.close();
                        }
                    };
                }

                btn!("New...", &Self::NEW, self.new_note());
                btn!("Open...", &Self::OPEN, self.open(None));
                btn!("ReOpen", &Self::REOPEN, self.reopen());
                btn!("Save", &Self::SAVE, self.save());
                btn!(
                    "Save as...",
                    &Self::SAVE_AS,
                    if let Err(err) = self.save_as() {
                        self.note.borrow_mut().state_msg = err.to_string();
                    }
                );
                btn!("Search", &Self::SEARCH, self.show_search_box = true);
            });

            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                &self.note.borrow().title,
                get_body_font_id(ui),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect());

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(get_button_height(ui) + 16.0)
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| self.ui_bottom_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(ui.style().visuals.extreme_bg_color))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let rect = ui.max_rect();
                    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                let id = ui.make_persistent_id("text_edit");
                                let output =
                                    egui::TextEdit::multiline(&mut self.note.borrow_mut().contents)
                                        .frame(false)
                                        .margin(Margin::ZERO)
                                        .code_editor()
                                        .id(id)
                                        .show(ui);

                                if output.response.changed() && !self.note.borrow().modified {
                                    self.note.borrow_mut().modified = true;
                                    self.note.borrow_mut().update_title();
                                }

                                if output.response.dragged() {
                                    let pointer = ui.input(|i| i.pointer.clone());
                                    if let Some(mouse_pos) = pointer.interact_pos() {
                                        ui.scroll_to_rect(
                                            Rect::from_min_max(mouse_pos, mouse_pos),
                                            None,
                                        );
                                    }
                                }

                                self.try_search(ui, id, output);
                            },
                        );
                    });
                });
            });
    }

    fn ui_bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt("codec").show_index(
                ui,
                &mut self.note.borrow_mut().codec_idx,
                codec::supported_encodings().len(),
                |i| codec::supported_encodings()[i].name(),
            );

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if theme_button(ui, get_theme_button(ui)).clicked() {
                    self.selector.apply_text_style(ui.ctx());
                }

                if self.selector.ui_and_should_rebuild_fonts(ui) {
                    self.rebuild_fonts(ui.ctx());
                }

                ui.set_clip_rect(ui.max_rect());
                ui.label(&self.note.borrow().state_msg);
            });
        });
    }

    fn ui_show_search_box(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Search Box")
            .auto_sized()
            .open(&mut self.show_search_box)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(self.dialog_cb.is_none(), |ui| {
                    ui.text_edit_singleline(&mut self.search_words);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.case_sense, "case sense");

                        ui.label(format!(
                            " {}[{}] {}[{}]",
                            ICON_TRIANGLE_DOWN,
                            ui.ctx().format_shortcut(&Self::SEARCH_DOWN),
                            ICON_TRIANGLE_UP,
                            ui.ctx().format_shortcut(&Self::SEARCH_UP)
                        ));
                    });
                });
            });
    }
}

macro_rules! confirm_dialog_or_calling {
    ($self:expr, $note:ident, $block:block) => {
        #[allow(unused_mut)]
        let mut cb = {
            let $note = $self.note.clone();
            move |yes: bool| {
                if yes {
                    $block
                }
                Ok(())
            }
        };

        if $self.note.borrow().modified {
            $self.set_confirm_dialog(Self::FILE_UNSAVED.to_owned(), cb);
            return;
        }

        if let Err(err) = cb(true) {
            $self.note.borrow_mut().state_msg = err.to_string();
        }
    };
}

impl App {
    const FILE_UNSAVED: &'static str = "File unsaved, Do you wish to continue?";
    const FILE_HAS_MODIFIED: &'static str =
        "File has been modified since the last access, Do you wish to continue?";

    fn new_note(&mut self) {
        confirm_dialog_or_calling!(self, note, {
            let note = &mut *note.borrow_mut();
            note.contents.clear();
            note.cur_file = None;
            note.modified = false;
            note.update_title();
            note.state_msg = "New note".to_owned();
        });
    }

    fn open(&mut self, mut path: Option<std::path::PathBuf>) {
        confirm_dialog_or_calling!(self, note, {
            if path.is_none() {
                if let Some(open_path) =
                    rfd::FileDialog::new().add_filter("*", &["txt"]).pick_file()
                {
                    path = Some(open_path);
                }
            }

            if let Some(path) = path {
                let last_modified_time = Note::get_modified_time(&path)?;
                let (contents, codec_idx) = Note::read_from_file(&path, None)?;

                let note = &mut *note.borrow_mut();
                note.contents = contents;
                note.codec_idx = codec_idx;
                note.cur_file = Some(File {
                    path,
                    last_modified_time,
                });
                note.modified = false;
                note.update_title();
                note.state_msg = format!(
                    "Open successfully (Encoding: {})",
                    codec::supported_encodings()[codec_idx].name()
                );
            }
        });
    }

    fn reopen(&mut self) {
        if self.note.borrow().cur_file.is_none() {
            return;
        }

        confirm_dialog_or_calling!(self, note, {
            let note = &mut *note.borrow_mut();
            let path = note.get_path().unwrap();
            let last_modified_time = Note::get_modified_time(path)?;
            let (contents, codec_idx) = Note::read_from_file(path, Some(note.codec_idx))?;

            note.contents = contents;
            note.codec_idx = codec_idx;
            note.cur_file.as_mut().unwrap().last_modified_time = last_modified_time;
            note.modified = false;
            note.update_title();
            note.state_msg = format!(
                "Reopen successfully (Encoding: {})",
                codec::supported_encodings()[codec_idx].name()
            );
        });
    }

    fn save(&mut self) {
        if self.note.borrow().cur_file.is_none() {
            eapp_utils::capture_error!(err => self.note.borrow_mut().state_msg = err.to_string(), {
                let path = self.save_as()?;
                let note = &mut *self.note.borrow_mut();
                let last_modified_time = Note::get_modified_time(&path)?;
                note.cur_file = Some(File {
                    path,
                    last_modified_time,
                });
                note.modified = false;
                note.update_title();
                note.state_msg = "Save successfully".to_owned();
            });
            return;
        }

        let cb = {
            let note = self.note.clone();
            move |yes: bool| {
                if yes {
                    let note = &mut *note.borrow_mut();
                    let path = note.get_path().unwrap();
                    note.write_to_file(path)?;
                    note.cur_file.as_mut().unwrap().last_modified_time =
                        Note::get_modified_time(path)?;
                    note.modified = false;
                    note.update_title();
                    note.state_msg = "Save successfully".to_owned();
                }

                Ok(())
            }
        };

        let show_dialog = {
            let cur_file = &self.note.borrow().cur_file;
            let File {
                path,
                last_modified_time,
            } = cur_file.as_ref().unwrap();

            let Ok(modified_time) = Note::get_modified_time(path) else {
                return;
            };

            path.is_file() && modified_time > *last_modified_time
        };

        if show_dialog {
            self.set_confirm_dialog(Self::FILE_HAS_MODIFIED.to_owned(), cb);
            return;
        }

        if let Err(err) = cb(true) {
            self.note.borrow_mut().state_msg = err.to_string();
        }
    }

    fn save_as(&self) -> Result<std::path::PathBuf> {
        if let Some(save_path) = rfd::FileDialog::new().save_file() {
            self.note.borrow().write_to_file(&save_path)?;
            return Ok(save_path);
        }

        Err("Save path not specified".into())
    }

    fn rebuild_fonts(&mut self, ctx: &egui::Context) {
        let fonts = self.selector.insert_font(eapp_utils::get_default_fonts());
        ctx.set_fonts(fonts);
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, UiFontSelector::KEY, &self.selector);
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

            let app_rect = ui.max_rect();

            let title_bar_height = get_button_height(ui) + 16.0;
            let title_bar_rect = {
                let mut rect = app_rect;
                rect.max.y = rect.min.y + title_bar_height;
                rect
            };

            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink2(Vec2::new(1.5, 1.0));

            self.process_close_request(ui);
            self.process_inputs(ui);

            self.ui_title_bar(ui, title_bar_rect);
            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );

            self.ui_show_search_box(ui);
            self.ui_show_confirm_dialog(ui);
        });
    }
}
