use eframe::egui::{self, Rect, pos2};
use ordered_float::NotNan;
use std::{ops::Range, ptr::NonNull};

use crate::danmu::{
    DanmuData, DanmuEmittedData, DanmuEmittedDataState, DanmuPtr, DanmuType, Manager, State,
};

impl Manager {
    pub fn push_pending(&mut self, range: Range<f64>, regex: Option<&regex::Regex>) {
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

            if let Some(reg) = regex
                && reg.is_match(&self.danmu[i].text)
            {
                continue;
            }

            self.danmu[i].emitted_data = Some(DanmuEmittedData::default());

            let ptr = NonNull::from(&mut self.danmu[i]);
            match self.danmu[i].danmu_type {
                DanmuType::Rolling => self.rolling_pending.push_back(ptr),
                DanmuType::Top | DanmuType::Bottom => self.centered_pending.push_back(ptr),
            }
        }
    }

    pub fn try_emit_pending_danmu(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let pending_ptrs: Vec<_> = self
            .centered_pending
            .iter()
            .chain(&self.rolling_pending)
            .filter(|&&ptr| {
                let danmu = unsafe { ptr.as_ref() };
                danmu
                    .emitted_data
                    .as_ref()
                    .is_some_and(|e| e.state == DanmuEmittedDataState::NotInit)
            })
            .copied()
            .collect();

        for mut ptr in pending_ptrs {
            let danmu = unsafe { ptr.as_mut() };
            Self::measure_text(&self.state, ui, danmu);
        }

        while let Some(mut ptr) = self.centered_pending.pop_front() {
            let danmu = unsafe { ptr.as_mut() };
            let emitted = danmu.emitted_data.as_ref().unwrap();

            if let Some(y) =
                self.find_centered_position(danmu.danmu_type, emitted.rect.height(), rect)
            {
                let danmu_mut = unsafe { ptr.as_mut() };
                if let Some(emitted) = danmu_mut.emitted_data.as_mut() {
                    emitted.rect = Rect::from_min_size(
                        pos2(rect.center().x - emitted.rect.width() / 2.0, y),
                        emitted.rect.size(),
                    );

                    self.centered_emitted_map
                        .insert(NotNan::new(y).unwrap(), emitted.rect.height());
                    self.emitted.insert(ptr);
                }
            } else {
                self.centered_pending.push_front(ptr);
                break;
            }
        }

        while let Some(mut ptr) = self.rolling_pending.pop_front() {
            let danmu = unsafe { ptr.as_mut() };
            let emitted = danmu.emitted_data.as_ref().unwrap();

            if let Some(y) = self.find_rolling_position(ptr, emitted.rect.height(), rect) {
                let danmu_mut = unsafe { ptr.as_mut() };
                if let Some(emitted) = danmu_mut.emitted_data.as_mut() {
                    emitted.rect = Rect::from_min_size(pos2(rect.right(), y), emitted.rect.size());

                    self.rolling_emitted_map
                        .insert(NotNan::new(y).unwrap(), (emitted.rect.height(), ptr));
                    self.emitted.insert(ptr);
                }
            } else {
                self.rolling_pending.push_front(ptr);
                break;
            }
        }
    }

    fn find_centered_position(
        &self,
        danmu_type: DanmuType,
        height: f32,
        rect: Rect,
    ) -> Option<f32> {
        match danmu_type {
            DanmuType::Top => {
                let mut last_bottom = rect.top();

                for (&top, &h) in &self.centered_emitted_map {
                    if top.into_inner() - last_bottom >= height {
                        return Some(last_bottom);
                    }
                    last_bottom = top.into_inner() + h;
                    if last_bottom + height > rect.bottom() {
                        break;
                    }
                }

                (last_bottom + height <= rect.bottom()).then_some(last_bottom)
            }

            DanmuType::Bottom => {
                let mut last_top = rect.bottom();

                for (&top, &h) in self.centered_emitted_map.iter().rev() {
                    if last_top - (top.into_inner() + h) >= height {
                        return Some(last_top - height);
                    }
                    last_top = top.into_inner();
                    if last_top - height < rect.top() {
                        break;
                    }
                }

                (last_top - height >= rect.top()).then_some(last_top - height)
            }

            _ => None,
        }
    }

    fn find_rolling_position(&self, ptr: DanmuPtr, height: f32, rect: Rect) -> Option<f32> {
        let mut last_bottom = rect.top();

        for (&top, &(cur_height, cur_ptr)) in &self.rolling_emitted_map {
            let top_val = top.into_inner();

            if top_val - last_bottom >= height {
                return Some(last_bottom);
            }

            let cur_danmu = unsafe { cur_ptr.as_ref() };
            if let Some(cur_emitted) = &cur_danmu.emitted_data {
                let speed_diff = unsafe { ptr.as_ref() }
                    .emitted_data
                    .as_ref()
                    .map(|e| e.speed - cur_emitted.speed)
                    .unwrap_or(0.0);

                let required_distance = 0.0_f32
                    .max(speed_diff * (cur_emitted.rect.right() - rect.left()) / cur_emitted.speed);

                let current_distance = rect.right() - cur_emitted.rect.right();

                if current_distance >= required_distance {
                    return Some(top_val);
                }
            }

            last_bottom = top_val + cur_height;
            if last_bottom + height > rect.bottom() {
                break;
            }
        }

        (last_bottom + height <= rect.bottom()).then_some(last_bottom)
    }

    fn measure_text(state: &State, ui: &egui::Ui, danmu: &mut DanmuData) {
        let font_id = state.font_loader.get_font_id();

        let galley =
            ui.fonts(|f| f.layout_no_wrap(danmu.text.clone(), font_id, egui::Color32::PLACEHOLDER));
        let size = galley.size();
        let padded_size = egui::vec2(size.x + 8.0, size.y + 4.0);

        let speed = (state.rolling_speed * padded_size.x / 160.0)
            .clamp(state.rolling_speed * 0.75, state.rolling_speed * 1.25);

        danmu.emitted_data = Some(DanmuEmittedData {
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), padded_size),
            lifetime: state.lifetime,
            speed,
            state: DanmuEmittedDataState::Ready,
            galley: Some(galley),
        });
    }
}
