use eframe::{
    egui::{self, ahash::HashMap},
    glow,
};

/// Do not use glow function delete any `glow::Texture` in this struct,
/// otherwise result in UB!
#[derive(Default, Debug)]
pub struct TexRegister {
    pub map: HashMap<glow::Texture, Option<egui::TextureId>>,
    pub pending: Vec<glow::Texture>,
}

impl TexRegister {
    /// Get egui texture id related to glow texture
    /// if not found, return `None` and pending the glow texture
    pub fn get(&mut self, tex: glow::Texture) -> Option<egui::TextureId> {
        if let Some(egui_tex_opt) = self.map.get(&tex) {
            return egui_tex_opt.clone();
        }

        self.map.insert(tex, None);
        self.pending.push(tex);

        None
    }

    /// This function should be called after all place that call `get`
    pub fn register_native_tex_if_any(&mut self, frame: &mut eframe::Frame) {
        if !self.pending.is_empty() {
            self.pending
                .iter()
                .map(|tex| (*tex, frame.register_native_glow_texture(*tex)))
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|(tex, egui_tex)| {
                    self.map.insert(tex, Some(egui_tex));
                });
            self.pending.clear();
        }
    }
}
