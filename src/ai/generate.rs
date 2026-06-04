/// AI 卡片生成
use anyhow::{Context, Result};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
};
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};

/// AI 生成的 Q&A 对
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedQA {
    pub question: String,
    pub answer: String,
}

/// 从文本生成卡片草案
pub async fn generate_from_text(text: &str) -> Result<Vec<GeneratedQA>> {
    let config = OpenAIConfig::default();
    let client = Client::with_config(config);

    let prompt = format!(
        r#"你是一个记忆卡片生成器。请从以下文本中提取最重要的概念，生成问答对格式的抽认卡。

要求：
1. 每个概念生成一个问题和一个简洁准确的答案
2. 问题应该清晰具体，答案应该完整但不冗余
3. 只输出严格的 JSON 数组，每个元素包含 "question" 和 "answer" 字段
4. 如果文本没有值得提取的知识点，返回空数组 []
5. 不要输出任何解释，只输出 JSON

文本内容：
{text}"#
    );

    let user_msg = ChatCompletionRequestUserMessage::from(prompt);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages([ChatCompletionRequestMessage::User(user_msg)])
        .temperature(0.3)
        .max_tokens(4096u32)
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

    let json_str = extract_json(&content);
    let qa_list: Vec<GeneratedQA> = serde_json::from_str(&json_str).context("解析 AI 响应失败")?;

    Ok(qa_list)
}

/// 从文件生成卡片草案
pub async fn generate_from_file(path: &str) -> Result<Vec<GeneratedQA>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("无法读取文件: {}", path))?;

    // 如果文件太长，分块处理
    let chunks = split_text(&content, 3000);
    let mut all_qa = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        eprintln!("  处理块 {}/{}...", i + 1, chunks.len());
        let qa = generate_from_text(chunk).await?;
        all_qa.extend(qa);
    }

    Ok(all_qa)
}

/// 从主题关键词生成卡片草案
pub async fn generate_from_topic(topic: &str) -> Result<Vec<GeneratedQA>> {
    let prompt = format!(
        r#"你是一个记忆卡片生成器。请为以下主题生成 5-10 张高质量的问答卡片。

主题：{topic}

要求：
1. 覆盖该主题的核心概念
2. 问题应该清晰具体，答案应该完整但不冗余
3. 只输出严格的 JSON 数组，每个元素包含 "question" 和 "answer" 字段
4. 不要输出任何解释，只输出 JSON"#
    );

    let config = OpenAIConfig::default();
    let client = Client::with_config(config);

    let user_msg = ChatCompletionRequestUserMessage::from(prompt);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages([ChatCompletionRequestMessage::User(user_msg)])
        .temperature(0.3)
        .max_tokens(4096u32)
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

    let json_str = extract_json(&content);
    let qa_list: Vec<GeneratedQA> = serde_json::from_str(&json_str).context("解析 AI 响应失败")?;

    Ok(qa_list)
}

/// 从 AI 响应中提取 JSON
fn extract_json(text: &str) -> String {
    // 尝试找到 JSON 数组
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}

/// 将文本按最大长度分块，尽量在句号处切分
fn split_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > max_chars && !current.is_empty() {
            chunks.push(current);
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json() {
        let text = "Here is the result:\n[{\"question\": \"Q1\", \"answer\": \"A1\"}]\nDone.";
        let json = extract_json(text);
        assert_eq!(json, "[{\"question\": \"Q1\", \"answer\": \"A1\"}]");
    }

    #[test]
    fn test_split_text() {
        let text = "Line 1.\nLine 2.\nLine 3.\n";
        let chunks = split_text(text, 12);
        assert!(chunks.len() >= 2);
    }
}
