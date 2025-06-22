use eapp_utils::natordset::NatOrdSet;
use std::{path::Path, slice::Iter, sync::mpsc::Receiver};
use walkdir::WalkDir;

#[derive(Default, Clone, Debug)]
pub struct ImgFinder {
    search_dir: Option<String>,
    cur_image: Option<usize>,
    cur_dir: Option<usize>,
    cur_image_set: NatOrdSet,
    cur_dir_set: NatOrdSet,
    dir_changed: bool,
}

impl ImgFinder {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_supported_ext(ext: &str) -> bool {
        let ext = ext.to_ascii_lowercase();
        image::ImageFormat::from_extension(ext).is_some_and(|fmt| fmt.can_read())
    }

    fn is_dir_has_supported_image(dir: &Path) -> std::io::Result<bool> {
        for item in std::fs::read_dir(dir)? {
            let item = item?.path();
            if item.is_file()
                && item
                    .extension()
                    .is_some_and(|ext| Self::is_supported_ext(ext.to_str().unwrap_or("")))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn is_subpath(&self, canonicalized_path: &Path) -> bool {
        if let Some(search_dir) = &self.search_dir {
            let search_dir_path = Path::new(search_dir);
            canonicalized_path.starts_with(search_dir_path)
        } else {
            false
        }
    }

    pub fn set_path(&mut self, canonicalized_path: &Path) {
        if canonicalized_path.is_file() {
            self.set_cur_dir(&canonicalized_path.parent().unwrap().to_string_lossy());
            self.set_cur_image(&canonicalized_path.to_string_lossy());
        } else if canonicalized_path.is_dir() {
            self.set_cur_dir(&canonicalized_path.to_string_lossy());
        }
    }

    pub fn search_dir(&self) -> Option<&String> {
        self.search_dir.as_ref()
    }

    pub fn from_search(
        canonicalized_path: &Path,
        cancel_receiver: Receiver<()>,
    ) -> std::io::Result<Self> {
        let search_dir = if canonicalized_path.is_file() {
            canonicalized_path.parent().unwrap()
        } else {
            canonicalized_path
        };

        let search_dir_str = search_dir.to_string_lossy().into_owned();

        let mut finder = Self {
            search_dir: Some(search_dir_str.clone()),
            ..Default::default()
        };

        for (i, entry) in WalkDir::new(search_dir)
            .same_file_system(true)
            .contents_first(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .enumerate()
        {
            if i % 50 == 0 && cancel_receiver.try_recv().is_ok() {
                return Err(std::io::Error::other("Search canceled"));
            }

            let entry_path = entry.path();
            if Self::is_dir_has_supported_image(entry_path)? {
                finder
                    .cur_dir_set
                    .push(entry_path.to_string_lossy().into_owned());
            }
        }

        finder.cur_dir_set.sort();
        finder.set_cur_dir(&search_dir_str);

        if canonicalized_path.is_file() {
            finder.set_cur_image(&canonicalized_path.to_string_lossy());
        }

        Ok(finder)
    }
    pub fn consume_dir_changed_flag(&mut self) -> bool {
        let mut flag = false;
        std::mem::swap(&mut flag, &mut self.dir_changed);
        flag
    }

    pub fn peek_dir_changed_flag(&self) -> bool {
        self.dir_changed
    }

    pub fn set_cur_image(&mut self, image_name: &str) {
        if let Ok(image) = self.cur_image_set.search(image_name) {
            self.cur_image = Some(image);
        }
    }

    pub fn set_cur_image_idx(&mut self, image: usize) {
        if image < self.cur_image_set.0.len() {
            self.cur_image = Some(image);
        }
    }

    pub fn cur_image(&self) -> Option<usize> {
        self.cur_image
    }

    pub fn cur_image_name(&self) -> Option<&str> {
        if let Some(image) = self.cur_image {
            return Some(&self.cur_image_set.0[image]);
        }

        None
    }

    pub fn image_iter(&self) -> Iter<'_, String> {
        self.cur_image_set.iter()
    }

    pub fn image_at(&self, idx: usize) -> Option<&String> {
        self.cur_image_set.0.get(idx)
    }

    pub fn cur_image_set(&self) -> &NatOrdSet {
        &self.cur_image_set
    }

    pub fn next_image(&mut self) {
        if let Some(image) = self.cur_image {
            if image + 1 < self.cur_image_set.0.len() {
                self.cur_image = Some(image + 1)
            }
            return;
        }

        if !self.cur_image_set.0.is_empty() {
            self.cur_image = Some(0);
        }
    }

    pub fn prev_image(&mut self) {
        if let Some(image) = self.cur_image {
            self.cur_image = Some(image.saturating_sub(1));
            return;
        }

        if !self.cur_image_set.0.is_empty() {
            self.cur_image = Some(0);
        }
    }

    pub fn set_cur_dir(&mut self, dir_name: &str) {
        if let Ok(dir) = self.cur_dir_set.search(dir_name) {
            self.set_cur_dir_idx(dir);
        }
    }

    pub fn set_cur_dir_idx(&mut self, dir: usize) {
        if self.cur_dir != Some(dir) && dir < self.cur_dir_set.0.len() {
            self.cur_dir = Some(dir);
            self.cur_image = None;
            self.cur_image_set.0.clear();
            self.dir_changed = true;

            let dir_path = &self.cur_dir_set.0[dir];
            match std::fs::read_dir(dir_path) {
                Ok(dir_items) => {
                    for item in dir_items {
                        if item.is_err() {
                            log::warn!("read dir item fails: {}", item.err().unwrap());
                            continue;
                        }

                        let item = item.unwrap().path();
                        if item.is_file()
                            && item
                                .extension()
                                .is_some_and(|ext| Self::is_supported_ext(ext.to_str().unwrap()))
                        {
                            self.cur_image_set.push(item.to_string_lossy().into_owned());
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading directory {}: {}", dir_path, e);
                    self.cur_dir = None;
                    self.cur_image = None;
                }
            }

            self.cur_image_set.sort();
            self.next_image();
        }
    }

    pub fn cur_dir(&self) -> Option<usize> {
        self.cur_dir
    }

    pub fn cur_dir_name(&self) -> Option<&str> {
        if let Some(dir) = self.cur_dir {
            return Some(&self.cur_dir_set.0[dir]);
        }

        None
    }

    pub fn cur_dir_set(&self) -> &NatOrdSet {
        &self.cur_dir_set
    }

    pub fn next_dir(&mut self) {
        if let Some(dir) = self.cur_dir {
            if dir + 1 < self.cur_dir_set.0.len() {
                self.set_cur_dir_idx(dir + 1);
            }
            return;
        }

        if !self.cur_dir_set.0.is_empty() {
            self.set_cur_dir_idx(0);
        }
    }

    pub fn prev_dir(&mut self) {
        if let Some(dir) = self.cur_dir {
            self.set_cur_dir_idx(dir.saturating_sub(1));
            return;
        }

        if !self.cur_dir_set.0.is_empty() {
            self.set_cur_dir_idx(0);
        }
    }
}
