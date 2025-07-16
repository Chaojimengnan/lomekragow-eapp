pub(crate) mod emit;
pub(crate) mod font_loader;

use eframe::egui::{self};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    ptr::NonNull,
    sync::Arc,
};

use crate::danmu::font_loader::DanmuFontLoader;

#[derive(Debug)]
pub struct DanmuData {
    /// how the danmu showing
    pub danmu_type: DanmuType,

    /// real playback time without delay
    pub playback_time_raw: f64,

    /// playback time with delay
    pub playback_time: f64,

    /// danmu text
    pub text: String,

    /// danmu color (rgb)
    pub color: (u8, u8, u8),

    /// the data for emitted danmu
    /// Some means that the danmu has been emitted or pended
    pub emitted_data: Option<DanmuEmittedData>,
}

impl Default for DanmuData {
    fn default() -> Self {
        Self {
            danmu_type: DanmuType::Rolling,
            playback_time_raw: 0.0,
            playback_time: 0.0,
            text: String::new(),
            color: (255, 255, 255),
            emitted_data: None,
        }
    }
}

#[derive(PartialEq, Default, Debug, Clone, Copy)]
pub enum DanmuEmittedDataState {
    #[default]
    NotInit,
    Ready,
}

#[derive(Debug)]
pub struct DanmuEmittedData {
    pub rect: egui::Rect,

    /// for [`DanmuType::Top`] and [`DanmuType::Bottom`]
    pub lifetime: f64,

    /// for [`DanmuType::Rolling`]
    pub speed: f32,

    pub state: DanmuEmittedDataState,
    pub galley: Option<Arc<egui::Galley>>,
}

impl Default for DanmuEmittedData {
    fn default() -> Self {
        Self {
            rect: egui::Rect::ZERO,
            lifetime: 0.0,
            speed: 0.0,
            state: DanmuEmittedDataState::NotInit,
            galley: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DanmuType {
    Rolling = 0,
    Top = 1,
    Bottom = 2,
}

pub type DanmuPtr = NonNull<DanmuData>;

pub struct Manager {
    danmu: Vec<DanmuData>,
    emitted: HashSet<DanmuPtr>,

    /// centered danmu pending list
    centered_pending: VecDeque<DanmuPtr>,

    /// rolling danmu pending list
    rolling_pending: VecDeque<DanmuPtr>,

    /// centered emitted danmu map. used for [`Manager::try_emit_pending_danmu`].
    ///
    /// `key`: current emitted danmu top (distance from `rect::top()`)
    ///
    /// `value`: (current danmu height)
    centered_emitted_map: BTreeMap<NotNan<f32>, f32>,

    /// rolling emitted danmu map. used for [`Manager::try_emit_pending_danmu`].
    ///
    /// `key`: current emitted danmu top (distance from `rect::top()`)
    ///
    /// `value`: (current danmu height, current danmu pointer)
    rolling_emitted_map: BTreeMap<NotNan<f32>, (f32, DanmuPtr)>,

    state: State,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// danmu rolling speed (in px)
    pub rolling_speed: f32,

    /// centered danmu lifetime (in secs)
    pub lifetime: f64,

    /// danmu emit range (0.25 ~ 1.0)
    pub lower_bound: f32,

    /// danmu delay (in secs)
    pub delay: f64,

    /// danmu alpha (0 ~ 255)
    pub alpha: u8,

    /// font loader
    pub font_loader: DanmuFontLoader,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rolling_speed: 180.0,
            lifetime: 5.0,
            lower_bound: 0.5,
            delay: 0.0,
            alpha: 240,
            font_loader: DanmuFontLoader::default(),
        }
    }
}

impl Manager {
    pub fn new(state: State) -> Self {
        Self {
            danmu: Vec::new(),
            emitted: HashSet::new(),
            centered_pending: VecDeque::new(),
            rolling_pending: VecDeque::new(),
            centered_emitted_map: BTreeMap::new(),
            rolling_emitted_map: BTreeMap::new(),
            state,
        }
    }

