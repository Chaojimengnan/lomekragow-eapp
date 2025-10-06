use crate::mpv;
use eapp_utils::natordset::NatOrdSet;
use eframe::egui::ahash::HashMap;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Playlist {
    #[serde(skip)]
    current_play: Option<(String, String)>,
    map: HashMap<String, NatOrdSet>,
}

impl Playlist {
    pub fn add_list(&mut self, list: String) {
        if self.map.contains_key(&list) {
            return;
        }

        let mut set = NatOrdSet::new();

        eapp_utils::capture_error!(
            err => log::error!("playlist add list '{list}' fails: {err}"),
            {
                for item in WalkDir::new(&list) {
                    let item = item?;
                    let item_path = item.path();
                    let item_ext = mpv::get_ext_lowercase(item_path);

                    let is_valid = item_path.is_file()
                        && item_ext.is_some_and(|ext| {
                            mpv::VIDEO_FORMATS.contains(&ext.as_str()) || mpv::AUDIO_FORMATS.contains(&ext.as_str())
                        });
                    if is_valid {
                        set.push(item_path.to_string_lossy().into_owned());
                    }
                }
            }
        );

        set.sort();
        self.map.insert(list, set);
    }

    pub fn remove_list(&mut self, list: &str) {
        if self
            .current_play
            .as_ref()
            .is_some_and(|(cur_list, _)| cur_list == list)
        {
            self.current_play = None;
        }

        self.map.remove(list);
    }

    pub fn set_current_play(&mut self, list_and_media: Option<(String, String)>) {
        if let Some((list, media)) = list_and_media {
            if let Some(media_set) = self.map.get(&list)
                && media_set.search(&media).is_ok()
            {
                self.current_play = Some((list, media));
            }
        } else {
            self.current_play = None;
        }
    }

    pub fn current_play(&self) -> Option<(&str, &str)> {
        self.current_play
            .as_ref()
            .map(|(list, media)| (list.as_str(), media.as_str()))
    }

    pub fn next_item(&mut self) -> Option<String> {
        let (list, media) = self.current_play.clone()?;
        let media_set = &self.map[&list];

        let next_idx = match media_set.search(&media) {
            Ok(media_idx) => (media_idx + 1) % media_set.0.len(),
            _ => 0,
        };

        let next = media_set.0[next_idx].clone();

        self.set_current_play(Some((list, next.clone())));
        Some(next)
    }

    pub fn prev_item(&mut self) -> Option<String> {
        let (list, media) = self.current_play.clone()?;
        let media_set = &self.map[&list];

        let prev_idx = match media_set.search(&media) {
            Ok(media_idx) => {
                if media_idx != 0 {
                    media_idx - 1
                } else {
                    media_set.0.len() - 1
                }
            }
            _ => 0,
        };
        let prev = media_set.0[prev_idx].clone();

        self.set_current_play(Some((list, prev.clone())));
        Some(prev)
    }

    pub fn inner_map(&self) -> &HashMap<String, NatOrdSet> {
        &self.map
    }
}
