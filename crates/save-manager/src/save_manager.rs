use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub struct SaveManager {
    #[serde(skip)]
    pub regex_str: String,

    pub main_save_dir: String,

    #[serde(skip)]
    pub main_save_dir_items: Vec<String>,

    #[serde(skip)]
    pub save_dirs: HashMap<String, Vec<String>>,

    #[serde(skip)]
    pub regex: Option<regex::Regex>,

    #[serde(skip)]
    pub regex_err_str: Option<String>,
}

enum RemoveCmd {
    RemoveAll,
    RemoveByRegex,
}

impl SaveManager {
    pub fn load_main_save_dir(&mut self) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        self.main_save_dir_items.clear();
        self.save_dirs.clear();

        let items = Self::search_dir_items(&self.main_save_dir)?;

        let info_path = Path::new(&self.main_save_dir).with_file_name("save_manager");

        let mut regex_str = None;
        let mut save_dirs = HashMap::new();

        std::fs::create_dir_all(&info_path)?;

        for item in std::fs::read_dir(&info_path)? {
            let path = item?.path();
            let filename = path.file_name().unwrap().to_string_lossy();
            if path.is_file() && filename == "regex.txt" {
                regex_str = Some(std::fs::read_to_string(&path)?);
            }

            if path.is_dir() {
                save_dirs.insert(filename.into_owned(), Self::search_dir_items(path)?);
            }
        }

        self.main_save_dir_items = items;
        self.save_dirs = save_dirs;

        if let Some(regex_str) = regex_str {
            self.regex_str = regex_str;
            self.build_regex_from_str();
        }

        Ok(())
    }

    pub fn build_regex_from_str(&mut self) {
        if self.regex_str.is_empty() {
            self.regex = None;
            self.regex_err_str = None;
        } else {
            self.regex = match regex::Regex::new(&self.regex_str) {
                Ok(v) => {
                    self.regex_err_str = None;
                    Some(v)
                }
                Err(err) => {
                    self.regex_err_str = Some(err.to_string());
                    None
                }
            };
        }
    }

    fn search_dir_items<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
        let mut items = Vec::new();
        for item in std::fs::read_dir(path)? {
            let path = item?.path();
            if !path.is_file() {
                continue;
            }

            items.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }

        Ok(items)
    }

    fn verify_main_save_dir(&self) -> std::io::Result<()> {
        let main_dir = Path::new(&self.main_save_dir);
        if !main_dir.is_dir() {
            return Err(std::io::Error::other(
                "Main save directory is empty or not a directory",
            ));
        }

        if main_dir.parent().is_none() {
            return Err(std::io::Error::other(
                "Main save directory should not be root directory",
            ));
        }

        Ok(())
    }

    pub fn save_regex(&self) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        let file_path = Path::new(&self.main_save_dir)
            .parent()
            .unwrap()
            .join("save_manager/regex.txt");

        std::fs::create_dir_all(file_path.parent().unwrap())?;
        std::fs::write(file_path, &self.regex_str)?;

        Ok(())
    }

    pub fn backup(&mut self, name: &str) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        if !self.save_dirs.contains_key(name) {
            return Err(std::io::Error::other(
                "Unable to find the directory specified for backup",
            ));
        }

        let main_dir = Path::new(&self.main_save_dir);
        let to_dir = Path::new(&self.main_save_dir)
            .parent()
            .unwrap()
            .join(format!("save_manager/{name}"));

        self.replace(main_dir, to_dir.as_path(), RemoveCmd::RemoveAll)?;
        *self.save_dirs.get_mut(name).unwrap() = Self::search_dir_items(to_dir)?;

        Ok(())
    }

    pub fn restore(&mut self, name: &str) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        if !self.save_dirs.contains_key(name) {
            return Err(std::io::Error::other(
                "Unable to find the directory specified for restore",
            ));
        }

        let main_dir = Path::new(&self.main_save_dir);
        let from_dir = Path::new(&self.main_save_dir)
            .parent()
            .unwrap()
            .join(format!("save_manager/{name}"));

        self.replace(from_dir.as_path(), main_dir, RemoveCmd::RemoveByRegex)?;
        self.main_save_dir_items = Self::search_dir_items(main_dir)?;

        Ok(())
    }

    pub fn add(&mut self, name: String) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        let dir_path = Path::new(&self.main_save_dir)
            .parent()
            .unwrap()
            .join(format!("save_manager/{name}"));
        std::fs::create_dir_all(&dir_path)?;

        self.save_dirs
            .insert(name, Self::search_dir_items(dir_path)?);

        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> std::io::Result<()> {
        self.verify_main_save_dir()?;

        let dir_path = Path::new(&self.main_save_dir)
            .parent()
            .unwrap()
            .join(format!("save_manager/{name}"));
        std::fs::remove_dir_all(dir_path)?;

        self.save_dirs.remove(name);

        Ok(())
    }

    fn replace<P: AsRef<Path>>(
        &self,
        from_dir: P,
        to_dir: P,
        cmd: RemoveCmd,
    ) -> std::io::Result<()> {
        let from_dir = from_dir.as_ref();
        let to_dir = to_dir.as_ref();

        if to_dir.exists() {
            match cmd {
                RemoveCmd::RemoveAll => std::fs::remove_dir_all(to_dir)?,
                RemoveCmd::RemoveByRegex => {
                    for item in std::fs::read_dir(to_dir)? {
                        let path = item?.path();
                        if !path.is_file() {
                            continue;
                        }

                        if let Some(reg) = self.regex.as_ref() {
                            let filename = path.file_name().unwrap().to_string_lossy();
                            if reg.is_match(&filename) {
                                std::fs::remove_file(path)?;
                            }
                        } else {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
            }
        }

        std::fs::create_dir_all(to_dir)?;

        for item in std::fs::read_dir(from_dir)? {
            let path = item?.path();
            if !path.is_file() {
                continue;
            }

            let from = if let Some(reg) = self.regex.as_ref() {
                if reg.is_match(&path.file_name().unwrap().to_string_lossy()) {
                    Some(path)
                } else {
                    None
                }
            } else {
                Some(path)
            };

            if let Some(from) = from {
                let to = to_dir.join(from.file_name().unwrap());
                std::fs::copy(from, to)?;
            }
        }

        Ok(())
    }
}
