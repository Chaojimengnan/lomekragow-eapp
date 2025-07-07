use eapp_utils::codicons::{ICON_ERROR, ICON_PIN, ICON_PINNED, ICON_REPLY};
use eframe::egui::{
    self, TextEdit,
    collapsing_header::CollapsingState,
    text::{CCursor, CCursorRange},
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::Path,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    Choices(#[serde(skip)] usize),
    Normal(#[serde(skip)] String),
    OneLine(#[serde(skip)] String),
    StoreTrue(#[serde(skip)] bool),
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Arg {
    pub name: String,
    pub r#type: ArgType,
    pub default: Option<String>,

    #[serde(default)]
    pub choices: Vec<String>,

    #[serde(default)]
    pub optional: bool,

    #[serde(default)]
    pub desc: String,

    #[serde(default)]
    pub password: bool,

    #[serde(default = "default_as_true")]
    pub remember: bool,

    #[serde(skip)]
    pub enabled: bool,

    #[serde(default)]
    pub existing_path: bool,
}

fn default_as_true() -> bool {
    true
}

impl Arg {
    pub fn optional_and_disabled(&self) -> bool {
        self.optional && !self.enabled
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        let name = if self.name.starts_with("--") {
            &self.name[2..]
        } else {
            &self.name
        };

        CollapsingState::load_with_default_open(ui.ctx(), egui::Id::new(name), true)
            .show_header(ui, |ui| self.show_ui_header(ui))
            .body(|ui| self.show_ui_body(ui));
    }

    fn show_ui_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let name = if self.name.starts_with("--") {
                &self.name[2..]
            } else {
                &self.name
            };

            if self.optional {
                let icon = if self.enabled {
                    ICON_PINNED.to_string()
                } else {
                    ICON_PIN.to_string()
                };
                ui.toggle_value(&mut self.enabled, icon)
                    .on_hover_text("Enable this option");
            }

            ui.label(name).on_hover_text(&self.desc);
        });
    }

    fn show_ui_body(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.enabled, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                let oneline = matches!(self.r#type, ArgType::OneLine(_));

                match &mut self.r#type {
                    ArgType::Choices(value) => {
                        egui::ComboBox::from_id_salt(format!("{}_combo", self.name))
                            .selected_text(&self.choices[*value])
                            .show_index(ui, value, self.choices.len(), |i| &self.choices[i]);
                    }
                    ArgType::OneLine(value) | ArgType::Normal(value) => {
                        let id = ui.make_persistent_id("text_edit");
                        let mut output = if oneline {
                            TextEdit::singleline(value)
                        } else {
                            TextEdit::multiline(value)
                        }
                        .code_editor()
                        .password(self.password)
                        .id(id)
                        .show(ui);

                        let mut response = output.response;

                        if self.existing_path {
                            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                                if let Some(mut cursor_range) = output.state.cursor.char_range() {
                                    if path_utils::tab_path_completion(value, &mut cursor_range) {
                                        response.mark_changed();
                                        output.state.cursor.set_char_range(Some(cursor_range));
                                        output.state.store(ui.ctx(), id);
                                    }
                                }
                            }

                            let errors = path_utils::check_path_existence(value);
                            if !errors.is_empty() {
                                ui.menu_button(ICON_ERROR.to_string(), |ui| {
                                    for error in &errors {
                                        ui.label(error);
                                    }
                                });
                            }
                        }
                    }
                    ArgType::StoreTrue(value) => {
                        ui.checkbox(value, "Append this option");
                    }
                }

                if self.default.is_some()
                    && ui
                        .button(ICON_REPLY.to_string())
                        .on_hover_text("Reset value to default")
                        .clicked()
                {
                    self.set_value(self.default.clone());
                }
            });
        });
    }

    pub fn set_value(&mut self, opt_value: Option<String>) {
        let Self { r#type, .. } = self;
        match r#type {
            ArgType::Choices(value) => {
                *value = opt_value
                    .map(|v| {
                        self.choices
                            .iter()
                            .position(|choice| *choice == v)
                            .unwrap_or(0usize)
                    })
                    .unwrap_or(0usize);
            }
            ArgType::Normal(value) | ArgType::OneLine(value) => {
                *value = opt_value.unwrap_or_default();
            }
            ArgType::StoreTrue(value) => {
                *value = opt_value
                    .unwrap_or("false".to_owned())
                    .parse::<bool>()
                    .ok()
                    .unwrap_or(false);
            }
        }
    }

    pub fn get_value(&self) -> String {
        match &self.r#type {
            ArgType::Choices(index) => self.choices.get(*index).cloned().unwrap_or_default(),
            ArgType::Normal(s) | ArgType::OneLine(s) => s.clone(),
            ArgType::StoreTrue(b) => b.to_string(),
        }
    }

    pub fn get_value_formatted(&self) -> String {
        let mut value_string = String::new();

        match &self.r#type {
            ArgType::Choices(value) => {
                let len = self.choices.len();
                if *value < len {
                    value_string = format!("{} {}", self.name, self.choices[*value]);
                }
            }
            ArgType::Normal(value) => {
                let value = value.clone().replace(['\n', '\r'], " ");
                value_string = format!("{} {}", self.name, value);
            }
            ArgType::OneLine(value) => {
                value_string = format!("{} {}", self.name, value);
            }
            ArgType::StoreTrue(value) => {
                if *value {
                    value_string = self.name.clone();
                }
            }
        }

        value_string
    }

    pub fn initialize_value(&mut self) {
        if matches!(self.r#type, ArgType::StoreTrue(_)) {
            self.optional = false;
        }

        if !self.optional {
            self.enabled = true;
        }

        if self.password {
            self.remember = false;
        }

        if self.existing_path && !matches!(self.r#type, ArgType::Normal(_) | ArgType::OneLine(_)) {
            self.existing_path = false;
        }

        self.set_value(self.default.clone());
    }
}

