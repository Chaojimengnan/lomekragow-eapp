use mlua::Lua;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use crate::auto_script::{
    CONSOLE_SYSTEM_LOG_PREFIEX, SCRIPT_EXECUTION_CANCELLED_MSG,
    binding::AutoScript,
    console::{Console, inject_lua_console},
};

pub struct ScriptExecutor {
    pub console: Console,
    sender: Sender<String>,
    handle: Option<JoinHandle<Result<(), String>>>,
    cancel_flag: Arc<AtomicBool>,
}

impl ScriptExecutor {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        ScriptExecutor {
            console: Console::new(receiver),
            sender,
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
        let sender = self.sender.clone();

        let handle = thread::spawn(move || {
            let lua = Lua::new();
            inject_lua_console(&lua, sender).map_err(|e| e.to_string())?;
            AutoScript::register_with_cancel_flag(&lua, flag).map_err(|e| e.to_string())?;
            lua.load(&code)
                .set_name("script")
                .exec()
                .map_err(|e| e.to_string())
        });

        self.handle = Some(handle);
    }

    pub fn get_console_logs(&self) -> &VecDeque<String> {
        &self.console.logs
    }

    pub fn update(&mut self) {
        self.console.update();
    }

    pub fn try_get_execute_result(&mut self) -> Option<Result<(), String>> {
        if let Some(handle) = &self.handle
            && handle.is_finished()
        {
            let result = self
                .handle
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|e| Err(format!("Script panicked: {e:?}")));

            if let Err(err) = result.as_ref()
                && err.contains(SCRIPT_EXECUTION_CANCELLED_MSG)
            {
                self.console.logs.push_back(format!(
                    "{CONSOLE_SYSTEM_LOG_PREFIEX} Script execution was cancelled by user"
                ));
                return Some(Ok(()));
            }

            return Some(result);
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
