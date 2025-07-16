use eframe::egui::{self, pos2};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    ops::Range,
    ptr::NonNull,
};

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

#[derive(PartialEq, Default, Debug)]
pub enum DanmuEmittedDataState {
    #[default]
    NotInit,
    Ready,
}

#[derive(Debug)]
pub struct DanmuEmittedData {
    pub rect: eframe::egui::Rect,

    /// for [`DanmuType::Top`] and [`DanmuType::Bottom`]
    pub lifetime: f64,

    /// for [`DanmuType::Rolling`]
    pub speed: f32,

    pub state: DanmuEmittedDataState,
}

impl Default for DanmuEmittedData {
    fn default() -> Self {
        Self {
            rect: egui::Rect::ZERO,
            lifetime: 0.0,
            speed: 0.0,
            state: DanmuEmittedDataState::NotInit,
        }
    }
}

#[derive(Debug)]
pub enum DanmuType {
    Rolling = 0,
    Top = 1,
    Bottom = 2,
}

type DanmuPtr = NonNull<DanmuData>;

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
}

impl Default for State {
    fn default() -> Self {
        Self {
            rolling_speed: 180.0,
            lifetime: 5.0,
            lower_bound: 0.5,
            delay: 0.0,
            alpha: 240,
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

        self.clear_emitted();
        self.danmu = danmu;

        Ok(())
    }

    pub fn delay_danmu(&mut self, delay: f64) {
        self.state.delay = delay;
        self.danmu
            .iter_mut()
            .for_each(|d| d.playback_time = (d.playback_time_raw + self.state.delay).max(0.0));
    }

    pub fn emit(&mut self, range: Range<f64>, regex: Option<&regex::Regex>) {
        let start = match self
            .danmu
            .binary_search_by(|t| t.playback_time.partial_cmp(&range.start).unwrap())
        {
            Ok(v) => v,
            Err(v) => v,
        };
        assert!(!range.is_empty());

        if start == self.danmu.len() {
            return;
        }

        for i in start..self.danmu.len() {
            if self.danmu[i].playback_time >= range.end {
                return;
            }

            if self.danmu[i].emitted_data.is_some() {
                continue;
            }

            if let Some(reg) = regex {
                if reg.is_match(&self.danmu[i].text) {
                    continue;
                }
            }

            self.danmu[i].emitted_data = Some(DanmuEmittedData::default());

            let ptr = NonNull::from(&mut self.danmu[i]);
            match self.danmu[i].danmu_type {
                DanmuType::Rolling => self.rolling_pending.push_back(ptr),
                DanmuType::Top | DanmuType::Bottom => self.centered_pending.push_back(ptr),
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, mut rect: egui::Rect, elapsed_time: f64) {
        // 限制弹幕显示区高度
        rect.set_bottom(rect.top() + rect.height() * self.state.lower_bound);
        self.try_emit_pending_danmu(ui, rect);

        if self.emitted.is_empty() {
            return;
        }

        let mut remove = HashSet::new();
        let painter = ui.painter();

        // 准备对比色
        let white = egui::Color32::from_rgba_unmultiplied(255, 255, 255, self.state.alpha / 2);
        let black = egui::Color32::from_rgba_unmultiplied(0, 0, 0, self.state.alpha / 2);

        for &(mut ptr) in &self.emitted {
            let danmu = unsafe { ptr.as_mut() };
            let emitted = danmu.emitted_data.as_mut().unwrap();

            // 更新位置和状态
            match danmu.danmu_type {
                DanmuType::Rolling => {
                    emitted.rect = emitted
                        .rect
                        .translate(egui::vec2(-emitted.speed * elapsed_time as f32, 0.0));
                    // 如果完全移出左侧
                    if emitted.rect.right() < rect.left() {
                        remove.insert(ptr);
                    }
                }
                DanmuType::Top | DanmuType::Bottom => {
                    // 重新计算居中位置
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

            // 根据文字颜色计算背景对比色
            let (r, g, b) = danmu.color;
            let luminance = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
            let bg_color = if luminance > 90.0 { black } else { white };

            // 绘制圆角矩形背景（egui v0.32 API）
            painter.rect_filled(emitted.rect, 4.0, bg_color);

            // 绘制文字，文字颜色为 danmu_ref.color
            let text_color = egui::Color32::from_rgba_unmultiplied(r, g, b, self.state.alpha);
            let text_pos = emitted.rect.left_top() + egui::vec2(4.0, 2.0);
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                &danmu.text,
                egui::TextStyle::Body.resolve(ui.style()),
                text_color,
            );
        }

        // 清理已移除的弹幕，并更新布局映射
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

    fn try_emit_pending_danmu(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let pending_ptrs: Vec<_> = self
            .centered_pending
            .iter()
            .chain(&self.rolling_pending)
            .filter(|&&ptr| {
                let danmu = unsafe { ptr.as_ref() };
                danmu
                    .emitted_data
                    .as_ref()
                    .map_or(false, |e| e.state == DanmuEmittedDataState::NotInit)
            })
            .copied()
            .collect();

        for ptr in pending_ptrs {
            let danmu = unsafe { ptr.as_mut() };
            Self::calculate_size(&self.state, ui, danmu);
        }

        while let Some(ptr) = self.centered_pending.pop_front() {
            let danmu = unsafe { ptr.as_ref() };
            let danmu_emitted = danmu.emitted_data.as_mut().unwrap();

            let mut danmu_top: Option<f32> = None;
            let danmu_height = danmu_emitted.rect.height();

            match danmu.danmu_type {
                DanmuType::Top => {
                    let mut last_danmu_bottom = rect.top();
                    for (cur_danmu_top, cur_danmu_height) in self.centered_emitted_map.iter() {
                        let cur_danmu_top = **cur_danmu_top;
                        if cur_danmu_top - last_danmu_bottom >= danmu_height {
                            danmu_top = Some(last_danmu_bottom);
                            break;
                        }
                        last_danmu_bottom = cur_danmu_top + cur_danmu_height;
                        if last_danmu_bottom + danmu_height > rect.bottom() {
                            break;
                        }
                    }

                    if danmu_top.is_none() && last_danmu_bottom + danmu_height <= rect.bottom() {
                        danmu_top = Some(last_danmu_bottom);
                    }
                }
                DanmuType::Bottom => {
                    let mut last_danmu_top = rect.bottom();
                    for (cur_danmu_top, cur_danmu_height) in self.centered_emitted_map.iter().rev()
                    {
                        let cur_danmu_top = **cur_danmu_top;
                        if last_danmu_top - (cur_danmu_top + cur_danmu_height) >= danmu_height {
                            danmu_top = Some(last_danmu_top - danmu_height);
                            break;
                        }
                        last_danmu_top = cur_danmu_top;
                        if last_danmu_top - danmu_height < rect.top() {
                            break;
                        }
                    }

                    if danmu_top.is_none() && last_danmu_top - danmu_height >= rect.top() {
                        danmu_top = Some(last_danmu_top - danmu_height);
                    }
                }
                DanmuType::Rolling => unreachable!(),
            }

            if let Some(y) = danmu_top {
                danmu_emitted.rect = egui::Rect::from_min_size(
                    pos2(rect.center().x - danmu_emitted.rect.width() / 2.0, y),
                    danmu_emitted.rect.size(),
                );
                self.emitted.insert(danmu);
                self.centered_emitted_map
                    .insert(NotNan::new(y).unwrap(), danmu_height);
            } else {
                self.centered_pending.push_front(danmu);
                break;
            }
        }

        while let Some(danmu) = self.rolling_pending.pop_front() {
            let danmu_ref = unsafe { &mut *danmu };
            let danmu_emitted = danmu_ref.emitted_data.as_mut().unwrap();

            let mut danmu_top: Option<f32> = None;
            let danmu_height = danmu_emitted.rect.height();

            match danmu_ref.danmu_type {
                DanmuType::Rolling => {
                    let mut last_danmu_bottom = rect.top();
                    for (cur_danmu_top, (cur_danmu_height, cur_danmu)) in
                        self.rolling_emitted_map.iter()
                    {
                        let cur_danmu_top = **cur_danmu_top;
                        if cur_danmu_top - last_danmu_bottom >= danmu_height {
                            danmu_top = Some(last_danmu_bottom);
                            break;
                        }
                        let cur_danmu_ref = unsafe { &mut **cur_danmu };
                        let cur_danmu_emitted = cur_danmu_ref.emitted_data.as_ref().unwrap();

                        let shortened_distance_before_cur_danmu_over = 0.0_f32.max(
                            (danmu_emitted.speed - cur_danmu_emitted.speed)
                                * (cur_danmu_emitted.rect.right() - rect.left())
                                / cur_danmu_emitted.speed,
                        );
                        let cur_distance = rect.right() - cur_danmu_emitted.rect.right();

                        // if the danmu to be emitted does not exceed current danmu
                        // then it can be emitted at same line
                        if cur_distance >= shortened_distance_before_cur_danmu_over {
                            danmu_top = Some(cur_danmu_top);
                            break;
                        }

                        last_danmu_bottom = cur_danmu_top + cur_danmu_height;
                        if last_danmu_bottom + danmu_height > rect.bottom() {
                            break;
                        }
                    }

                    if danmu_top.is_none() && last_danmu_bottom + danmu_height <= rect.bottom() {
                        danmu_top = Some(last_danmu_bottom);
                    }
                }
                _ => unreachable!(),
            }

            if let Some(y) = danmu_top {
                danmu_emitted.rect =
                    egui::Rect::from_min_size(pos2(rect.right(), y), danmu_emitted.rect.size());
                self.emitted.insert(danmu);
                self.rolling_emitted_map
                    .insert(NotNan::new(y).unwrap(), (danmu_height, danmu));
            } else {
                self.rolling_pending.push_front(danmu);
                break;
            }
        }
    }

    fn calculate_size(state: &State, ui: &egui::Ui, ptr: *mut DanmuData) {
        let danmu_ref = unsafe { &mut *ptr };
        // 文本布局，不自动换行
        let text_style = egui::TextStyle::Body.resolve(ui.style());
        let galley = ui
            .fonts(|f| f.layout_no_wrap(danmu_ref.text.clone(), text_style, egui::Color32::WHITE));
        let size = galley.size();
        // 加入内边距: 左右各4px，上下各2px
        let padded_size = egui::vec2(size.x + 8.0, size.y + 4.0);
        // 计算速度，保持与原逻辑大致一致
        let speed = (state.rolling_speed * padded_size.x / 160.0)
            .clamp(state.rolling_speed * 0.75, state.rolling_speed * 1.25);
        danmu_ref.emitted_data = Some(DanmuEmittedData {
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), padded_size),
            lifetime: state.lifetime,
            speed,
            state: DanmuEmittedDataState::Ready,
        });
    }

    pub fn danmu(&self) -> &Vec<DanmuData> {
        &self.danmu
    }

    pub fn emitted(&self) -> &HashSet<*mut DanmuData> {
        &self.emitted
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// this function should be called if font or stroke size or embolden is changed
    pub fn clear_emitted(&mut self) {
        for ptr in self
            .emitted
            .iter()
            .chain(self.rolling_pending.iter())
            .chain(self.centered_pending.iter())
        {
            let danmu_ref = unsafe { &mut **ptr };
            danmu_ref.emitted_data = None;
        }
        self.centered_emitted_map.clear();
        self.rolling_emitted_map.clear();
        self.rolling_pending.clear();
        self.centered_pending.clear();
        self.emitted.clear();
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
