use ansi_term::Colour::{Cyan, Red, Yellow};
use ansi_term::Style;

/// 把 Markdown 文本渲染成带 ANSI 样式的纯文本（参考 glow 最简版）
///
/// 支持的语法：
/// - `# / ## / ###` 标题 → 粗体青色
/// - `**bold**` → 粗体
/// - `*italic*` → 斜体
/// - `` `code` `` → 黄色反显
/// - `- / *` 列表 → `•` 前缀
/// - `>` 引用 → 灰色 `│` 前缀
/// - ``` ``` ``` 代码块 → 整段灰色
/// - `[text](url)` → `text`
pub fn render(text: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        // 代码块检测
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            out.push_str(&Style::new().dimmed().paint(line).to_string());
            out.push('\n');
            continue;
        }

        if in_code_block {
            out.push_str(&Style::new().dimmed().paint(line).to_string());
            out.push('\n');
            continue;
        }

        out.push_str(&render_line(line));
        out.push('\n');
    }

    // 去掉末尾多余的换行
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn render_line(line: &str) -> String {
    let trimmed = line.trim_start();

    // 标题
    if let Some(header) = strip_header(trimmed) {
        return Cyan.bold().paint(header).to_string();
    }

    // 引用
    if let Some(rest) = trimmed.strip_prefix("> ") {
        return Style::new().dimmed().paint(format!("│ {}", rest)).to_string();
    } else if trimmed == ">" {
        return Style::new().dimmed().paint("│").to_string();
    }

    // 列表
    if let Some(rest) = strip_list_marker(trimmed) {
        return format!("  • {}", render_inline(rest));
    }

    // 普通行
    render_inline(line)
}

fn strip_header(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix("### ") {
        Some(rest)
    } else if let Some(rest) = s.strip_prefix("## ") {
        Some(rest)
    } else if let Some(rest) = s.strip_prefix("# ") {
        Some(rest)
    } else {
        None
    }
}

fn strip_list_marker(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix("- ") {
        Some(rest)
    } else if let Some(rest) = s.strip_prefix("* ") {
        Some(rest)
    } else {
        None
    }
}

/// inline 渲染：bold/italic/code/link
fn render_inline(text: &str) -> String {
    // 先处理 code（避免内部 ** 被误识别）
    let mut out = String::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        let c = chars[i];

        // 反引号代码
        if c == '`' {
            if let Some(end) = find_char(&chars[i + 1..], '`') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str(&Yellow.paint(format!(" {} ", inner)).to_string());
                i = i + 1 + end + 1;
                continue;
            }
        }

        // 链接 [text](url)
        if c == '[' {
            if let Some((text_part, consumed)) = try_parse_link(&chars[i..]) {
                out.push_str(&text_part);
                i += consumed;
                continue;
            }
        }

        // **bold**
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars[i + 2..]) {
                let inner: String = chars[i + 2..i + 2 + end].iter().collect();
                out.push_str(&Style::new().bold().paint(inner).to_string());
                i = i + 2 + end + 2;
                continue;
            }
        }

        // *italic*
        if c == '*' {
            if let Some(end) = find_char(&chars[i + 1..], '*') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                if !inner.is_empty() {
                    out.push_str(&Style::new().italic().paint(inner).to_string());
                    i = i + 1 + end + 1;
                    continue;
                }
            }
        }

        let _ = bytes;
        out.push(c);
        i += 1;
    }

    out
}

fn find_char(chars: &[char], target: char) -> Option<usize> {
    chars.iter().position(|&c| c == target)
}

fn find_double_star(chars: &[char]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn try_parse_link(chars: &[char]) -> Option<(String, usize)> {
    // [text](url)
    let close_bracket = find_char(chars, ']')?;
    if close_bracket == 0 {
        return None;
    }
    let after = close_bracket + 1;
    if after >= chars.len() || chars[after] != '(' {
        return None;
    }
    let close_paren = find_char(&chars[after + 1..], ')')?;
    let text: String = chars[1..close_bracket].iter().collect();
    Some((text, close_paren + after + 2))
}

// 提供给 `Red` 避免未使用警告（保留 API 表面以便未来扩展）
#[allow(dead_code)]
fn _silence_unused() {
    let _ = Red;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_italic() {
        let s = render("这是 **粗体** 和 *斜体* 文本");
        assert!(s.contains("\x1b[1m粗体\x1b[0m"), "bold not found: {}", s);
        assert!(s.contains("\x1b[3m斜体\x1b[0m"), "italic not found: {}", s);
    }

    #[test]
    fn test_header() {
        let s = render("# 标题一\n## 标题二");
        assert!(s.contains("标题一"));
        assert!(s.contains("标题二"));
        // 粗体青色
        assert!(s.contains("\x1b[1;36m"));
    }

    #[test]
    fn test_list() {
        let s = render("- 项目一\n* 项目二");
        assert!(s.contains("• 项目一"));
        assert!(s.contains("• 项目二"));
    }

    #[test]
    fn test_code_block() {
        let s = render("```\nlet x = 1;\n```");
        // 灰色（dim）
        assert!(s.contains("\x1b[2m"), "code block not dimmed: {}", s);
        assert!(s.contains("let x = 1;"));
    }
}
