use mlua::Lua;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::auto_script::binding::AutoScript;

pub struct ScriptExecutor {
    handle: Option<JoinHandle<Result<(), String>>>,
    cancel_flag: Arc<AtomicBool>,
}

impl ScriptExecutor {
    pub fn new() -> Self {
        ScriptExecutor {
            handle: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn check_script(&self, script: &str) -> Result<(), String> {
        let lua = Lua::new();
        AutoScript::register_with_cancel_flag(&lua, self.cancel_flag.clone())
            .map_err(|e| e.to_string())?;
        lua.load(script)
            .set_name("script")
            .into_function()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn execute_script(&mut self, script: String) {
        assert!(!self.is_executing());
        self.cancel_flag.store(false, Ordering::SeqCst);

        let flag = self.cancel_flag.clone();
        let code = script.clone();

        let handle = thread::spawn(move || {
            let lua = Lua::new();
            AutoScript::register_with_cancel_flag(&lua, flag).map_err(|e| e.to_string())?;
            lua.load(&code)
                .set_name("script")
                .exec()
                .map_err(|e| e.to_string())
        });

        self.handle = Some(handle);
    }

    pub fn try_get_execute_result(&mut self) -> Option<Result<(), String>> {
        if let Some(handle) = &self.handle {
            if handle.is_finished() {
                let result = self
                    .handle
                    .take()
                    .unwrap()
                    .join()
                    .unwrap_or_else(|e| Err(format!("Script panicked: {e:?}")));
                return Some(result);
            }
        }
        None
    }

    pub fn is_executing(&self) -> bool {
        self.handle.is_some()
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }
}
