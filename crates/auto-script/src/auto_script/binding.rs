use mlua::{Error::RuntimeError, Lua, Result, Table, UserData, UserDataMethods, Value};
use rustautogui::{MatchMode, MouseClick, RustAutoGui};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::auto_script::SCRIPT_EXECUTION_CANCELLED_MSG;

pub struct AutoScript;

impl AutoScript {
    pub fn register_with_cancel_flag(lua: &Lua, cancel_flag: Arc<AtomicBool>) -> mlua::Result<()> {
        let constructor = lua.create_function(move |_, debug: bool| {
            let inner = RustAutoGui::new(debug).map_err(|e| RuntimeError(e.to_string()))?;
            Ok(AutoGui {
                inner,
                cancel_flag: cancel_flag.clone(),
            })
        })?;
        let table = lua.create_table()?;
        table.set("new", constructor)?;
        lua.globals().set("AutoScript", table)
    }
}

impl UserData for AutoScript {}

pub struct AutoGui {
    pub inner: RustAutoGui,
    pub cancel_flag: Arc<AtomicBool>,
}

impl AutoGui {
    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

impl UserData for AutoGui {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        macro_rules! cancelled {
            ($this:expr) => {
                if $this.is_cancelled() {
                    return Err(RuntimeError(SCRIPT_EXECUTION_CANCELLED_MSG.into()));
                }
            };
        }

        // ----- Click methods with String parsing -----
        macro_rules! click_str_method {
            ($name:literal, $fn_call:ident) => {
                methods.add_method($name, |_, this, s: String| {
                    cancelled!(this);
                    let btn = parser::parse_mouse_click(s)?;
                    this.inner
                        .$fn_call(btn)
                        .map_err(|e| RuntimeError(e.to_string()))
                });
            };
        }
        click_str_method!("click", click);
        click_str_method!("click_down", click_down);
        click_str_method!("click_up", click_up);

        // ----- Simple click macros -----
        macro_rules! click_simple {
            ($name:literal, $fn_call:ident) => {
                methods.add_method($name, |_, this, ()| {
                    cancelled!(this);
                    this.inner
                        .$fn_call()
                        .map_err(|e| RuntimeError(e.to_string()))
                });
            };
        }
        click_simple!("double_click", double_click);
        click_simple!("left_click", left_click);
        click_simple!("right_click", right_click);
        click_simple!("middle_click", middle_click);

        // ----- Parameterized mouse methods -----
        macro_rules! mouse_method {
            ($name:literal, $fn_call:ident, ($($arg:ident : $ty:ty),*)) => {
                methods.add_method($name, |_, this, ($($arg),*): ($($ty),*)| {
                    cancelled!(this);
                    this.inner.$fn_call($($arg),*)
                        .map_err(|e| RuntimeError(e.to_string()))
                });
            };
        }
        mouse_method!("move_mouse", move_mouse, (x: i32, y: i32, t: f32));
        mouse_method!("move_mouse_to_pos", move_mouse_to_pos, (x: u32, y: u32, t: f32));
        mouse_method!("drag_mouse", drag_mouse, (x: i32, y: i32, t: f32));
        mouse_method!("drag_mouse_to_pos", drag_mouse_to_pos, (x: u32, y: u32, t: f32));

        // Optional-coordinate variants
        methods.add_method(
            "move_mouse_to",
            |_, this, (ox, oy, t): (Option<u32>, Option<u32>, f32)| {
                cancelled!(this);
                this.inner
                    .move_mouse_to(ox, oy, t)
                    .map_err(|e| RuntimeError(e.to_string()))
            },
        );
        methods.add_method(
            "drag_mouse_to",
            |_, this, (ox, oy, t): (Option<u32>, Option<u32>, f32)| {
                cancelled!(this);
                this.inner
                    .drag_mouse_to(ox, oy, t)
                    .map_err(|e| RuntimeError(e.to_string()))
            },
        );

        // ----- Scrolling methods -----
        macro_rules! scroll_method {
            ($name:literal, $fn_call:ident) => {
                methods.add_method($name, |_, this, n: u32| {
                    cancelled!(this);
                    this.inner
                        .$fn_call(n)
                        .map_err(|e| RuntimeError(e.to_string()))
                });
            };
        }
        scroll_method!("scroll_up", scroll_up);
        scroll_method!("scroll_down", scroll_down);
        scroll_method!("scroll_left", scroll_left);
        scroll_method!("scroll_right", scroll_right);

        // ----- Keyboard methods -----
        macro_rules! key_method {
            ($name:literal, $fn_call:ident) => {
                methods.add_method($name, |_, this, key: String| {
                    cancelled!(this);
                    this.inner
                        .$fn_call(&key)
                        .map_err(|e| RuntimeError(e.to_string()))
                });
            };
        }
        key_method!("key_down", key_down);
        key_method!("key_up", key_up);
        methods.add_method("keyboard_input", |_, this, text: String| {
            cancelled!(this);
            this.inner
                .keyboard_input(&text)
                .map_err(|e| RuntimeError(e.to_string()))
        });
        methods.add_method("keyboard_command", |_, this, cmd: String| {
            cancelled!(this);
            this.inner
                .keyboard_command(&cmd)
                .map_err(|e| RuntimeError(e.to_string()))
        });
        methods.add_method(
            "keyboard_multi_key",
            |_, this, (a, b, c): (String, String, Option<String>)| {
                cancelled!(this);
                this.inner
                    .keyboard_multi_key(&a, &b, c.as_deref())
                    .map_err(|e| RuntimeError(e.to_string()))
            },
        );

