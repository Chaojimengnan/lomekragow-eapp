use crate::mpv;
use eframe::egui::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Bound};
use walkdir::WalkDir;

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Playlist {
    #[serde(skip)]
    current_play: Option<(String, String)>,
    map: HashMap<String, BTreeSet<String>>,
}

impl Playlist {
    pub fn add_list(&mut self, list: String) {
        if self.map.contains_key(&list) {
            return;
        }

        let mut set = BTreeSet::new();

        eapp_utils::capture_error!(
            err => log::error!("playlist add list '{list}' fails: {err}"),
            {
                for item in WalkDir::new(&list) {
                    let item = item?;
                    let item_path = item.path();
                    let is_valid = item_path.is_file()
                        && item_path.extension().is_some_and(|ext| {
                            let ext = ext.to_str().unwrap_or("").to_ascii_lowercase();
                            mpv::VIDEO_FORMAT.contains(&ext.as_str())
                        });
                    if is_valid {
                        set.insert(item_path.to_string_lossy().into_owned());
                    }
                }
            }
        );

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
            if let Some(media_set) = self.map.get(&list) {
                if media_set.contains(&media) {
                    self.current_play = Some((list, media));
                }
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
        self.current_play.as_ref()?;

        let (list, media) = self.current_play.clone().unwrap();
        let media_set = &self.map[&list];
        let next = media_set
            .range((Bound::Excluded(media), Bound::Unbounded))
            .next()
            .unwrap_or(media_set.first().unwrap())
            .to_owned();

        self.set_current_play(Some((list, next.clone())));
        Some(next)
    }

    pub fn prev_item(&mut self) -> Option<String> {
        self.current_play.as_ref()?;

        let (list, media) = self.current_play.clone().unwrap();
        let list_c = &self.map[&list];
        let prev = list_c
            .range((Bound::Unbounded, Bound::Excluded(media)))
            .next_back()
            .unwrap_or(list_c.last().unwrap())
            .to_owned();

        self.set_current_play(Some((list, prev.clone())));
        Some(prev)
    }

    pub fn inner_map(&self) -> &HashMap<String, BTreeSet<String>> {
        &self.map
    }
}
