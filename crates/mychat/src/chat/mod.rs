pub mod config;
pub mod dialogue;
pub mod dialogue_manager;
pub mod dialogue_task;

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    Assistant,
    #[default]
    User,
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role = match self {
            Role::System => "system",
            Role::Assistant => "assistant",
            Role::User => "user",
        };
        write!(f, "{role}")
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub thinking_content: Option<String>,
}

impl Message {
    fn parse_thinking_content(response: &str) -> (Option<String>, String) {
        let think_start = "<think>";
        let think_end = "</think>";

        if let Some(start_idx) = response.find(think_start) {
            let content_start = start_idx + think_start.len();
            if let Some(end_idx) = response[content_start..].find(think_end) {
                let content_end = content_start + end_idx;
                let thinking = response[content_start..content_end].trim().to_string();
                let content = response[content_end + think_end.len()..].trim().to_string();
                return (Some(thinking), content);
            }
        }

        (None, response.to_string())
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.thinking_content = None;
    }

    pub fn split_thinking_content(&mut self) {
        if self.thinking_content.is_none() {
            let (thinking, content) = Message::parse_thinking_content(&self.content);
            self.content = content;
            self.thinking_content = thinking;
        }
    }
}
