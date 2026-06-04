use anyhow::{anyhow, Result};
use std::path::Path;

use super::ParsedCard;

/// 解析 tab 分隔的 CSV 文件（无表头）
///
/// 格式：
/// - 2 列：`<问题>\t<答案>`
/// - 3 列：`<问题>\t<答案>\t<标签>`（标签用 `|` 分隔）
/// - 答案中的字面 `\n` 视为换行
/// - 空行跳过
pub fn parse_csv_file(file_path: &Path) -> Result<Vec<ParsedCard>> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow!("读取文件失败 {}: {}", file_path.display(), e))?;
    let source_file = file_path.to_str().unwrap_or("unknown").to_string();

    parse_csv(&content, &source_file)
}

/// 解析 tab 分隔的 CSV 文本（无表头）
pub fn parse_csv(content: &str, source_file: &str) -> Result<Vec<ParsedCard>> {
    let mut cards = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();

        // 跳过空行
        if trimmed.is_empty() {
            continue;
        }

        let cols: Vec<&str> = line.split('\t').collect();

        match cols.len() {
            2 => {
                let question = cols[0].trim();
                let answer_raw = cols[1].trim();

                if question.is_empty() {
                    return Err(anyhow!("CSV 第 {} 行: 列为空", line_no));
                }
                if answer_raw.is_empty() {
                    return Err(anyhow!("CSV 第 {} 行: 答案列为空", line_no));
                }

                let answer = unescape_newlines(answer_raw);
                cards.push(ParsedCard {
                    source_file: source_file.to_string(),
                    card_index: cards.len() as i32,
                    question: question.to_string(),
                    answer,
                    tags: Vec::new(),
                });
            }
            3 => {
                let question = cols[0].trim();
                let answer_raw = cols[1].trim();
                let tags_raw = cols[2].trim();

                if question.is_empty() {
                    return Err(anyhow!("CSV 第 {} 行: 列为空", line_no));
                }
                if answer_raw.is_empty() {
                    return Err(anyhow!("CSV 第 {} 行: 答案列为空", line_no));
                }

                let answer = unescape_newlines(answer_raw);
                let tags = parse_tags(tags_raw);
                cards.push(ParsedCard {
                    source_file: source_file.to_string(),
                    card_index: cards.len() as i32,
                    question: question.to_string(),
                    answer,
                    tags,
                });
            }
            n => {
                return Err(anyhow!(
                    "CSV 第 {} 行列数错误: 期望 2 或 3，实际 {}",
                    line_no,
                    n
                ));
            }
        }
    }

    Ok(cards)
}

/// 把字面 `\n` 转换为真换行符
fn unescape_newlines(s: &str) -> String {
    s.replace("\\n", "\n")
}

/// 按 `|` 分隔解析标签字符串
fn parse_tags(s: &str) -> Vec<String> {
    s.split('|')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_two_columns() {
        let csv = "问题一？\t答案一\n问题二？\t答案二";
        let cards = parse_csv(csv, "test.csv").unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].question, "问题一？");
        assert_eq!(cards[0].answer, "答案一");
        assert!(cards[0].tags.is_empty());
        assert_eq!(cards[1].question, "问题二？");
        assert_eq!(cards[1].answer, "答案二");
    }

    #[test]
    fn test_parse_three_columns_with_tags() {
        let csv = "问题1?\t答案1\tRust|语法\n问题2?\t答案2\t算法";
        let cards = parse_csv(csv, "test.csv").unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].tags, vec!["Rust", "语法"]);
        assert_eq!(cards[1].tags, vec!["算法"]);
    }

    #[test]
    fn test_escape_newlines() {
        let csv = "问题?\t第一行\\n第二行\\n第三行\t标签";
        let cards = parse_csv(csv, "test.csv").unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].answer, "第一行\n第二行\n第三行");
        assert_eq!(cards[0].tags, vec!["标签"]);
    }

    #[test]
    fn test_skip_empty_and_column_error() {
        // 第一行有效
        let csv = "问题?\t答案\n\n";
        let cards = parse_csv(csv, "test.csv").unwrap();
        assert_eq!(cards.len(), 1);

        // 列数错误
        let csv = "问题?\t答案\textra\tmore";
        let err = parse_csv(csv, "test.csv").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("第 1 行") && msg.contains("实际 4"), "msg={}", msg);

        // 答案空
        let csv = "问题?\t";
        let err = parse_csv(csv, "test.csv").unwrap_err();
        assert!(err.to_string().contains("答案列"));
    }
}