mod path_utils {
    use super::*;
    use eframe::egui::text_selection::text_cursor_state::byte_index_from_char_index;

    pub fn check_path_existence(value: &str) -> Vec<String> {
        let mut errors = Vec::new();

        for (line_num, line) in value.lines().enumerate() {
            let path = line.trim();
            if !path.is_empty() && !std::path::Path::new(path).exists() {
                errors.push(format!(
                    "Line {}: Path '{}' does not exist",
                    line_num + 1,
                    path
                ));
            }
        }

        errors
    }

    pub fn tab_path_completion(value: &mut String, cursor_range: &mut CCursorRange) -> bool {
        let text = value.as_str();
        let primary_char_index = cursor_range.primary.index;

        let byte_index = byte_index_from_char_index(text, primary_char_index);

        let line_start_byte = text[..byte_index]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let line_end_byte = text[byte_index..]
            .find('\n')
            .map(|pos| byte_index + pos)
            .unwrap_or_else(|| text.len());

        let current_line = &text[line_start_byte..line_end_byte];
        let path = current_line.trim();

        if path.is_empty() {
            return false;
        }

        let expanded_path = shellexpand::tilde(path).to_string();
        let path_buf = std::path::PathBuf::from(&expanded_path);

        let (parent, current_file_name) = if let Some(parent) = path_buf.parent() {
            let file_name = path_buf
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), file_name)
        } else {
            (
                std::path::PathBuf::from("."),
                path_buf.to_string_lossy().to_string(),
            )
        };

        let entries = match read_dir_entries(&parent) {
            Ok(entries) => entries,
            Err(_) => return false,
        };

        let (new_name, is_dir) = match entries
            .iter()
            .position(|(name, _)| name == &current_file_name)
        {
            Some(current_index) => {
                let next_index = (current_index + 1) % entries.len();
                &entries[next_index]
            }
            None => match entries
                .iter()
                .find(|(name, _)| name.starts_with(&current_file_name))
            {
                Some(entry) => entry,
                None => return false,
            },
        };

        let mut new_path = parent.join(new_name).to_string_lossy().to_string();
        if *is_dir {
            new_path.push(std::path::MAIN_SEPARATOR);
        }

        let mut new_text = String::new();
        new_text.push_str(&text[..line_start_byte]);
        new_text.push_str(&new_path);
        new_text.push_str(&text[line_end_byte..]);

        let new_path_char_count = new_path.chars().count();
        let prefix_char_count = text[..line_start_byte].chars().count();
        let new_cursor_char_index = prefix_char_count + new_path_char_count;

        *cursor_range = CCursorRange::one(CCursor::new(new_cursor_char_index));
        *value = new_text;

        true
    }

    fn read_dir_entries(path: &std::path::Path) -> std::io::Result<Vec<(String, bool)>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type()?;
            entries.push((file_name, file_type.is_dir()));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Script {
    pub name: String,

    #[serde(default)]
    pub desc: Vec<String>,

    #[serde(default)]
    pub require_admin: bool,

    #[serde(default)]
    pub tag: HashSet<String>,

    #[serde(default)]
    pub args: Vec<Arg>,

    #[serde(skip)]
    pub desc_cache: Option<(CommonMarkCache, String)>,
}

