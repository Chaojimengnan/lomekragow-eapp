use crate::chat::{
    Message, Role,
    dialogue::{Dialogue, DialogueState, DialoguesData},
    dialogue_task::dialogue_task,
};

use eframe::egui;

use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub enum Request {
    Send((usize, SendType, Vec<Message>, CancellationToken)),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendType {
    Assistant,
    User,
    Summary,
}

pub enum Result {
    Streaming((usize, String)),
    Done(usize),
    Error((usize, String)),
}

#[derive(Default, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct DialogueManager {
    pub cur_dialogue_idx: usize,
    pub data: DialoguesData,
    request_tx: Sender<Request>,
    result_rx: Receiver<Result>,
    cancellation_tokens: HashMap<usize, CancellationToken>,
}

impl DialogueManager {
    pub fn new(ctx: egui::Context) -> Self {
        let data = match DialoguesData::load() {
            Ok(data) => data,
            Err(err) => {
                log::error!("Error when load `DialogueData`: {err}");
                DialoguesData::default()
            }
        };
        let (request_tx, request_rx) = mpsc::channel::<Request>(100);
        let (result_tx, result_rx) = mpsc::channel::<Result>(100);

        tokio::spawn(dialogue_task(
            request_rx,
            result_tx,
            data.config.clone(),
            ctx,
        ));

        let cur_dialogue_idx = 0;
        let cancellation_tokens = HashMap::new();

        Self {
            cur_dialogue_idx,
            data,
            request_tx,
            result_rx,
            cancellation_tokens,
        }
    }

    pub fn new_dialogue(&mut self) {
        self.data.dialogues.push_front(Dialogue::default());
        self.cur_dialogue_idx = 0;
    }

    pub fn remove_dialogue(&mut self, dialogue_idx: usize) {
        assert!(self.is_idle());
        if dialogue_idx >= self.data.dialogues.len() {
            return;
        }

        self.data.dialogues.remove(dialogue_idx);

        self.cur_dialogue_idx = self
            .cur_dialogue_idx
            .min(self.data.dialogues.len().saturating_sub(1));
    }

    pub fn is_idle(&self) -> bool {
        self.cancellation_tokens.is_empty()
    }

    pub fn is_dialogue_idle(&self, idx: usize) -> bool {
        !self.cancellation_tokens.contains_key(&idx)
    }

    pub fn is_cur_dialogue_idle(&self) -> bool {
        self.is_dialogue_idle(self.cur_dialogue_idx)
    }

    pub fn is_empty(&self) -> bool {
        self.data.dialogues.is_empty()
    }

    pub fn dialogue(&self, idx: usize) -> &Dialogue {
        assert!(!self.is_empty());
        &self.data.dialogues[idx]
    }

    pub fn dialogue_mut(&mut self, idx: usize) -> &mut Dialogue {
        assert!(!self.is_empty());
        &mut self.data.dialogues[idx]
    }

    pub fn cur_dialogue(&self) -> &Dialogue {
        self.dialogue(self.cur_dialogue_idx)
    }

    pub fn cur_dialogue_mut(&mut self) -> &mut Dialogue {
        self.dialogue_mut(self.cur_dialogue_idx)
    }

    pub fn len(&self) -> usize {
        self.data.dialogues.len()
    }

    pub fn push_message(&mut self, msg: Message) {
        assert!(self.is_cur_dialogue_idle());

        let dialogue = &mut self.data.dialogues[self.cur_dialogue_idx];
        dialogue.messages.push_back(msg.into());
    }

