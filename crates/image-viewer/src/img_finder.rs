use std::{
    collections::{btree_set::Range, BTreeSet},
    ops::Bound,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Default, Clone, Debug)]
pub struct ImgFinder {
    search_dir: Option<String>,
    cur_image: Option<String>,
    cur_dir: Option<String>,
    cur_image_set: BTreeSet<String>,
    cur_dir_set: BTreeSet<String>,
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
                    .is_some_and(|ext| Self::is_supported_ext(ext.to_str().unwrap()))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn search_dir(&self) -> Option<&String> {
        self.search_dir.as_ref()
    }

    pub fn search_from_new_image(mut self, new_image_path: &str) -> std::io::Result<Self> {
        let path = PathBuf::from(new_image_path);
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|ext| Self::is_supported_ext(ext.to_str().unwrap()))
        {
            return Ok(self);
        }

        let dir = path.parent().unwrap();
        let dir_str = dir.to_str().unwrap();
        if !self.cur_dir_set.contains(dir_str) {
            self = Self::default();
            self.search_dir = Some(dir.parent().unwrap_or(dir).to_string_lossy().into_owned());
            if let Some(dir_parent) = dir.parent() {
                for item in WalkDir::new(dir_parent) {
                    let item = item?;
                    let item_path = item.path();
                    if item_path.is_dir() && Self::is_dir_has_supported_image(item_path)? {
                        self.cur_dir_set
                            .insert(item_path.to_string_lossy().into_owned());
                    }
                }
            } else {
                self.cur_dir_set.insert(dir_str.to_owned());
            }
        }

        self.set_cur_dir(dir_str);
        self.set_cur_image(new_image_path);

        Ok(self)
    }

    pub fn consume_dir_changed_flag(&mut self) -> bool {
        let mut flag = false;
        std::mem::swap(&mut flag, &mut self.dir_changed);
        flag
    }

    pub fn peek_dir_changed_flag(&self) -> bool {
        self.dir_changed
    }

    pub fn set_cur_image(&mut self, image: &str) {
        if self.cur_image_set.contains(image) {
            self.cur_image = Some(image.to_owned());
        }
    }

    pub fn cur_image(&self) -> Option<&String> {
        self.cur_image.as_ref()
    }

    pub fn cur_image_set(&self) -> &BTreeSet<String> {
        &self.cur_image_set
    }

    pub fn next_image(&mut self) -> Option<Range<'_, String>> {
        if let Some(image) = self.cur_image.as_ref() {
            let mut range = self
                .cur_image_set
                .range((Bound::Excluded(image.to_owned()), Bound::Unbounded));
            if let Some(next_image) = range.next() {
                self.cur_image = Some(next_image.to_owned());
                return Some(range);
            }

            return None;
        }

        self.cur_image = self.cur_image_set.first().cloned();
        None
    }

    pub fn prev_image(&mut self) -> Option<Range<'_, String>> {
        if let Some(image) = self.cur_image.as_ref() {
            let mut range = self
                .cur_image_set
                .range((Bound::Unbounded, Bound::Excluded(image.to_owned())));
            if let Some(prev_image) = range.next_back() {
                self.cur_image = Some(prev_image.to_owned());
                return Some(range);
            }

            return None;
        }

        self.cur_image = self.cur_image_set.first().cloned();
        None
    }

    pub fn set_cur_dir(&mut self, dir: &str) {
        let dir = dir.to_owned();
        if self.cur_dir_set.contains(&dir) && self.cur_dir.as_ref() != Some(&dir) {
            self.cur_dir = Some(dir.to_owned());
            self.cur_image = None;
            self.cur_image_set.clear();
            self.dir_changed = true;

            if let Ok(dir_items) = std::fs::read_dir(&dir) {
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
                        self.cur_image_set
                            .insert(item.to_string_lossy().into_owned());
                    }
                }
            }

            self.next_image();
        }
    }

    pub fn cur_dir(&self) -> Option<&String> {
        self.cur_dir.as_ref()
    }

    pub fn cur_dir_set(&self) -> &BTreeSet<String> {
        &self.cur_dir_set
    }

    pub fn next_dir(&mut self) {
        if let Some(dir) = self.cur_dir.as_ref() {
            if let Some(next_dir) = self
                .cur_dir_set
                .range((Bound::Excluded(dir.to_owned()), Bound::Unbounded))
                .next()
            {
                self.set_cur_dir(&next_dir.to_owned());
            }
        }
    }

    pub fn prev_dir(&mut self) {
        if let Some(dir) = self.cur_dir.as_ref() {
            if let Some(prev_dir) = self
                .cur_dir_set
                .range((Bound::Unbounded, Bound::Excluded(dir.to_owned())))
                .next_back()
            {
                self.set_cur_dir(&prev_dir.to_owned());
            }
        }
    }
}
