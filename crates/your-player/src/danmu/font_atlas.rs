use crate::danmu::font_def::{FontData, FontDef};
use eframe::{egui::ahash::HashMap, epaint::TextureAtlas};
use serde::{Deserialize, Serialize};
use swash::{
    scale::{Render, Source},
    zeno::Style,
};

#[derive(Debug)]
pub struct Glyph {
    pub size: [u32; 2],
    pub advance: f32,
    pub left: i32,
    pub top: i32,
    pub uv_min: [u16; 2],
    pub uv_max: [u16; 2],
}

#[derive(Debug)]
pub struct StrokedGlyph {
    pub stroke: Glyph,
    pub glyph: Glyph,
}

#[derive(Serialize, Deserialize)]
pub struct FontAtlas {
    #[serde(skip)]
    glyphs: HashMap<char, Option<StrokedGlyph>>,
    #[serde(skip)]
    fallback: Option<StrokedGlyph>,
    #[serde(skip)]
    atlas: Option<TextureAtlas>,
    #[serde(skip)]
    need_recrate_atlas: bool,
    #[serde(skip)]
    renderer: Option<Render<'static>>,

    fonts: FontDef,
    font_size: f32,
    stroke_size: f32,
}

impl Default for FontAtlas {
    fn default() -> Self {
        Self {
            glyphs: Default::default(),
            fallback: Default::default(),
            atlas: Default::default(),
            need_recrate_atlas: Default::default(),
            renderer: Default::default(),
            fonts: Default::default(),
            font_size: 28.0,
            stroke_size: 1.0,
        }
    }
}

impl FontAtlas {
    pub fn get_glyphs_into<'this>(
        &'this mut self,
        text: &str,
        list: &mut Vec<&'this StrokedGlyph>,
    ) {
        assert!(
            !self.need_recrate_atlas,
            "Do not change font and stroke size before get glyphs"
        );

        if self.fallback.is_none() {
            self.fallback = self.render_glyph_to_atlas(None);
        }

        if self.fallback.is_none() {
            return;
        }

        for char in text.chars() {
            if self.glyphs.get(&char).is_some_and(|data| data.is_some()) {
                continue;
            }

            let glyph = self.render_glyph_to_atlas(Some(char));
            self.glyphs.insert(char, glyph);
        }

        for char in text.chars() {
            let opt_glyph = self.glyphs.get(&char).unwrap();
            if let Some(glyph) = opt_glyph {
                list.push(glyph);
            } else {
                list.push(self.fallback.as_ref().unwrap());
            }
        }
    }

    fn render_glyph_to_atlas(&mut self, char: Option<char>) -> Option<StrokedGlyph> {
        debug_assert!(
            char.is_none()
                || char.is_some_and(|c| !self.glyphs.get(&c).is_some_and(|data| data.is_some()))
        );

        for (_, opt_font_data) in &mut self.fonts.0 {
            if let Some(font) = opt_font_data {
                let glyph_id = match char {
                    Some(c) => match font.font_ref.charmap().map(c) {
                        0 => continue,
                        id => id,
                    },
                    None => 0,
                };

                macro_rules! make_glyph {
                    ($name:ident, $style:expr) => {
                        let img = self
                            .renderer
                            .as_mut()
                            .unwrap()
                            .style($style)
                            .render(&mut font.scaler, glyph_id)
                            .unwrap();

                        let (pos, image) = self
                            .atlas
                            .as_mut()
                            .unwrap()
                            .allocate((img.placement.width as _, img.placement.height as _));

                        for y in 0..img.placement.height {
                            for x in 0..img.placement.width {
                                let v = img.data[(y * img.placement.width + x) as usize];
                                if v == 0 {
                                    continue;
                                }
                                let px = pos.0 + x as usize;
                                let py = pos.1 + y as usize;
                                image[(px, py)] = v as f32 / std::u8::MAX as f32;
                            }
                        }

                        let $name = Glyph {
                            size: [img.placement.width, img.placement.height],
                            advance: font.metrics.advance_width(glyph_id),
                            left: img.placement.left,
                            top: img.placement.top,
                            uv_min: [pos.0 as _, pos.1 as _],
                            uv_max: [
                                (pos.0 + img.placement.width as usize) as _,
                                (pos.1 + img.placement.height as usize) as _,
                            ],
                        };
                    };
                }

                make_glyph!(glyph, Style::default());

                make_glyph!(
                    stroke,
                    Style::Stroke(swash::zeno::Stroke::new(self.stroke_size))
                );

                return Some(StrokedGlyph { stroke, glyph });
            }
        }

        None
    }

    pub fn atlas(&self) -> &TextureAtlas {
        self.atlas.as_ref().unwrap()
    }

    pub fn atlas_mut(&mut self) -> &mut TextureAtlas {
        self.atlas.as_mut().unwrap()
    }

    pub fn recrate_atlas(&mut self, max_texture_size: usize) {
        self.need_recrate_atlas = false;
        let height = (self.font_size + self.stroke_size * 2.0).ceil() as _;
        self.atlas = Some(TextureAtlas::new([max_texture_size, height]));
        self.glyphs.clear();
        self.fallback = None;
    }

    /// only call this when first initialize
    pub fn setup(&mut self, max_texture_size: usize) {
        pub const RENDER_SOURCE: [Source; 1] = [Source::Outline];

        self.recrate_atlas(max_texture_size);
        self.renderer = Some(Render::new(&RENDER_SOURCE));

        for (path, file) in &mut self.fonts.0 {
            assert!(file.is_none());
            *file = FontData::new(path, self.font_size);
            if file.is_none() {
                log::error!("load font '{path}' fails");
            }
        }
    }

    pub fn add_font(&mut self, font_path: &str) {
        if self
            .fonts
            .0
            .iter()
            .find(|(key, _)| key == font_path)
            .is_some()
        {
            return;
        }

        let content = FontData::new(font_path, self.font_size);
        if content.is_none() {
            log::error!("load font '{font_path}' fails");
        }

        self.fonts.0.push((font_path.to_owned(), content));
    }

    pub fn clear_fonts(&mut self) {
        self.fonts.0.clear();
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn stroke_size(&self) -> f32 {
        self.stroke_size
    }

    pub fn fonts(&self) -> &FontDef {
        &self.fonts
    }

    pub fn set_font_size(&mut self, font_size: f32) -> bool {
        assert!(font_size >= 16.0 && font_size <= 32.0);
        if font_size == self.font_size {
            return false;
        }
        self.font_size = font_size;
        self.fonts.0.iter_mut().for_each(|(_, v)| {
            if let Some(data) = v {
                data.set_font_size(font_size);
            }
        });
        self.need_recrate_atlas = true;
        return true;
    }

    pub fn set_stroke_size(&mut self, stroke_size: f32) -> bool {
        assert!(stroke_size > 0.0 && stroke_size <= 4.0);
        if stroke_size == self.stroke_size {
            return false;
        }
        self.stroke_size = stroke_size;
        self.need_recrate_atlas = true;
        return true;
    }

    pub fn need_recrate_atlas(&self) -> bool {
        self.need_recrate_atlas || self.atlas().fill_ratio() >= 0.8
    }
}
