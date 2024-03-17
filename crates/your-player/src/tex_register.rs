use eframe::{
    egui::{self, ahash::HashMap},
    glow,
};
use std::sync::{Arc, Mutex};

/// Do not use glow function delete any `glow::Texture` in this struct,
/// otherwise result in UB!
#[derive(Default, Debug)]
pub struct TexRegister(Arc<Mutex<TexRegisterImpl>>);

#[derive(Default, Debug)]
struct TexRegisterImpl {
    pub map: HashMap<glow::Texture, Option<egui::TextureId>>,
    pub pending: Vec<glow::Texture>,
}

impl TexRegister {
    /// Get egui texture id related to glow texture
    /// if not found, return `None` and pending the glow texture
    pub fn get(&self, tex: glow::Texture) -> Option<egui::TextureId> {
        let mut this = self.0.lock().unwrap();
        if let Some(egui_tex_opt) = this.map.get(&tex) {
            return egui_tex_opt.clone();
        }

        this.map.insert(tex, None);
        this.pending.push(tex);

        None
    }

    /// This function should be called after all place that call `get`
    pub fn register_native_tex_if_any(&self, ui: &mut egui::Ui) {
        let this = self.0.lock().unwrap();
        if !this.pending.is_empty() {
            let this = self.0.clone();
            let cb = eframe::egui_glow::CallbackFn::new(move |_info, painter| {
                let mut this = this.lock().unwrap();
                this.pending
                    .iter()
                    .map(|tex| (*tex, painter.register_native_texture(*tex)))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .for_each(|(tex, egui_tex)| {
                        this.map.insert(tex, Some(egui_tex));
                    });
                this.pending.clear();
            });

            ui.painter().add(egui::PaintCallback {
                rect: egui::Rect::EVERYTHING,
                callback: Arc::new(cb),
            });
        }
    }
}
