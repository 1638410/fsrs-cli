use anyhow::Result;
use std::io::{self, BufRead, BufReader, Write};

/// 简易多行输入循环
///
/// 用 `BufReader::read_line()` 读 stdin，回显和退格由系统 cooked mode 处理。
/// 用户逐行输入，输入 `:end` 或按 Ctrl+D（空行上）结束。
pub fn multiline_input(prompt: &str) -> Result<String> {
    println!(
        "{} (逐行输入 Markdown；单独输入 :end 结束；Ctrl+D 也可结束)",
        prompt
    );

    let mut lines: Vec<String> = Vec::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    loop {
        print!("  > ");
        io::stdout().flush()?;

        let mut line = String::new();
        let n = reader.read_line(&mut line)?;

        // n == 0 → EOF（Ctrl+D 在空行上），正常退出
        if n == 0 {
            break;
        }

        // 去掉末尾换行符
        if line.ends_with('\n') {
            line.pop();
        }

        if line.trim() == ":end" {
            break;
        }

        lines.push(line);
    }

    Ok(lines.join("\n"))
}

/// 解析 `|` 分隔的标签字符串
pub fn parse_tags(input: &str) -> Vec<String> {
    input
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tags_basic() {
        assert_eq!(parse_tags("Rust|语法"), vec!["Rust", "语法"]);
    }

    #[test]
    fn test_parse_tags_empty() {
        assert!(parse_tags("").is_empty());
        assert!(parse_tags("   ").is_empty());
    }

    #[test]
    fn test_parse_tags_with_spaces() {
        assert_eq!(
            parse_tags(" Rust | 语法 | 基础 "),
            vec!["Rust", "语法", "基础"]
        );
    }

    #[test]
    fn test_parse_tags_filter_empty() {
        assert_eq!(parse_tags("Rust||语法|"), vec!["Rust", "语法"]);
    }
}
