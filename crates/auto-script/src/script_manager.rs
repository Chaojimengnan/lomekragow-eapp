use serde::{Deserialize, Serialize};
use std::collections::{
    VecDeque,
    vec_deque::{Iter, IterMut},
};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Script {
    pub name: String,
    pub content: String,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            name: "New Script".to_string(),
            content: String::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ScriptManager {
    pub scripts: VecDeque<Script>,
}

impl ScriptManager {
    const FILENAME: &'static str = "scripts.json";

    pub fn new_script(&mut self) {
        self.scripts.push_front(Script::default());
    }

    pub fn remove_script(&mut self, idx: usize) {
        self.scripts.remove(idx);
    }

    pub fn iter(&self) -> Iter<'_, Script> {
        self.scripts.iter()
    }

    #[allow(unused)]
    pub fn iter_mut(&mut self) -> IterMut<'_, Script> {
        self.scripts.iter_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.scripts.is_empty()
    }

    pub fn load() -> std::io::Result<Self> {
        let path = std::env::current_exe()?.join(format!("../{}", Self::FILENAME));
        Ok(serde_json::from_str::<Self>(&std::fs::read_to_string(
            path,
        )?)?)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = std::env::current_exe()?.join(format!("../{}", Self::FILENAME));
        std::fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }
}