        // ----- Utility methods -----
        methods.add_method("get_mouse_position", |_, this, ()| {
            cancelled!(this);
            this.inner
                .get_mouse_position()
                .map_err(|e| RuntimeError(e.to_string()))
        });
        methods.add_method_mut("get_screen_size", |_, this, ()| {
            Ok(this.inner.get_screen_size())
        });

        // ----- Sleep binding -----
        methods.add_method("sleep", |_, this, secs: f32| {
            cancelled!(this);
            std::thread::sleep(Duration::from_secs_f32(secs));
            Ok(Value::Nil)
        });

        // ----- Image template methods -----
        methods.add_method_mut(
            "store_image",
            |_, this, (path, tbl, mode_s, alias): (String, Option<Table>, String, String)| {
                cancelled!(this);
                let region = parser::parse_region(tbl)?;
                let mode = parser::parse_match_mode(mode_s)?;
                this.inner
                    .store_template_from_file(&path, region, mode, &alias)
                    .map_err(|e| RuntimeError(e.to_string()))?;
                Ok(Value::Nil)
            },
        );

        // ----- Image finding methods -----
        methods.add_method_mut(
            "find_image_on_screen",
            |lua, this, (precision, alias): (f32, String)| {
                cancelled!(this);
                let r = this
                    .inner
                    .find_stored_image_on_screen(precision, &alias)
                    .map_err(|e| RuntimeError(e.to_string()))?;
                parser::results_to_table(lua, r)
            },
        );
        methods.add_method_mut(
            "find_image_on_screen_and_move",
            |lua, this, (precision, time, alias): (f32, f32, String)| {
                cancelled!(this);
                let r = this
                    .inner
                    .find_stored_image_on_screen_and_move_mouse(precision, time, &alias)
                    .map_err(|e| RuntimeError(e.to_string()))?;
                parser::results_to_table(lua, r)
            },
        );
        methods.add_method_mut(
            "loop_find_image_on_screen",
            |lua, this, (precision, timeout, alias): (f32, u64, String)| {
                cancelled!(this);
                let r = this
                    .inner
                    .loop_find_stored_image_on_screen(precision, timeout, &alias)
                    .map_err(|e| RuntimeError(e.to_string()))?;
                parser::results_to_table(lua, r)
            },
        );
        methods.add_method_mut(
            "loop_find_image_on_screen_and_move",
            |lua, this, (precision, time, timeout, alias): (f32, f32, u64, String)| {
                cancelled!(this);
                let r = this
                    .inner
                    .loop_find_stored_image_on_screen_and_move_mouse(
                        precision, time, timeout, &alias,
                    )
                    .map_err(|e| RuntimeError(e.to_string()))?;
                parser::results_to_table(lua, r)
            },
        );
    }
}

mod parser {
    use super::*;

    pub fn parse_region(opt: Option<Table>) -> Result<Option<(u32, u32, u32, u32)>> {
        if let Some(tbl) = opt {
            let x = tbl.get::<u32>(1)?;
            let y = tbl.get::<u32>(2)?;
            let w = tbl.get::<u32>(3)?;
            let h = tbl.get::<u32>(4)?;
            Ok(Some((x, y, w, h)))
        } else {
            Ok(None)
        }
    }

    pub fn parse_match_mode(s: String) -> Result<MatchMode> {
        match s.as_str() {
            "FFT" => Ok(MatchMode::FFT),
            "Segmented" => Ok(MatchMode::Segmented),
            other => Err(RuntimeError(format!("Unknown MatchMode: {other}"))),
        }
    }

    pub fn parse_mouse_click(s: String) -> Result<MouseClick> {
        match s.to_lowercase().as_str() {
            "left" => Ok(MouseClick::LEFT),
            "right" => Ok(MouseClick::RIGHT),
            "middle" => Ok(MouseClick::MIDDLE),
            other => Err(RuntimeError(format!("Unknown MouseClick: {other}"))),
        }
    }

    pub fn results_to_table(lua: &Lua, opt: Option<Vec<(u32, u32, f32)>>) -> Result<Value> {
        if let Some(vec) = opt {
            let tbl = lua.create_table()?;
            for (i, (x, y, score)) in vec.into_iter().enumerate() {
                let entry = lua.create_table()?;
                entry.set("x", x)?;
                entry.set("y", y)?;
                entry.set("score", score)?;
                tbl.set(i + 1, entry)?;
            }
            Ok(Value::Table(tbl))
        } else {
            Ok(Value::Nil)
        }
    }
}
