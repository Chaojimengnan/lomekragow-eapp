use eframe::egui::{FontData, FontDefinitions, FontFamily, FontId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct DanmuFontLoader {
    pub font_size: f32,
    font_paths: BTreeSet<String>,

    #[serde(skip)]
    family: FontFamily,
}

impl Default for DanmuFontLoader {
    fn default() -> Self {
        Self {
            font_paths: BTreeSet::new(),
            font_size: 26.0,
            family: FontFamily::Name("danmu".into()),
        }
    }
}

impl DanmuFontLoader {
    pub fn add_font(&mut self, path: impl Into<String>) {
        self.font_paths.insert(path.into());
    }

    pub fn remove_font(&mut self, path: &str) {
        self.font_paths.remove(path);
    }

    pub fn clear(&mut self) {
        self.font_paths.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.font_paths.iter().map(String::as_str)
    }

    pub fn get_font_id(&self) -> FontId {
        FontId::new(self.font_size, self.family.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.font_paths.is_empty()
    }

    pub fn insert_fonts(&mut self, mut fonts: FontDefinitions) -> FontDefinitions {
        let mut font_counter = 0;
        let mut danmu_fonts = vec![];

        for path in &self.font_paths {
            if let Ok(data) = std::fs::read(path) {
                let name = format!("danmu-{font_counter}");
                font_counter += 1;

                fonts
                    .font_data
                    .insert(name.clone(), FontData::from_owned(data).into());
                danmu_fonts.push(name);
            }
        }

        if !danmu_fonts.is_empty() {
            fonts
                .families
                .entry(self.family.clone())
                .or_default()
                .extend(danmu_fonts);
        }

        fonts
    }
}
