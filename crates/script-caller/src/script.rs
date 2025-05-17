use eapp_utils::easy_mark;
use eframe::egui::{self, CollapsingHeader};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashSet, fmt::Write};

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

    #[serde(skip)]
    pub enabled: bool,
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

        CollapsingHeader::new(name)
            .default_open(true)
            .show(ui, |ui| {
                if self.optional {
                    ui.checkbox(&mut self.enabled, "Enable this option");
                }
                ui.add_enabled_ui(self.enabled, |ui| match &mut self.r#type {
                    ArgType::Choices(value) => {
                        egui::ComboBox::from_id_salt(format!("{}_combo", self.name))
                            .selected_text(&self.choices[*value])
                            .show_index(ui, value, self.choices.len(), |i| &self.choices[i]);
                    }
                    ArgType::Normal(value) => {
                        ui.text_edit_multiline(value);
                    }
                    ArgType::OneLine(value) => {
                        ui.text_edit_singleline(value);
                    }
                    ArgType::StoreTrue(value) => {
                        ui.checkbox(value, "Append this option");
                    }
                })
            })
            .header_response
            .on_hover_text(&self.desc);
    }

    pub fn get_value_string(&self) -> String {
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
        if !self.optional {
            self.enabled = true;
        }

        let Self { r#type, .. } = self;
        match r#type {
            ArgType::Choices(value) => {
                let mut default = 0usize;
                if let Some(default_str) = &self.default {
                    for (i, choice) in self.choices.iter().enumerate() {
                        if default_str == choice {
                            default = i;
                            break;
                        }
                    }
                }
                *value = default;
            }
            ArgType::Normal(value) | ArgType::OneLine(value) => {
                let default = self.default.clone().unwrap_or_default();
                *value = default;
            }
            ArgType::StoreTrue(value) => {
                let default = self
                    .default
                    .clone()
                    .unwrap_or("false".to_owned())
                    .parse::<bool>()
                    .ok()
                    .unwrap_or(false);
                *value = default;
            }
        }
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
    pub desc_cache: Option<Vec<easy_mark::parser::Item>>,
}

impl Script {
    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        if self.desc_cache.is_none() {
            self.desc_cache = Some(easy_mark::parser::Parser::new(&self.desc.join("\n")).collect());
        }

        easy_mark::viewer::easy_mark_it(ui, self.desc_cache.as_ref().unwrap().iter());
        ui.add_space(16.0);
        self.args.iter_mut().for_each(|arg| arg.show_ui(ui));
    }

    pub fn generate_args_string(&self) -> String {
        let mut result = String::new();

        for arg in &self.args {
            if arg.optional_and_disabled() {
                continue;
            }

            write!(&mut result, "{} ", arg.get_value_string()).unwrap();
        }

        result
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Loader {
    pub tag_list: Vec<String>,
    pub script_list: Vec<Script>,

    #[serde(skip)]
    pub script_path: String,
}

impl Loader {
    const FILENAME: &'static str = "python_path.json";
    const INFO_FILENAME: &'static str = "info.json";

    pub fn load() -> std::io::Result<Self> {
        let path = std::env::current_exe()?
            .parent()
            .unwrap()
            .join(Self::FILENAME);

        let script_path =
            serde_json::from_str::<Value>(&std::fs::read_to_string(path)?)?["python_path"]
                .as_str()
                .ok_or(std::io::Error::other("Cannot found 'script_path' in json"))?
                .to_owned();

        let mut this = serde_json::from_str::<Loader>(&std::fs::read_to_string(format!(
            "{}/{}",
            script_path,
            Self::INFO_FILENAME
        ))?)?;

        this.script_list.iter_mut().for_each(|script| {
            script
                .args
                .iter_mut()
                .for_each(|arg| arg.initialize_value())
        });

        this.script_path = script_path;

        Ok(this)
    }
}

pub fn runas_admin(script_path: &str, args: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::UI::Shell::{
            SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHELLEXECUTEINFOW_0, ShellExecuteExW,
        };

        let mut args: Vec<_> = format!("python -i \"{}\" {}", script_path, args)
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

        ShellExecuteExW(std::ptr::from_mut(&mut info));

        let error_code = GetLastError();
        if error_code != 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn load_test() -> std::io::Result<()> {
        let loader = Loader::load()?;
        println!("{:?}", loader);
        println!("\n\n");
        for script in &loader.script_list {
            println!("{}", script.generate_args_string());
        }
        Ok(())
    }
}
