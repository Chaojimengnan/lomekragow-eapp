pub mod ui;

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::HotKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub use global_hotkey::hotkey::{Code, Modifiers};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct KeyMap<Action: Default>(pub HashMap<u32, (HotKey, Action)>);

impl<Action: Default> Deref for KeyMap<Action> {
    type Target = HashMap<u32, (HotKey, Action)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Action: Default> DerefMut for KeyMap<Action> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct GlobalHotkeyHandler<Action: Default> {
    key_map: KeyMap<Action>,
    manager: Option<GlobalHotKeyManager>,
    action_to_edit: Option<Action>,
}

impl<Action> GlobalHotkeyHandler<Action>
where
    Action: Clone + PartialEq + Default,
{
    pub fn create_manager(&mut self) -> global_hotkey::Result<()> {
        self.manager = Some(GlobalHotKeyManager::new()?);
        Ok(())
    }

    pub fn get_key_map(&self) -> &KeyMap<Action> {
        &self.key_map
    }

    pub fn is_ok(&self) -> bool {
        self.manager.is_some()
    }

    pub fn register_hotkey(
        &mut self,
        action: Action,
        modifiers: Option<Modifiers>,
        keycode: Code,
    ) -> global_hotkey::Result<()> {
        assert!(self.is_ok(), "call `create_manager` first");
        let hotkey = HotKey::new(modifiers, keycode);
        self.manager.as_ref().unwrap().register(hotkey)?;
        self.key_map.insert(hotkey.id(), (hotkey, action));
        Ok(())
    }

    pub fn unregister_hotkey(&mut self, action: &Action) {
        assert!(self.is_ok(), "call `create_manager` first");
        let keys: Vec<_> = self
            .key_map
            .iter()
            .filter_map(|(_, (key, v))| if v == action { Some(*key) } else { None })
            .collect();

        for key in keys {
            let _ = self.manager.as_ref().unwrap().unregister(key);
            self.key_map.remove(&key.id());
        }
    }

    pub fn update_hotkey(
        &mut self,
        action: Action,
        modifiers: Option<Modifiers>,
        keycode: Code,
    ) -> global_hotkey::Result<()> {
        self.unregister_hotkey(&action);
        self.register_hotkey(action, modifiers, keycode)
    }

    pub fn poll_events(&mut self) -> Vec<Action> {
        assert!(self.is_ok(), "call `create_manager` first");
        let mut result = Vec::new();
        for event in GlobalHotKeyEvent::receiver().try_iter() {
            if let Some((_, action)) = self.key_map.get(&event.id())
                && self.action_to_edit.is_none()
            {
                result.push(action.clone());
            }
        }
        result
    }
}
