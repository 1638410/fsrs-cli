use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser as MdParser, TagEnd};
use std::path::Path;

pub mod csv;
pub use csv::parse_csv_file;

/// 解析 Markdown 文件，提取所有卡片
pub fn parse_file(file_path: &Path) -> Result<Vec<ParsedCard>> {
    let content = std::fs::read_to_string(file_path)?;
    let source_file = file_path.to_str().unwrap_or("unknown").to_string();

    parse_markdown(&content, &source_file)
}

/// 解析 Markdown 文本，提取所有卡片
pub fn parse_markdown(content: &str, source_file: &str) -> Result<Vec<ParsedCard>> {
    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;

    let parser = MdParser::new_ext(content, options);

    let mut cards = Vec::new();
    let mut in_card_block = false;
    let mut card_content = String::new();
    let mut frontmatter_tags: Vec<String> = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_content = String::new();
    let mut card_index = 0;
    let mut last_event_was_text = false;

    for event in parser {
        match event {
            // 检测 YAML frontmatter
            Event::Start(pulldown_cmark::Tag::MetadataBlock(_)) => {
                in_frontmatter = true;
                frontmatter_content.clear();
            }
            Event::End(TagEnd::MetadataBlock(_)) => {
                in_frontmatter = false;
                frontmatter_tags = parse_frontmatter_tags(&frontmatter_content);
            }
            Event::Text(text) if in_frontmatter => {
                frontmatter_content.push_str(&text);
            }

            // 检测 HTML 注释 <!-- card -->
            Event::Html(html) => {
                let trimmed = html.trim();
                if trimmed == "<!-- card -->" {
                    if in_card_block {
                        // 结束卡片块，解析内容
                        if let Some(card) = parse_card_block(
                            &card_content,
                            source_file,
                            card_index,
                            &frontmatter_tags,
                        ) {
                            cards.push(card);
                            card_index += 1;
                        }
                        card_content.clear();
                        in_card_block = false;
                    } else {
                        // 开始新的卡片块
                        in_card_block = true;
                        card_content.clear();
                        last_event_was_text = false;
                    }
                }
            }

            // 在卡片块内的文本
            Event::Text(text) if in_card_block => {
                // pulldown-cmark 对同一段落内的连续 Text 节点不插入 SoftBreak，
                // 需要手动在连续 Text 之间补换行，否则多行内容会合并成一行。
                if last_event_was_text && !card_content.is_empty() {
                    card_content.push('\n');
                }
                card_content.push_str(&text);
                last_event_was_text = true;
            }
            Event::Code(code) if in_card_block => {
                card_content.push('`');
                card_content.push_str(&code);
                card_content.push('`');
                last_event_was_text = false;
            }
            Event::SoftBreak if in_card_block => {
                card_content.push('\n');
                last_event_was_text = false;
            }
            Event::HardBreak if in_card_block => {
                card_content.push('\n');
                last_event_was_text = false;
            }
            _ => {
                last_event_was_text = false;
            }
        }
    }

    // 如果文件末尾有未关闭的卡片块，尝试解析
    if in_card_block && !card_content.is_empty() {
        if let Some(card) =
            parse_card_block(&card_content, source_file, card_index, &frontmatter_tags)
        {
            cards.push(card);
        }
    }

    Ok(cards)
}

/// 解析 frontmatter 中的 tags
fn parse_frontmatter_tags(content: &str) -> Vec<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(tags_str) = line.strip_prefix("tags:") {
            let tags_str = tags_str.trim();
            // 尝试解析 YAML 数组格式: [tag1, tag2]
            if let Some(inner) = tags_str.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                return inner
                    .split(',')
                    .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// 解析卡片块内容，提取 Q:: 和 A::
fn parse_card_block(
    content: &str,
    source_file: &str,
    card_index: i32,
    tags: &[String],
) -> Option<ParsedCard> {
    let mut question = String::new();
    let mut answer = String::new();
    let mut current_field: Option<Field> = None;

    for line in content.lines() {
        let line = line.trim();

        if let Some(q) = line.strip_prefix("Q::") {
            current_field = Some(Field::Question);
            question = q.trim().to_string();
        } else if let Some(a) = line.strip_prefix("A::") {
            current_field = Some(Field::Answer);
            answer = a.trim().to_string();
        } else if let Some(ref field) = current_field {
            match field {
                Field::Question => {
                    if !question.is_empty() {
                        question.push('\n');
                    }
                    question.push_str(line);
                }
                Field::Answer => {
                    if !answer.is_empty() {
                        answer.push('\n');
                    }
                    answer.push_str(line);
                }
            }
        }
    }

    if question.is_empty() || answer.is_empty() {
        return None;
    }

    Some(ParsedCard {
        source_file: source_file.to_string(),
        card_index,
        question,
        answer,
        tags: tags.to_vec(),
    })
}

#[derive(Clone, Copy)]
enum Field {
    Question,
    Answer,
}

/// 解析后的卡片（尚未分配 ID）
#[derive(Debug)]
pub struct ParsedCard {
    pub source_file: String,
    pub card_index: i32,
    pub question: String,
    pub answer: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_card() {
        let md = r#"<!-- card -->
Q:: 问题一？
A:: 答案一。
<!-- card -->"#;

        let cards = parse_markdown(md, "test.md").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].question, "问题一？");
        assert_eq!(cards[0].answer, "答案一。");
    }

    #[test]
    fn test_parse_multiple_cards() {
        let md = r#"<!-- card -->
Q:: 第一个问题？
A:: 第一个答案。
<!-- card -->

<!-- card -->
Q:: 第二个问题？
A:: 第二个答案。
<!-- card -->"#;

        let cards = parse_markdown(md, "test.md").unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].card_index, 0);
        assert_eq!(cards[1].card_index, 1);
    }

    #[test]
    fn test_parse_with_frontmatter() {
        let md = r#"---
tags: [rust, ownership]
---

<!-- card -->
Q:: 什么是所有权？
A:: 每个值都有一个所有者。
<!-- card -->"#;

        let cards = parse_markdown(md, "test.md").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].tags, vec!["rust", "ownership"]);
    }

    #[test]
    fn test_multiline_card_content() {
        let md = r#"<!-- card -->
Q:: 多行问题？
   第二行
A:: 多行答案
   答案第二行
   答案第三行
<!-- card -->"#;

        let cards = parse_markdown(md, "test.md").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0].question,
            "多行问题？\n第二行"
        );
        assert_eq!(
            cards[0].answer,
            "多行答案\n答案第二行\n答案第三行"
        );
    }

    #[test]
    fn test_skip_invalid_card() {
        let md = r#"<!-- card -->
Q:: 只有问题没有答案
<!-- card -->"#;

        let cards = parse_markdown(md, "test.md").unwrap();
        assert_eq!(cards.len(), 0);
    }
}
