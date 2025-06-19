use eframe::egui;
use serde::{Deserialize, Serialize};

pub struct ImgTranslation {
    pub scale_old_for_calculate: Option<f32>,
    pub scale: f32,
    pub min_scale: f32,
    pub is_dragging: bool,
    pub drag_start_offset: egui::Vec2,
    pub image_offset: egui::Vec2,
    pub max_offset: egui::Vec2,
    pub image_fit_space_size: bool,
    pub image_exceeds_space: (bool, bool),
}

impl ImgTranslation {
    pub fn reset_translation(&mut self) {
        self.scale = 1.0;
        self.image_offset = egui::Vec2::ZERO;
    }

    pub fn clamp_offset(&self, offset: egui::Vec2) -> egui::Vec2 {
        offset.clamp(-self.max_offset, self.max_offset)
    }

    pub fn image_fully_contained(&self) -> bool {
        !self.image_exceeds_space.0 && !self.image_exceeds_space.1
    }

    pub fn fit_space_if_need(&mut self, mode: InitialScalingMode) {
        self.image_fit_space_size = match mode {
            InitialScalingMode::OriginalSize => false,
            InitialScalingMode::FitToSpace => true,
            InitialScalingMode::SmartFit => true,
        };
    }
}

impl Default for ImgTranslation {
    fn default() -> Self {
        Self {
            scale_old_for_calculate: None,
            scale: 1.0,
            is_dragging: false,
            image_fit_space_size: true,
            image_offset: egui::Vec2::ZERO,
            drag_start_offset: egui::Vec2::ZERO,
            image_exceeds_space: (false, false),
            max_offset: egui::Vec2::ZERO,
            min_scale: 1.0,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Copy, Default)]
pub enum InitialScalingMode {
    /// Display in the original size of the image
    OriginalSize,

    /// The image will automatically adapt to the available space size
    FitToSpace,

    /// When the image is smaller than the available space size,
    /// display it in its original size; otherwise, the behavior is equal to [`ImgScaleMode::Fit`]
    #[default]
    SmartFit,
}

#[derive(Clone, Copy, Debug)]
pub struct LastImageInfo {
    pub average_color: egui::Color32,
    pub rect: egui::Rect,
}