    pub fn load_danmu(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut danmu = Vec::new();

        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;

        let mut get_danmu = || -> Option<()> {
            for value in json.as_array()? {
                let text = value.get("text")?.as_str()?.to_owned();
                let playback_time_raw = value.get("pos")?.as_str()?.parse().ok()?;
                let danmu_type = || -> Option<DanmuType> {
                    match value.get("layout")?.as_u64()? {
                        0 => Some(DanmuType::Rolling),
                        1 => Some(DanmuType::Top),
                        2 => Some(DanmuType::Bottom),
                        _ => None,
                    }
                }()
                .unwrap_or(DanmuType::Rolling);

                let color = || -> Option<(u8, u8, u8)> {
                    Some(Self::u32_to_rgb(value.get("color")?.as_u64()? as _))
                }()
                .unwrap_or((255, 255, 255));

                danmu.push(DanmuData {
                    text,
                    playback_time_raw,
                    playback_time: (playback_time_raw + self.state.delay).max(0.0),
                    danmu_type,
                    color,
                    ..Default::default()
                });
            }
            Some(())
        };

        if get_danmu().is_none() {
            return Err("fail to load danmu json".into());
        }

        danmu.sort_by(|a, b| {
            a.playback_time_raw
                .partial_cmp(&b.playback_time_raw)
                .unwrap()
        });

        self.clear();
        self.danmu = danmu;

        Ok(())
    }

    pub fn delay_danmu(&mut self, delay: f64) {
        self.state.delay = delay;
        self.danmu
            .iter_mut()
            .for_each(|d| d.playback_time = (d.playback_time_raw + self.state.delay).max(0.0));
    }

    pub fn render(&mut self, ui: &mut egui::Ui, mut rect: egui::Rect, elapsed_time: f64) {
        rect.set_bottom(rect.top() + rect.height() * self.state.lower_bound);
        self.try_emit_pending_danmu(ui, rect);

        if self.emitted.is_empty() {
            return;
        }

        let mut remove = HashSet::new();
        let painter = ui.painter();

        let white = egui::Color32::from_rgba_unmultiplied(255, 255, 255, self.state.alpha / 2);
        let black = egui::Color32::from_rgba_unmultiplied(0, 0, 0, self.state.alpha / 2);

        for &(mut ptr) in &self.emitted {
            let danmu = unsafe { ptr.as_mut() };
            let emitted = danmu.emitted_data.as_mut().unwrap();

            match danmu.danmu_type {
                DanmuType::Rolling => {
                    emitted.rect = emitted
                        .rect
                        .translate(egui::vec2(-emitted.speed * elapsed_time as f32, 0.0));
                    if emitted.rect.right() < rect.left() {
                        remove.insert(ptr);
                    }
                }
                DanmuType::Top | DanmuType::Bottom => {
                    let size = emitted.rect.size();
                    let center = egui::pos2(rect.center().x, emitted.rect.center().y);
                    emitted.rect = egui::Rect::from_center_size(center, size);
                    emitted.lifetime -= elapsed_time;
                    if emitted.lifetime <= 0.0 {
                        remove.insert(ptr);
                    }
                }
            }

            if remove.contains(&ptr) {
                continue;
            }

            let (r, g, b) = danmu.color;
            let luminance = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
            let bg_color = if luminance > 70.0 { black } else { white };

            painter.rect_filled(emitted.rect, 4.0, bg_color);

            let text_color = egui::Color32::from_rgba_unmultiplied(r, g, b, self.state.alpha);
            let text_pos = emitted.rect.left_top() + egui::vec2(4.0, 2.0);

            if let Some(galley) = &emitted.galley {
                painter.galley(text_pos, galley.clone(), text_color);
            }
        }

        for mut ptr in remove {
            let danmu = unsafe { ptr.as_mut() };
            if let Some(emitted) = danmu.emitted_data.as_ref() {
                let top_key = NotNan::new(emitted.rect.top()).unwrap();
                match danmu.danmu_type {
                    DanmuType::Rolling => {
                        if let Some(&(_, p)) = self.rolling_emitted_map.get(&top_key) {
                            if p == ptr {
                                self.rolling_emitted_map.remove(&top_key);
                            }
                        }
                    }
                    DanmuType::Top | DanmuType::Bottom => {
                        self.centered_emitted_map.remove(&top_key);
                    }
                }
            }
            danmu.emitted_data = None;
            self.emitted.remove(&ptr);
        }
    }

    pub fn danmu(&self) -> &Vec<DanmuData> {
        &self.danmu
    }

    pub fn emitted(&self) -> &HashSet<DanmuPtr> {
        &self.emitted
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn clear(&mut self) {
        self.centered_emitted_map.clear();
        self.rolling_emitted_map.clear();
        self.rolling_pending.clear();
        self.centered_pending.clear();
        self.emitted.clear();
        self.danmu.clear();
    }

    fn u32_to_rgb(color: u32) -> (u8, u8, u8) {
        let r = ((color >> 16) & 255) as u8;
        let g = ((color >> 8) & 255) as u8;
        let b = (color & 255) as u8;
        (r, g, b)
    }
}
