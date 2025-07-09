use crate::chat::{Message, Role, config::ChatConfig};
use egui_commonmark::CommonMarkCache;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

#[derive(Default, PartialEq, Eq)]
pub enum DialogueState {
    #[default]
    Idle,
    Summarizing,
    Sending,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MessageWithUiData {
    pub message: Message,
    #[serde(skip)]
    pub cache: CommonMarkCache,
}

impl From<Message> for MessageWithUiData {
    fn from(message: Message) -> Self {
        Self {
            message,
            cache: CommonMarkCache::default(),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct ScrollState {
    all_messages_scroll: f32,
    all_messages_height: f32,
    summarized_height: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Dialogue {
    pub messages: VecDeque<MessageWithUiData>,
    pub summary: MessageWithUiData,
    pub amount_of_message_summarized: usize,
    #[serde(skip)]
    pub generate_user_input: bool,
    #[serde(skip)]
    pub state: DialogueState,
    #[serde(skip)]
    scroll_state: ScrollState,
}

impl Default for Dialogue {
    fn default() -> Self {
        let mut summary = MessageWithUiData::default();
        summary.message.role = Role::System;
        Self {
            messages: Default::default(),
            summary,
            amount_of_message_summarized: Default::default(),
            generate_user_input: Default::default(),
            state: Default::default(),
            scroll_state: Default::default(),
        }
    }
}

impl Dialogue {
    pub fn clear_summary(&mut self) {
        self.summary.message.clear();
        self.amount_of_message_summarized = 0;
    }

    pub fn is_summary_empty(&self) -> bool {
        self.summary.message.content.is_empty()
    }

    pub fn start_idx(&self, show_summarized: bool) -> usize {
        if show_summarized {
            0
        } else {
            self.amount_of_message_summarized
        }
    }

    fn height_offset(&self, show_summarized: bool) -> f32 {
        if show_summarized {
            0.0
        } else {
            self.scroll_state.all_messages_height - self.scroll_state.summarized_height
        }
    }

    pub fn scroll_offset(&self, show_summarized: bool) -> f32 {
        self.scroll_state.all_messages_scroll - self.height_offset(show_summarized)
    }

    pub fn set_height(&mut self, show_summarized: bool, new_height: f32) {
        if show_summarized {
            self.scroll_state.all_messages_height = new_height;
        } else {
            self.scroll_state.summarized_height = new_height;
        }
    }

    pub fn set_scroll_offset(&mut self, show_summarized: bool, new_offset: f32) {
        self.scroll_state.all_messages_scroll = new_offset + self.height_offset(show_summarized);
    }

    pub fn token_count(&self) -> usize {
        // TODO: use Tiktoken?
        let mut total = 0;

        total += self.summary.message.content.len();

        for msg in self.messages.iter().skip(self.amount_of_message_summarized) {
            total += msg.message.content.len();
        }

        total
    }

    pub fn back_to(&mut self, idx: isize) {
        assert!(self.is_idle());

        let new_len = (idx + 1).max(0) as usize;
        if new_len >= self.messages.len() {
            return;
        }

        self.messages.truncate(new_len);

        if new_len <= self.amount_of_message_summarized {
            self.clear_summary();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.state == DialogueState::Idle
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct DialoguesData {
    pub dialogues: VecDeque<Dialogue>,
    pub config: Arc<RwLock<ChatConfig>>,
}

impl DialoguesData {
    const FILENAME: &'static str = "dialogues_data.json";

    pub fn load() -> std::io::Result<Self> {
        let path = std::env::current_exe()?.join(format!("../{}", Self::FILENAME));
        Ok(serde_json::from_str::<DialoguesData>(
            &std::fs::read_to_string(path)?,
        )?)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = std::env::current_exe()?.join(format!("../{}", Self::FILENAME));
        std::fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }
}
