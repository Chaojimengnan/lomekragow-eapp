use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatConfig {
    pub compression_threshold: f32,

    pub n_ctx: usize,
    pub api_key: String,
    pub api_url: String,
    pub model: String,

    pub summary_param: ChatParam,
    pub param: ChatParam,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            compression_threshold: 0.7,
            n_ctx: 4096,
            api_key: String::new(),
            api_url: String::new(),
            model: String::new(),
            summary_param: ChatParam::summary_param(),
            param: ChatParam::param(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatParam {
    pub max_tokens: isize,
    pub system_message: String,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub min_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
}

impl ChatParam {
    fn summary_param() -> Self {
        Self {
            max_tokens: -1,
            system_message: String::from(
                r"【强制总结指令】
请严格按以下格式输出总结：
1. 开头必须为：'历史对话总结如下：'
2. 尽可能详细概括对话核心内容，但是不要超过600字
3. 禁用任何额外输出
4. 不要丢弃上一次总结的信息，而是将其与现有的历史对话结合在一起

【历史对话格式】
1. 历史对话以`[CHAT HISTORY START]`开头，以`[CHAT HISTORY END]`结束
1. 以`system =>`开头是上一次总结的历史对话(可能为`empty`)
2. 以`user =>`开头是用户的输入
3. 以`assistant =>`开头是AI的输出
----------------",
            ),
            temperature: 0.1,
            top_p: 0.95,
            top_k: 40,
            min_p: 0.05,
            frequency_penalty: 0.0,
            presence_penalty: 1.2,
        }
    }

    fn param() -> Self {
        Self {
            max_tokens: -1,
            system_message: String::new(),
            temperature: 0.7,
            top_p: 0.95,
            top_k: 40,
            min_p: 0.05,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        }
    }
}
