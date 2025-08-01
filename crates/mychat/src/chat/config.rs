use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ChatConfig {
    pub compression_threshold: f32,
    pub n_ctx: usize,

    pub summary_param: ChatParam,
    pub assistant_param: ChatParam,
    pub user_param: ChatParam,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            compression_threshold: 0.7,
            n_ctx: 4096,
            summary_param: ChatParam::summary_param(),
            assistant_param: ChatParam::param(),
            user_param: ChatParam::param(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatParam {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
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
            api_key: String::new(),
            api_url: String::new(),
            model: String::new(),
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
            api_key: String::new(),
            api_url: String::new(),
            model: String::new(),
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ChatConfigProfile {
    pub name: String,
    pub config: ChatConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ChatConfigManager {
    pub profiles: Vec<ChatConfigProfile>,
    pub current_profile_index: usize,
}

impl Default for ChatConfigManager {
    fn default() -> Self {
        Self {
            profiles: vec![ChatConfigProfile {
                name: "Default".to_string(),
                config: ChatConfig::default(),
            }],
            current_profile_index: 0,
        }
    }
}

impl ChatConfigManager {
    pub fn cur_config(&self) -> &ChatConfig {
        &self.profiles[self.current_profile_index].config
    }

    pub fn cur_config_mut(&mut self) -> &mut ChatConfig {
        &mut self.profiles[self.current_profile_index].config
    }

    pub fn cur_name(&self) -> &String {
        &self.profiles[self.current_profile_index].name
    }

    pub fn cur_name_mut(&mut self) -> &mut String {
        &mut self.profiles[self.current_profile_index].name
    }

    pub fn add_profile(&mut self, name: &str) {
        self.profiles.push(ChatConfigProfile {
            name: name.to_string(),
            config: self.cur_config().clone(),
        });
    }

    pub fn remove_profile(&mut self, index: usize) {
        if self.profiles.len() > 1 {
            self.profiles.remove(index);
            if self.current_profile_index >= index {
                self.current_profile_index = self.current_profile_index.saturating_sub(1);
            }
        }
    }
}
