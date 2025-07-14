use crate::global_hotkey::GlobalHotkeyHandler;
use eframe::egui;
use global_hotkey::{
    Result,
    hotkey::{Code, HotKey, Modifiers},
};
use std::fmt::Debug;

impl<Action> GlobalHotkeyHandler<Action>
where
    Action: Clone + PartialEq + Debug + Default,
{
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        assert!(self.is_ok(), "call `create_manager` first");

        ui.add_enabled_ui(self.action_to_edit.is_none(), |ui| {
            ui.columns(2, |ui| {
                for (_, (hotkey, action)) in self.key_map.iter() {
                    ui[0].vertical_centered(|ui| ui.label(format!("{action:?}")));
                    ui[1].vertical_centered(|ui| {
                        if self.action_to_edit.as_ref().is_some_and(|a| a == action) {
                            ui.label("Press new hotkey... (BACKSPACE to cancel)");
                        } else {
                            let label = hotkey_label(hotkey);
                            if ui.button(label).clicked() {
                                self.action_to_edit = Some(action.clone());
                            }
                        }
                    });
                }
            });
        });

        if self.action_to_edit.is_none() {
            return Ok(());
        }

        let action = self.action_to_edit.clone().unwrap();

        ui.input(|input| {
            for event in &input.events {
                let egui::Event::Key {
                    key,
                    modifiers,
                    pressed,
                    ..
                } = event
                else {
                    continue;
                };

                if !pressed {
                    continue;
                }

                if *key == egui::Key::Backspace {
                    self.action_to_edit = None;
                    return Ok(());
                }

                if let Some(code) = to_code(*key) {
                    let mods = to_modifiers(*modifiers);

                    self.action_to_edit = None;
                    return self.update_hotkey(action, Some(mods), code);
                }
            }
            Ok(())
        })
    }
}

fn hotkey_label(hotkey: &HotKey) -> String {
    let mut label = String::new();
    if hotkey.mods.ctrl() {
        label.push_str("Ctrl+");
    }
    if hotkey.mods.alt() {
        label.push_str("Alt+");
    }
    if hotkey.mods.shift() {
        label.push_str("Shift+");
    }
    if hotkey.mods.meta() {
        label.push_str("Win+");
    }

    label.push_str(&format!("{:?}", hotkey.key));
    label
}

fn to_modifiers(mods: egui::Modifiers) -> Modifiers {
    let mut new_mods = Modifiers::empty();
    if mods.ctrl {
        new_mods |= Modifiers::CONTROL;
    }

    if mods.alt {
        new_mods |= Modifiers::ALT;
    }

    if mods.shift {
        new_mods |= Modifiers::SHIFT;
    }

    new_mods
}

fn to_code(key: egui::Key) -> Option<Code> {
    use egui::Key::*;
    match key {
        A => Some(Code::KeyA),
        B => Some(Code::KeyB),
        C => Some(Code::KeyC),
        D => Some(Code::KeyD),
        E => Some(Code::KeyE),
        F => Some(Code::KeyF),
        G => Some(Code::KeyG),
        H => Some(Code::KeyH),
        I => Some(Code::KeyI),
        J => Some(Code::KeyJ),
        K => Some(Code::KeyK),
        L => Some(Code::KeyL),
        M => Some(Code::KeyM),
        N => Some(Code::KeyN),
        O => Some(Code::KeyO),
        P => Some(Code::KeyP),
        Q => Some(Code::KeyQ),
        R => Some(Code::KeyR),
        S => Some(Code::KeyS),
        T => Some(Code::KeyT),
        U => Some(Code::KeyU),
        V => Some(Code::KeyV),
        W => Some(Code::KeyW),
        X => Some(Code::KeyX),
        Y => Some(Code::KeyY),
        Z => Some(Code::KeyZ),
        Num0 => Some(Code::Digit0),
        Num1 => Some(Code::Digit1),
        Num2 => Some(Code::Digit2),
        Num3 => Some(Code::Digit3),
        Num4 => Some(Code::Digit4),
        Num5 => Some(Code::Digit5),
        Num6 => Some(Code::Digit6),
        Num7 => Some(Code::Digit7),
        Num8 => Some(Code::Digit8),
        Num9 => Some(Code::Digit9),
        F1 => Some(Code::F1),
        F2 => Some(Code::F2),
        F3 => Some(Code::F3),
        F4 => Some(Code::F4),
        F5 => Some(Code::F5),
        F6 => Some(Code::F6),
        F7 => Some(Code::F7),
        F8 => Some(Code::F8),
        F9 => Some(Code::F9),
        F10 => Some(Code::F10),
        F11 => Some(Code::F11),
        F12 => Some(Code::F12),
        Escape => Some(Code::Escape),
        _ => None,
    }
}
