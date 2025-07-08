use crate::sync::{self, ItemCmd, Syncer};
use eapp_utils::{
    borderless,
    codicons::{ICON_FOLDER, ICON_SETTINGS_GEAR},
    get_body_font_id,
    widgets::simple_widgets::{get_theme_button, theme_button, toggle_ui},
};
use eframe::egui::{self, Color32, RichText, UiBuilder, Vec2, Widget};
use serde::{Deserialize, Serialize};
use std::thread::JoinHandle;

pub struct App {
    state: State,
    syncer: Option<Syncer>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct State {
    /// Source directory for synchronization
    pub source: String,

    /// Target directory for synchronization
    ///
    /// All new versions of files in the [`State::source`] are synchronized to [`State::target`]
    pub target: String,

    /// Only get items that need to be synchronized (Not [`ItemCmd::Keep`])
    pub only_sync: bool,

    /// Allow delete [`State::target`] items that do not exist in [`State::source`]
    pub allow_delete: bool,

    /// Items from source directory for synchronization
    #[serde(skip)]
    pub items: Vec<sync::Item>,

    /// Message shown in the status bar
    #[serde(skip)]
    pub msg: String,
}

impl State {
    pub fn get_items(&mut self) {
        if let Err(err) = sync::get_items(
            &self.source,
            &self.target,
            &mut self.items,
            self.only_sync,
            self.allow_delete,
        ) {
            self.msg = err.to_string();
            self.items.clear();
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        eapp_utils::setup_fonts(&cc.egui_ctx);

        let state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            State::default()
        };

        let (syncer, handle) = {
            let (syncer, handle) = Syncer::new(&cc.egui_ctx);
            (Some(syncer), Some(handle))
        };

        Self {
            state,
            syncer,
            handle,
        }
    }

    fn update_syncer(&mut self) {
        let syncer = self.syncer.as_mut().unwrap();
        while let Some(result) = syncer.update_once(&mut self.state.items) {
            match result {
                Ok(true) => {
                    if self.state.allow_delete {
                        if let Err(err) = sync::remove_empty_dirs(&self.state.target) {
                            self.state.msg = err.to_string();
                        }
                    }
                    self.state.get_items();
                }
                Ok(false) => (),
                Err(err) => self.state.msg = err,
            }
        }
    }
}

impl App {
    fn ui_title_bar(&mut self, ui: &mut egui::Ui, title_bar_rect: egui::Rect) {
        borderless::title_bar(ui, title_bar_rect, |ui| {
            ui.add_space(8.0);
            ui.visuals_mut().button_frame = false;

            theme_button(ui, get_theme_button(ui));

            let synchronizing = self.syncer.as_ref().unwrap().synchronizing();

            ui.add_enabled_ui(!synchronizing, |ui| {
                ui.menu_button(ICON_SETTINGS_GEAR.to_string(), |ui| {
                    ui.checkbox(&mut self.state.only_sync, "Only sync");
                    ui.checkbox(&mut self.state.allow_delete, "Allow delete");
                });
            });

            ui.painter().text(
                title_bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                "syncer",
                get_body_font_id(ui),
                ui.style().visuals.text_color(),
            );
        });
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect());

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(32.0)
            .frame(egui::Frame::side_top_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_animated_inside(ui, !self.state.msg.is_empty(), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        self.state.msg.clear();
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_clip_rect(ui.max_rect());
                        ui.label(&self.state.msg);
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| {
                fn directory_line(ui: &mut egui::Ui, path: &mut String, label: &str) {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        if ui
                            .add(egui::Button::new(ICON_FOLDER.to_string()).frame(false))
                            .clicked()
                        {
                            if let Some(dir_path) = rfd::FileDialog::new().pick_folder() {
                                *path = dir_path.to_string_lossy().into_owned();
                            }
                        }
                        egui::TextEdit::singleline(path)
                            .desired_width(f32::INFINITY)
                            .show(ui);
                    });
                }

                let synchronizing = self.syncer.as_ref().unwrap().synchronizing();

                ui.add_enabled_ui(!synchronizing, |ui| {
                    directory_line(ui, &mut self.state.source, "source directory");
                    directory_line(ui, &mut self.state.target, "target directory");
                });

                ui.columns(3, |ui| {
                    macro_rules! btn {
                        ($i:literal, $name:literal, $condition:expr, $expr:expr) => {
                            ui[$i].vertical_centered_justified(|ui| {
                                if ui
                                    .add_enabled($condition, egui::Button::new($name))
                                    .clicked()
                                {
                                    $expr;
                                }
                            });
                        };
                    }

                    let syncer = self.syncer.as_mut().unwrap();
                    let synchronizing = syncer.synchronizing();

                    btn!(0, "refresh", !synchronizing, self.state.get_items());
                    btn!(1, "sync", !synchronizing, syncer.sync(&self.state.items));

                    let synchronizing = syncer.synchronizing();
                    btn!(2, "cancel", synchronizing, syncer.cancel());
                });

                ui.separator();

                let synchronizing = self.syncer.as_ref().unwrap().synchronizing();
                if synchronizing {
                    ui.label(format!(
                        "Synchronizing: {} / {}",
                        self.state
                            .items
                            .iter()
                            .filter(|item| item.progress == 1.0)
                            .count(),
                        self.state
                            .items
                            .iter()
                            .filter(|item| item.should_sync())
                            .count()
                    ));
                }

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        self.state.items.len(),
                        |ui, range| {
                            ui.add_enabled_ui(!synchronizing, |ui| self.ui_items(ui, range))
                        },
                    )
            });
    }

    fn ui_items(&mut self, ui: &mut egui::Ui, range: std::ops::Range<usize>) {
        let synchronizing = self.syncer.as_ref().unwrap().synchronizing();

        for i in range {
            ui.horizontal(|ui| {
                toggle_ui(ui, &mut self.state.items[i].ignore);

                let item = &self.state.items[i];

                if ui.button("show").clicked() {
                    eapp_utils::open_in_explorer(item.get_path().to_string_lossy().as_ref());
                }

                if synchronizing && item.should_sync() {
                    egui::ProgressBar::new(item.progress)
                        .text(format!(
                            "{}% => {}",
                            (item.progress * 100.0) as usize,
                            item.filename
                        ))
                        .ui(ui);
                } else {
                    let bg_col = match item.cmd {
                        ItemCmd::Create => Color32::from_rgb(0, 156, 0),
                        ItemCmd::Replace => Color32::from_rgb(156, 156, 0),
                        ItemCmd::Delete => Color32::from_rgb(200, 40, 40),
                        ItemCmd::Keep => ui.visuals().window_fill,
                    };
                    let col = if bg_col != ui.visuals().window_fill {
                        ui.visuals().strong_text_color()
                    } else {
                        Color32::PLACEHOLDER
                    };

                    let mut text = RichText::new(&item.filename)
                        .color(col)
                        .background_color(bg_col);

                    if item.ignore {
                        text = text.strikethrough();
                    }

                    ui.label(text)
                        .on_hover_text(item.get_path().to_string_lossy());
                }
            });
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        borderless::window_frame(ctx, Some(ctx.style().visuals.window_fill)).show(ctx, |ui| {
            borderless::handle_resize(ui);

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

            self.update_syncer();

            self.ui_title_bar(ui, title_bar_rect);

            self.ui_contents(
                &mut ui.new_child(UiBuilder::new().layout(*ui.layout()).max_rect(content_rect)),
            );
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        drop(self.syncer.take());
        self.handle.take().unwrap().join().unwrap();
    }
}
