use serde::{Deserialize, Serialize};
use swash::{
    scale::{ScaleContext, Scaler},
    FontRef, GlyphMetrics,
};

pub struct FontData {
    // use pointer to deal with self refenrece
    pub data: *mut u8,
    pub scale_ctx: *mut ScaleContext,
    pub font_ref: FontRef<'static>,
    pub scaler: Scaler<'static>,
    pub metrics: GlyphMetrics<'static>,
}

impl FontData {
    pub fn new(font_path: &str, font_size: f32) -> Option<Self> {
        let data = std::fs::read(font_path).ok()?;

        let scale_ctx = std::ptr::from_mut(Box::leak(Box::new(ScaleContext::new())));

        let data = Box::leak(data.into_boxed_slice());
        let data_ptr = data.as_mut_ptr();
        let font_ref = swash::FontRef::from_index(data, 0)?;
        let metrics = font_ref.glyph_metrics(&[]).scale(font_size);

        let scaler = (unsafe { &mut *scale_ctx })
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .build();

        Some(Self {
            data: data_ptr,
            scale_ctx,
            font_ref,
            scaler,
            metrics,
        })
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.scaler = (unsafe { &mut *self.scale_ctx })
            .builder(self.font_ref)
            .size(font_size)
            .hint(true)
            .build();
        self.metrics = self.font_ref.glyph_metrics(&[]).scale(font_size);
    }
}

impl Drop for FontData {
    fn drop(&mut self) {
        std::mem::drop(unsafe { Box::from_raw(self.data) });
        std::mem::drop(unsafe { Box::from_raw(self.scale_ctx) });
    }
}

#[derive(Default)]
pub struct FontDef(pub Vec<(String, Option<FontData>)>);

impl Serialize for FontDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|(key, _)| key))
    }
}

impl<'de> Deserialize<'de> for FontDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct ElmVisitor;

        impl<'de> de::Visitor<'de> for ElmVisitor {
            type Value = FontDef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut map = FontDef(Vec::new());
                while let Some(item) = seq.next_element()? {
                    map.0.push((item, None))
                }

                Ok(map)
            }
        }

        deserializer.deserialize_seq(ElmVisitor)
    }
}
