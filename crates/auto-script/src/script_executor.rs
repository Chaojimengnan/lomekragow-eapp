use mlua::{Error::RuntimeError, HookTriggers, Lua, VmState};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::auto_script::AutoScript;

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
        AutoScript::register_to_global(&lua.globals()).map_err(|e| e.to_string())?;
        lua.load(script)
            .set_name("script")
            .into_function()
            .map(|_| ())
            .map_err(|e| format!("Lua syntax error: {e}"))
    }

    pub fn execute_script(&mut self, script: String) {
        self.cancel();
        if let Some(prev) = self.handle.take() {
            let _ = prev.join();
        }
        self.cancel_flag.store(false, Ordering::SeqCst);

        let flag = Arc::clone(&self.cancel_flag);
        let code = script.clone();

        let handle = thread::spawn(move || {
            let lua = Lua::new();
            lua.set_hook(
                HookTriggers {
                    every_nth_instruction: Some(1000),
                    ..Default::default()
                },
                move |_, _| {
                    if flag.load(Ordering::SeqCst) {
                        return Err(RuntimeError("Script cancelled".into()));
                    }
                    Ok(VmState::Continue)
                },
            );
            AutoScript::register_to_global(&lua.globals()).map_err(|e| e.to_string())?;
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
