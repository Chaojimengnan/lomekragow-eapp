use crate::codec;
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
    text_edit::TextEditOutput,
    text_selection::text_cursor_state::{byte_index_from_char_index, cursor_rect},
    vec2, Color32, Margin, Vec2,
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

    pub fn read_from_file<P>(&self, path: P) -> Result<String>
    where
        P: AsRef<std::path::Path>,
    {
        let contents = std::fs::read(path.as_ref())?;

        if self.codec_idx == 0 {
            return Ok(String::from_utf8_lossy(&contents).into_owned());
        }

        let encoding = codec::get_codec_list()[self.codec_idx];
        Ok(codec::decode_to_utf8(encoding, &contents))
    }

    pub fn write_to_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<std::path::Path>,
    {
        if self.codec_idx == 0 {
            return Ok(std::fs::write(path, &self.contents)?);
        }

        let encoding = codec::get_codec_list()[self.codec_idx];
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
        eapp_utils::setup_fonts(&cc.egui_ctx);
        let mut this = Self {
            note: Rc::new(RefCell::new(Note::default())),
            dialog_cb: None,
            show_search_box: false,
            case_sense: true,
            search_words: String::default(),
            search_down: None,
        };

        if let Some(file) = std::env::args().nth(1) {
            this.open(Some(file.into()));
        }

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
                                output.galley_pos,
                                &output.galley,
                                &output.state.cursor.range(&output.galley).unwrap().primary,
                                16.0,
                            );
                            ui.scroll_to_rect(primary_cursor_rect, None);
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
    fn ui_show_confirm_dialog(&mut self, ui: &mut egui::Ui, pos: egui::Pos2) {
        if self.dialog_cb.is_some() {
            egui::Window::new("Warning")
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
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
        eapp_utils::borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.add_space(8.0);
            ui.visuals_mut().button_frame = false;

            ui.menu_button("File", |ui| {
                macro_rules! btn {
                    ($name:literal, $shortcut:expr, $stmt:stmt) => {
                        let btn = egui::Button::new($name)
                            .shortcut_text(ui.ctx().format_shortcut($shortcut));
                        if ui.add(btn).clicked() {
                            $stmt
                            ui.close_menu();
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
                egui::FontId::proportional(16.0),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect());

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(32.0)
            .frame(egui::Frame::default().rounding(egui::Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: 8.0,
                se: 8.0,
            }))
            .show_inside(ui, |ui| self.ui_bottom_panel(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_gray(10)))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let rect = ui.max_rect().shrink2(vec2(8.0, 0.0));
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                let id = ui.make_persistent_id("text_edit");
                                let output =
                                    egui::TextEdit::multiline(&mut self.note.borrow_mut().contents)
                                        .frame(false)
                                        .margin(Margin::ZERO)
                                        .id(id)
                                        .show(ui);

                                if output.response.changed() && !self.note.borrow().modified {
                                    self.note.borrow_mut().modified = true;
                                    self.note.borrow_mut().update_title();
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
            ui.add_space(8.0);
            egui::ComboBox::from_id_source("codec").show_index(
                ui,
                &mut self.note.borrow_mut().codec_idx,
                codec::get_codec_list().len(),
                |i| codec::get_codec_list()[i].name(),
            );

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_clip_rect(ui.max_rect());
                ui.add_space(8.0);
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
                            eapp_utils::codicons::ICON_TRIANGLE_DOWN,
                            ui.ctx().format_shortcut(&Self::SEARCH_DOWN),
                            eapp_utils::codicons::ICON_TRIANGLE_UP,
                            ui.ctx().format_shortcut(&Self::SEARCH_UP)
                        ));
                    });
                });
            });
    }
}

macro_rules! confirm_dialog_or_calling {
    ($self:expr, $note:ident, $block:block) => {
        let cb = {
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

            if path.is_some() {
                let path = path.unwrap();
                let last_modified_time = Note::get_modified_time(&path)?;

                let note = &mut *note.borrow_mut();
                note.contents = note.read_from_file(&path)?;
                note.cur_file = Some(File {
                    path,
                    last_modified_time,
                });
                note.modified = false;
                note.update_title();
                note.state_msg = "Open successfully".to_owned();
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
            note.contents = note.read_from_file(path)?;
            note.cur_file.as_mut().unwrap().last_modified_time = last_modified_time;
            note.modified = false;
            note.update_title();
            note.state_msg = "Reopen successfully".to_owned();
        });
    }

    fn save(&mut self) {
        if self.note.borrow().cur_file.is_none() {
            eapp_utils::capture_error!(
                err,
                {
                    self.note.borrow_mut().state_msg = err.to_string();
                },
                {
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
                }
            );
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
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eapp_utils::borderless::window_frame(ctx, None).show(ctx, |ui| {
            eapp_utils::borderless::handle_resize(ui);

            let app_rect = ui.max_rect();

            let title_bar_height = 32.0;
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

            ui.add_enabled_ui(self.dialog_cb.is_none(), |ui| {
                self.ui_title_bar(ui, title_bar_rect);
                self.ui_contents(&mut ui.child_ui(content_rect, *ui.layout()));
            });

            self.ui_show_search_box(ui);
            self.ui_show_confirm_dialog(ui, app_rect.center());
        });
    }
}
