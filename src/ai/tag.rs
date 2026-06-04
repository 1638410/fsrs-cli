/// AI 自动标签
use anyhow::{Context, Result};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
};
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TagSuggestion {
    pub tags: Vec<String>,
}

/// 为卡片推荐标签
pub async fn suggest_tags(question: &str, answer: &str) -> Result<Vec<String>> {
    let config = OpenAIConfig::default();
    let client = Client::with_config(config);

    let prompt = format!(
        r#"为以下问答卡片推荐 1-5 个最相关的标签（中文或英文）。

问题：{question}
答案：{answer}

要求：
1. 标签应该是简洁的关键词（1-3个词）
2. 只输出 JSON 数组格式，例如 ["标签1", "标签2"]
3. 不要输出任何解释，只输出 JSON"#
    );

    let user_msg = ChatCompletionRequestUserMessage::from(prompt);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages([ChatCompletionRequestMessage::User(user_msg)])
        .temperature(0.2)
        .max_tokens(256u32)
        .build()
        .context("构建请求失败")?;

    let response = client
        .chat()
        .create(request)
        .await
        .context("调用 OpenAI API 失败")?;

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    let json_str = extract_json_array(&content);
    let tags: Vec<String> = serde_json::from_str(&json_str).unwrap_or_default();

    Ok(tags)
}

fn extract_json_array(text: &str) -> String {
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}