    pub fn trigger_request(&mut self) {
        assert!(self.is_cur_dialogue_idle());

        let token = CancellationToken::new();
        self.cancellation_tokens
            .insert(self.cur_dialogue_idx, token.clone());

        let dialogue = &mut self.data.dialogues[self.cur_dialogue_idx];

        dialogue.generate_user_input = dialogue.messages.is_empty()
            || dialogue
                .messages
                .back()
                .is_some_and(|m| m.message.role == Role::Assistant);
        dialogue.messages.push_back(
            Message {
                role: Role::Assistant.reversed_if(dialogue.generate_user_input),
                content: String::new(),
                thinking_content: None,
            }
            .into(),
        );

        let config = self.data.config.read().unwrap();
        let current_tokens = dialogue.token_count();
        let threshold = (config.compression_threshold * config.n_ctx as f32) as usize;

        if current_tokens > threshold {
            dialogue.state = DialogueState::Summarizing;

            let mut accumulated_tokens = 0;
            let mut start_idx = dialogue.amount_of_message_summarized;
            let end_idx = dialogue.messages.len().saturating_sub(2).max(start_idx);

            for idx in dialogue.amount_of_message_summarized..end_idx {
                if let Some(msg) = dialogue.messages.get(idx) {
                    accumulated_tokens += msg.message.content.len();

                    if (current_tokens - accumulated_tokens) <= threshold / 2 {
                        start_idx = idx + 1;
                        break;
                    }
                }
            }

            start_idx = start_idx.min(end_idx.saturating_sub(1));

            let mut messages_to_summarize: Vec<Message> = dialogue
                .messages
                .range(dialogue.amount_of_message_summarized..start_idx)
                .map(|m| m.message.clone())
                .collect();

            dialogue.amount_of_message_summarized = start_idx;

            let mut summary_message = dialogue.summary.message.clone();
            if summary_message.content.is_empty() {
                summary_message.content = "empty".to_owned();
            }

            messages_to_summarize.insert(0, summary_message);
            dialogue.summary.message.clear();

            tokio::spawn({
                let idx = self.cur_dialogue_idx;
                let tx = self.request_tx.clone();
                async move {
                    let _ = tx
                        .send(Request::Send((
                            idx,
                            SendType::Summary,
                            messages_to_summarize,
                            token,
                        )))
                        .await;
                }
            });
        } else {
            dialogue.state = DialogueState::Sending;

            let (messages_to_send, send_type) =
                self.prepare_messages_for_sending(self.cur_dialogue_idx);

            tokio::spawn({
                let idx = self.cur_dialogue_idx;
                let tx = self.request_tx.clone();
                async move {
                    let _ = tx
                        .send(Request::Send((idx, send_type, messages_to_send, token)))
                        .await;
                }
            });
        }
    }

    pub fn cancel(&mut self) {
        assert!(!self.is_cur_dialogue_idle());

        if let Some(token) = self.cancellation_tokens.get(&self.cur_dialogue_idx) {
            token.cancel();
        }
    }

    pub fn update(&mut self, status_msg: &mut String) {
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                Result::Streaming((idx, content)) => {
                    if let Some(dialogue) = self.data.dialogues.get_mut(idx) {
                        match dialogue.state {
                            DialogueState::Summarizing => {
                                dialogue.summary.message.content.push_str(&content);
                                dialogue.summary.message.split_thinking_content();
                            }
                            DialogueState::Sending => {
                                if let Some(last_msg) = dialogue.messages.back_mut() {
                                    last_msg.message.content.push_str(&content);
                                    last_msg.message.split_thinking_content();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Result::Done(idx) => {
                    if let Some(dialogue) = self.data.dialogues.get_mut(idx) {
                        match dialogue.state {
                            DialogueState::Summarizing => {
                                dialogue.summary.message.split_thinking_content();

                                dialogue.state = DialogueState::Sending;
                                let (messages_to_send, send_type) =
                                    self.prepare_messages_for_sending(idx);

                                let token = self.cancellation_tokens.get(&idx).unwrap().clone();

                                let tx = self.request_tx.clone();
                                tokio::spawn(async move {
                                    let _ = tx
                                        .send(Request::Send((
                                            idx,
                                            send_type,
                                            messages_to_send,
                                            token,
                                        )))
                                        .await;
                                });
                            }
                            DialogueState::Sending => {
                                if let Some(last_msg) = dialogue.messages.back_mut() {
                                    last_msg.message.split_thinking_content();
                                }

                                dialogue.state = DialogueState::Idle;
                                dialogue.generate_user_input = false;
                                self.cancellation_tokens.remove(&idx);
                            }
                            _ => {}
                        }
                    }
                }
                Result::Error((idx, err)) => {
                    let error_msg = format!("Dialogue error: {err}");
                    log::error!("{error_msg}");
                    *status_msg = error_msg;
                    if let Some(dialogue) = self.data.dialogues.get_mut(idx) {
                        dialogue.state = DialogueState::Idle;
                        dialogue.generate_user_input = false;
                    }
                    self.cancellation_tokens.remove(&idx);
                }
            }
        }
    }

    pub fn save(&self) {
        if let Err(err) = self.data.save() {
            log::error!("Error when save `DialogueData`: {err}");
        }
    }

    fn prepare_messages_for_sending(&self, dialogue_idx: usize) -> (Vec<Message>, SendType) {
        let mut messages = Vec::new();
        let dialogue = &self.data.dialogues[dialogue_idx];

        if !dialogue.is_summary_empty() {
            messages.push(dialogue.summary.message.clone());
        }

        let start_idx = dialogue.amount_of_message_summarized;
        let end_idx = dialogue.messages.len().saturating_sub(1);
        messages.extend(
            dialogue
                .messages
                .range(start_idx..end_idx)
                .map(|m| Message {
                    role: m.message.role.reversed_if(dialogue.generate_user_input),
                    content: m.message.content.clone(),
                    thinking_content: None,
                }),
        );

        let send_type = match dialogue.generate_user_input {
            true => SendType::User,
            false => SendType::Assistant,
        };

        (messages, send_type)
    }
}
