use crate::chat::{
    Message, Role,
    config::{ChatConfig, ChatConfigManager},
    dialogue_manager::{CancellationToken, Request, Result, SendType, StreamType},
};
use anyhow::anyhow;
use eframe::egui;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::fmt::Write as _;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn dialogue_task(
    mut request_rx: Receiver<Request>,
    result_tx: Sender<Result>,
    manager: Arc<RwLock<ChatConfigManager>>,
    ctx: egui::Context,
) {
    loop {
        let request = match request_rx.recv().await {
            Some(request) => request,
            None => return,
        };

        match request {
            Request::Send((idx, send_type, messages, token)) => {
                tokio::spawn({
                    let config = manager.read().unwrap().cur_config().clone();
                    let tx = result_tx.clone();
                    let ctx = ctx.clone();

                    async move {
                        match stream_from_api(&ctx, token, &config, send_type, messages, &tx, idx)
                            .await
                        {
                            Ok(_) => {
                                let _ = tx.send(Result::Done(idx)).await;
                            }
                            Err(err) => {
                                let _ = tx.send(Result::Error((idx, err.to_string()))).await;
                            }
                        }
                        ctx.request_repaint();
                    }
                });
            }
        }
    }
}

async fn stream_from_api(
    ctx: &egui::Context,
    token: CancellationToken,
    config: &ChatConfig,
    send_type: SendType,
    messages: Vec<Message>,
    tx: &Sender<Result>,
    dialogue_idx: usize,
) -> anyhow::Result<()> {
    let (param, all_messages) = match send_type {
        SendType::Assistant => {
            let mut all_messages = vec![Message {
                role: Role::System,
                content: config.assistant_param.system_message.clone(),
                thinking_content: None,
            }];
            all_messages.extend(messages);

            (&config.assistant_param, all_messages)
        }
        SendType::User => {
            let mut all_messages = vec![Message {
                role: Role::System,
                content: config.user_param.system_message.clone(),
                thinking_content: None,
            }];
            all_messages.extend(messages);

            (&config.user_param, all_messages)
        }
        SendType::Summary => {
            let content = format!(
                "{}\n [CHAT HISTORY START]{}[CHAT HISTORY END]",
                config.summary_param.system_message,
                compress_message(&messages)?
            );

            let all_messages = vec![Message {
                role: Role::System,
                content,
                thinking_content: None,
            }];

            (&config.summary_param, all_messages)
        }
    };

    let request_body = json!({
        "model": param.model,
        "messages": all_messages,
        "max_tokens": param.max_tokens,
        "temperature": param.temperature,
        "top_p": param.top_p,
        "top_k": param.top_k,
        "min_p": param.min_p,
        "frequency_penalty": param.frequency_penalty,
        "presence_penalty": param.presence_penalty,
        "stream": true,
        "include_reasoning": true,
    });

    // TODO: DEBUG
    log::warn!("\n{send_type:?}: {request_body}\n\n");

    let response = Client::new()
        .post(&param.api_url)
        .header("Authorization", format!("Bearer {}", param.api_key))
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let stauts = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(anyhow!("API error {}: {}", stauts, error_body));
    }

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if token.is_cancelled() {
            return Err(anyhow!("Request cancelled"));
        }

        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        for line in chunk_str.lines() {
            if line.is_empty() || !line.starts_with("data: ") {
                continue;
            }

            if &line[6..] == "[DONE]" {
                break;
            }

            if let Some(data) = line.strip_prefix("data: ")
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                && let Some(delta) = json["choices"][0]["delta"].as_object()
            {
                macro_rules! send_streaming_if_has {
                    ($name: expr, $stream_type: expr) => {
                        if let Some(part) = delta.get($name).and_then(|v| v.as_str()) {
                            ctx.request_repaint();
                            tx.send(Result::Streaming((
                                dialogue_idx,
                                $stream_type,
                                part.to_string(),
                            )))
                            .await
                            .map_err(|e| anyhow!("Failed to send streaming: {}", e))?;
                        }
                    };
                }

                send_streaming_if_has!("content", StreamType::Content);
                send_streaming_if_has!("reasoning", StreamType::Reasoning);
            }
        }
    }

    Ok(())
}

fn compress_message(messages: &[Message]) -> anyhow::Result<String> {
    let mut content = String::new();
    for message in messages.iter() {
        writeln!(&mut content, "{} => {}", message.role, message.content)?;
    }
    Ok(content)
}
