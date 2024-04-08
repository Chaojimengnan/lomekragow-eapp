use crate::codec;
use eframe::egui::{self, vec2, Color32, Margin, Vec2};
use std::{
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
        };

        if let Some(file) = std::env::args().nth(1) {
            this.open(Some(file.into()));
        }

        this
    }

    fn process_inputs(&mut self, ui: &mut egui::Ui) {
        if self.dialog_cb.is_none() {
            if ui.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.ctrl) {
                self.new_note();
            }

            if ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.ctrl) {
                self.open(None);
            }

            if ui.input(|i| i.key_pressed(egui::Key::R) && i.modifiers.ctrl) {
                self.reopen();
            }

            if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl) {
                self.save();
            }

            if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.alt) {
                if let Err(err) = self.save_as() {
                    self.note.borrow_mut().state_msg = err.to_string();
                }
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
                    ($name:literal, $stmt:stmt) => {
                        if ui.button($name).clicked() {
                            $stmt
                            ui.close_menu();
                        }
                    };
                }
                btn!("New...     Ctrl+N", self.new_note());
                btn!("Open...    Ctrl+O", self.open(None));
                btn!("ReOpen     Ctrl+R", self.reopen());
                btn!("Save       Ctrl+S", self.save());
                btn!(
                    "Save as...  Alt+S",
                    if let Err(err) = self.save_as() {
                        self.note.borrow_mut().state_msg = err.to_string();
                    }
                );
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
                                let output =
                                    egui::TextEdit::multiline(&mut self.note.borrow_mut().contents)
                                        .id_source("text_edit")
                                        .code_editor()
                                        .frame(false)
                                        .margin(Margin::ZERO)
                                        .show(ui);

                                if output.response.changed() && !self.note.borrow().modified {
                                    self.note.borrow_mut().modified = true;
                                    self.note.borrow_mut().update_title();
                                }
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

            ui.add_enabled_ui(self.dialog_cb.is_none(), |ui| {
                self.ui_title_bar(ui, title_bar_rect);
                self.ui_contents(&mut ui.child_ui(content_rect, *ui.layout()));
            });

            self.ui_show_confirm_dialog(ui, app_rect.center());

            self.process_inputs(ui);
        });
    }
}
