use mlua::{Lua, Value};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

pub struct Console {
    pub logs: VecDeque<String>,
    max_lines: usize,
    receiver: Receiver<String>,
}

impl Console {
    pub fn new(receiver: Receiver<String>) -> Self {
        Self {
            logs: VecDeque::new(),
            max_lines: 500,
            receiver,
        }
    }

    pub fn update(&mut self) {
        while let Ok(log) = self.receiver.try_recv() {
            self.logs.push_back(log);

            if self.logs.len() > self.max_lines {
                self.logs.pop_front();
            }
        }
    }
}

pub fn inject_lua_console(lua: &Lua, sender: Sender<String>) -> mlua::Result<()> {
    let print_func = lua.create_function(move |_, args: mlua::Variadic<Value>| {
        let mut output = String::new();
        for (i, val) in args.iter().enumerate() {
            if i > 0 {
                output.push('\t');
            }
            output += &match val {
                Value::String(s) => s
                    .to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "<invalid utf8>".to_string()),
                Value::Nil => "nil".to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Table(_) => "<table>".to_string(),
                Value::Function(_) => "<function>".to_string(),
                Value::Thread(_) => "<thread>".to_string(),
                Value::UserData(_) => "<userdata>".to_string(),
                Value::LightUserData(_) => "<lightuserdata>".to_string(),
                Value::Error(e) => format!("<error: {e}>"),
                Value::Other(_) => "<other>".to_string(),
            };
        }

        let _ = sender.send(output);
        Ok(())
    })?;

    lua.globals().set("print", print_func)?;
    Ok(())
}
