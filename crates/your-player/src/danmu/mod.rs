use eframe::{
    egui::{self, pos2, vec2},
    glow::{self, HasContext},
};
use font_atlas::FontAtlas;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    ops::Range,
};

pub mod font_atlas;
pub mod font_def;

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
    Corrupt,
}

#[derive(Debug)]
pub struct DanmuEmittedData {
    pub rect: eframe::egui::Rect,

    /// the distance from rect.top(), used for put glphy
    pub baseline: i32,

    /// for `DanmuType::Top` and `DanmuType::Bottom`
    pub lifetime: f64,

    /// for `DanmuType::Rolling`
    pub speed: f32,

    pub state: DanmuEmittedDataState,
}

impl Default for DanmuEmittedData {
    fn default() -> Self {
        Self {
            rect: egui::Rect::ZERO,
            baseline: 0,
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

pub struct Manager {
    danmu: Vec<DanmuData>,
    emitted: HashSet<*mut DanmuData>,

    /// centered danmu pending list
    centered_pending: VecDeque<*mut DanmuData>,

    /// rolling danmu pending list
    rolling_pending: VecDeque<*mut DanmuData>,

    /// centered emitted danmu map. used for `try_emit_pending_danmu`.
    ///
    /// `key`: current emitted danmu top (distance from `rect::top()`)
    ///
    /// `value`: (current danmu height)
    centered_emitted_map: BTreeMap<NotNan<f32>, f32>,

    /// rolling emitted danmu map. used for `try_emit_pending_danmu`.
    ///
    /// `key`: current emitted danmu top (distance from `rect::top()`)
    ///
    /// `value`: (current danmu height, current danmu pointer)
    rolling_emitted_map: BTreeMap<NotNan<f32>, (f32, *mut DanmuData)>,

    tex: glow::Texture,
    state: State,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub atlas: FontAtlas,

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
            atlas: Default::default(),
            rolling_speed: 180.0,
            lifetime: 5.0,
            lower_bound: 0.5,
            delay: 0.0,
            alpha: 240,
        }
    }
}

impl Manager {
    pub fn new(
        mut state: State,
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        state.atlas.setup(cc.egui_ctx.input(|i| i.max_texture_side));
        Ok(Self {
            danmu: Vec::new(),
            emitted: HashSet::new(),
            centered_pending: VecDeque::new(),
            rolling_pending: VecDeque::new(),
            centered_emitted_map: BTreeMap::new(),
            rolling_emitted_map: BTreeMap::new(),
            tex: unsafe { crate::mpv::get_texture(cc.gl.as_ref().unwrap())? },
            state,
        })
    }

    pub fn load_danmu(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut danmu = Vec::new();

        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;

        if let None = || -> Option<()> {
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
        }() {
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

    pub fn update(&mut self, ui: &mut egui::Ui, gl: &glow::Context) {
        if self.state.atlas.need_recrate_atlas() {
            self.state
                .atlas
                .recrate_atlas(ui.ctx().input(|i| i.max_texture_side));
        }

        if let Some(image) = self.state.atlas.atlas_mut().take_delta() {
            let (data, [w, h]) = match &image.image {
                egui::ImageData::Font(data) => (
                    data.srgba_pixels(None)
                        .flat_map(|a| a.to_array())
                        .collect::<Vec<u8>>(),
                    data.size,
                ),
                egui::ImageData::Color(_) => unreachable!(),
            };

            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));

                let level = 0;
                if let Some([x, y]) = image.pos {
                    gl.tex_sub_image_2d(
                        glow::TEXTURE_2D,
                        level,
                        x as _,
                        y as _,
                        w as _,
                        h as _,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(&data),
                    );
                    eframe::egui_glow::check_for_gl_error!(gl);
                } else {
                    let border = 0;
                    gl.tex_image_2d(
                        glow::TEXTURE_2D,
                        level,
                        glow::SRGB8_ALPHA8 as _,
                        w as _,
                        h as _,
                        border,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        Some(&data),
                    );
                    eframe::egui_glow::check_for_gl_error!(gl);
                }

                gl.bind_texture(glow::TEXTURE_2D, None);
            }
        }
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