impl Script {
    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        if self.desc_cache.is_none() {
            let markdown_desc = self.desc.join("\n");
            self.desc_cache = Some((CommonMarkCache::default(), markdown_desc));
        }

        let (cache, desc) = self.desc_cache.as_mut().unwrap();
        CommonMarkViewer::new().show(ui, cache, desc);

        ui.add_space(16.0);
        self.args.iter_mut().for_each(|arg| arg.show_ui(ui));
    }

    pub fn generate_args_string(&self) -> String {
        let mut result = String::new();

        for arg in &self.args {
            if arg.optional_and_disabled() {
                continue;
            }

            write!(&mut result, "{} ", arg.get_value_formatted()).unwrap();
        }

        result
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Loader {
    pub tag_list: Vec<String>,
    pub script_list: Vec<Script>,

    #[serde(skip)]
    pub script_path: String,
}

pub type RememberedArgs = HashMap<(String, String), String>;

impl Loader {
    const INFO_FILENAME: &'static str = "info.json";

    pub fn load(
        info_json_path: Option<&str>,
        remembered_args: &RememberedArgs,
    ) -> anyhow::Result<Self> {
        let script_path = info_json_path
            .map(|path| {
                if let Some(parent) = Path::new(path).parent() {
                    parent.to_string_lossy().to_string()
                } else {
                    String::new()
                }
            })
            .ok_or(anyhow::anyhow!("Invalid info json path"))?;

        let info_path = format!("{}/{}", script_path, Self::INFO_FILENAME);
        let json = std::fs::read_to_string(&info_path)?;
        let mut loader = serde_json::from_str::<Loader>(&json)?;
        loader.script_list.iter_mut().for_each(|script| {
            script.args.iter_mut().for_each(|arg| {
                arg.initialize_value();
                if let Some(value) = remembered_args.get(&(script.name.clone(), arg.name.clone())) {
                    arg.set_value(Some(value.to_owned()));
                }
            })
        });
        loader.script_path = script_path;
        Ok(loader)
    }

    pub fn generate_remembered_args(&self) -> RememberedArgs {
        let mut remembered_args = HashMap::new();

        for script in &self.script_list {
            for arg in &script.args {
                if arg.remember {
                    let value = arg.get_value();
                    if !value.is_empty() && Some(&value) != arg.default.as_ref() {
                        remembered_args.insert((script.name.clone(), arg.name.clone()), value);
                    }
                }
            }
        }

        remembered_args
    }
}

pub fn runas_admin(script_path: &str, args: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::UI::Shell::{
            SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHELLEXECUTEINFOW_0, ShellExecuteExW,
        };

        let mut args: Vec<_> = format!("python -i \"{script_path}\" {args}")
            .encode_utf16()
            .collect();
        let mut verb: Vec<_> = "runas".encode_utf16().collect();
        let mut file: Vec<_> = "wt".encode_utf16().collect();
        args.push(0);
        verb.push(0);
        file.push(0);

        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: args.as_ptr(),
            nShow: 1,
            hwnd: 0,
            lpDirectory: std::ptr::null(),
            hInstApp: 0,
            lpIDList: std::ptr::null_mut(),
            lpClass: std::ptr::null(),
            hkeyClass: 0,
            dwHotKey: 0,
            Anonymous: SHELLEXECUTEINFOW_0 { hIcon: 0 },
            hProcess: 0,
        };

        if ShellExecuteExW(&mut info) == 0 {
            let error_code = GetLastError();
            anyhow::bail!("error with code: {error_code}");
        }
    }

    #[cfg(not(windows))]
    unimplemented!();

    Ok(())
}

pub fn runas_normal(script_path: &str, args: &str) -> anyhow::Result<()> {
    std::process::Command::new("wt")
        .args(
            ["python", "-i", script_path]
                .as_slice()
                .iter()
                .chain(args.split_whitespace().collect::<Vec<&str>>().iter()),
        )
        .spawn()?;

    Ok(())
}