            let ptr = std::ptr::from_mut(&mut self.danmu[i]);
            match self.danmu[i].danmu_type {
                DanmuType::Rolling => self.rolling_pending.push_back(ptr),
                DanmuType::Top | DanmuType::Bottom => self.centered_pending.push_back(ptr),
            }
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        tex: egui::TextureId,
        mut rect: egui::Rect,
        elapsed_time: f64,
    ) {
        rect.set_bottom(rect.top() + rect.height() * self.state.lower_bound);
        self.try_emit_pending_danmu(rect);

        if self.emitted.is_empty() {
            return;
        }

        let mut remove = Vec::new();
        let mut stroked_mesh = eframe::egui::Mesh::with_texture(tex);
        let mut mesh = eframe::egui::Mesh::with_texture(tex);

        for danmu in &self.emitted {
            let danmu_ref = unsafe { &mut **danmu };

            let emitted = danmu_ref.emitted_data.as_mut().unwrap();
            match danmu_ref.danmu_type {
                DanmuType::Rolling => {
                    emitted.rect = emitted
                        .rect
                        .translate(vec2(-emitted.speed * elapsed_time as f32, 0.0));
                    if emitted.rect.right() < rect.left()
                        || emitted.rect.left() > rect.right() + emitted.rect.width()
                    {
                        remove.push(*danmu);
                    }
                }
                DanmuType::Top | DanmuType::Bottom => {
                    emitted.rect = egui::Rect::from_center_size(
                        pos2(rect.center().x, emitted.rect.center().y),
                        emitted.rect.size(),
                    );
                    emitted.lifetime = (emitted.lifetime - elapsed_time).min(self.state.lifetime);
                    if emitted.lifetime <= 0.0 {
                        remove.push(*danmu);
                    }
                }
            }

            if !remove.contains(&danmu) {
                let mut glyphs = Vec::new();
                let [tex_w, tex_h] = self.state.atlas.atlas().size();

                self.state
                    .atlas
                    .get_glyphs_into(&danmu_ref.text, &mut glyphs);

                let mut px = emitted.rect.left().round() as i32 - glyphs[0].stroke.left;
                let py = emitted.rect.top().round() as i32 + emitted.baseline;

                macro_rules! add_glyph {
                    ($expr:expr, $mesh:ident,$color:expr) => {
                        let rect = egui::Rect::from_min_size(
                            pos2((px + $expr.left) as _, (py - $expr.top) as _),
                            [$expr.size[0] as _, $expr.size[1] as _].into(),
                        );
                        let uv = egui::Rect::from_min_max(
                            pos2(
                                $expr.uv_min[0] as f32 / tex_w as f32,
                                $expr.uv_min[1] as f32 / tex_h as f32,
                            ),
                            pos2(
                                $expr.uv_max[0] as f32 / tex_w as f32,
                                $expr.uv_max[1] as f32 / tex_h as f32,
                            ),
                        );
                        $mesh.add_rect_with_uv(rect, uv, $color);
                    };
                }

                let white = egui::Color32::from_rgba_unmultiplied(255, 255, 255, self.state.alpha);
                let black = egui::Color32::from_rgba_unmultiplied(0, 0, 0, self.state.alpha);

                for glyph in &glyphs {
                    add_glyph!(
                        glyph.stroke,
                        stroked_mesh,
                        if (danmu_ref.color.0 as u32
                            + danmu_ref.color.1 as u32
                            + danmu_ref.color.2 as u32)
                            >= 150
                        {
                            black
                        } else {
                            white
                        }
                    );
                    add_glyph!(
                        glyph.glyph,
                        mesh,
                        egui::Color32::from_rgba_unmultiplied(
                            danmu_ref.color.0,
                            danmu_ref.color.1,
                            danmu_ref.color.2,
                            self.state.alpha
                        )
                    );
                    px += glyph.stroke.advance.round() as i32;
                }
            }
        }

        for item in remove {
            let danmu_ref = unsafe { &mut *item };
            let danmu_emitted = danmu_ref.emitted_data.as_ref().unwrap();
            let danmu_top = NotNan::new(danmu_emitted.rect.top()).unwrap();
            match danmu_ref.danmu_type {
                DanmuType::Rolling => {
                    if let Some(&(_, ptr)) = self.rolling_emitted_map.get(&danmu_top) {
                        if ptr == item {
                            self.rolling_emitted_map.remove(&danmu_top);
                        }
                    }
                }
                DanmuType::Top | DanmuType::Bottom => {
                    self.centered_emitted_map.remove(&danmu_top);
                }
            }
            danmu_ref.emitted_data = None;
            self.emitted.remove(&item);
        }
        ui.painter().add(stroked_mesh);
        ui.painter().add(mesh);
    }

    fn try_emit_pending_danmu(&mut self, rect: egui::Rect) {
        let danmu_not_initlize = |v: &*mut DanmuData| {
            unsafe { &mut **v }.emitted_data.as_ref().unwrap().state
                == DanmuEmittedDataState::NotInit
        };
        let list: Vec<_> = self
            .centered_pending
            .iter()
            .rev()
            .map(|v| *v)
            .take_while(danmu_not_initlize)
            .chain(
                self.rolling_pending
                    .iter()
                    .rev()
                    .map(|v| *v)
                    .take_while(danmu_not_initlize),
            )
            .collect();

        for ptr in list.iter() {
            self.calculate_size(*ptr);
        }

        while let Some(danmu) = self.centered_pending.pop_front() {
            let danmu_ref = unsafe { &mut *danmu };
            let danmu_emitted = danmu_ref.emitted_data.as_mut().unwrap();

            if danmu_emitted.state == DanmuEmittedDataState::Corrupt {
                continue;
            }

            let mut danmu_top: Option<f32> = None;
            let danmu_height = danmu_emitted.rect.height();

            match danmu_ref.danmu_type {
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

            if danmu_emitted.state == DanmuEmittedDataState::Corrupt {
                continue;
            }

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

    fn calculate_size(&mut self, ptr: *mut DanmuData) {
        let danmu_ref = unsafe { &mut *ptr };

        let mut glyphs = Vec::new();
        self.state
            .atlas
            .get_glyphs_into(&danmu_ref.text, &mut glyphs);

        if !glyphs.is_empty() {
            let mut width = -glyphs[0].stroke.left;
            let mut baseline = 0;
            for glyph in &glyphs {
                width += glyph.stroke.advance.round() as i32;
                baseline = baseline.max(glyph.stroke.top);
            }
            // important: it's necessary to keep all danmus have the same height
            // to ensure emit properly
            let height =
                (self.state.atlas.font_size() + self.state.atlas.stroke_size() * 2.0).ceil() as i32;

            danmu_ref.emitted_data = Some(DanmuEmittedData {
                rect: egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(width as _, height as _)),
                baseline,
                lifetime: self.state.lifetime,
                speed: (self.state.rolling_speed * width as f32 / 160.0).clamp(
                    self.state.rolling_speed * 0.75,
                    self.state.rolling_speed * 1.25,
                ),
                state: DanmuEmittedDataState::Ready,
            })
        } else {
            danmu_ref.emitted_data.as_mut().unwrap().state = DanmuEmittedDataState::Corrupt;
        }
    }

    pub fn texture(&self) -> &glow::Texture {
        &self.tex
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
