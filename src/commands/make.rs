use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input};
use std::path::Path;

use crate::tui::editor;

/// TUI 制卡命令：交互式输入卡片，写入 Markdown，自动 import
pub fn run(base_path: &Path, output: &Path) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    // 输出文件路径：相对 base_path
    let output_abs = if output.is_absolute() {
        output.to_path_buf()
    } else {
        base_path.join(output)
    };

    println!(
        "{} 制作卡组 → {}",
        style("📝").blue(),
        output_abs.display()
    );
    println!();

    let mut raw_cards: Vec<RawCard> = Vec::new();

    loop {
        // 问题
        let question: String = Input::new()
            .with_prompt("问题")
            .allow_empty(false)
            .interact_text()
            .context("输入问题失败")?;

        // 答案（多行，:end 结束）
        let answer = editor::multiline_input("答案 (支持 Markdown)")?;
        if answer.trim().is_empty() {
            println!("{} 答案为空，已跳过这张卡片", style("⚠").yellow());
            continue;
        }

        // 标签
        let tags_input: String = Input::new()
            .with_prompt("标签 (| 分隔，可留空)")
            .allow_empty(true)
            .interact_text()
            .context("输入标签失败")?;
        let tags = editor::parse_tags(&tags_input);

        raw_cards.push(RawCard {
            question: question.trim().to_string(),
            answer: answer.trim().to_string(),
            tags,
        });

        println!(
            "{} 已添加第 {} 张卡片",
            style("✓").green(),
            raw_cards.len()
        );
        println!();

        let again = Confirm::new()
            .with_prompt("继续添加下一张?")
            .default(true)
            .interact()
            .unwrap_or(false);
        if !again {
            break;
        }
    }

    if raw_cards.is_empty() {
        println!("{} 未添加任何卡片，退出", style("⚠").yellow());
        return Ok(());
    }

    // 写入文件
    write_to_file(&output_abs, &raw_cards)?;
    println!(
        "{} 已写入 {} 张卡片到 {}",
        style("✓").green(),
        raw_cards.len(),
        output_abs.display()
    );

    // 自动 import
    println!();
    println!("{} 自动导入中...", style("📥").blue());
    crate::commands::import::run(&output_abs, base_path)?;

    Ok(())
}

struct RawCard {
    question: String,
    answer: String,
    tags: Vec<String>,
}

/// 检查文件是否已有 frontmatter 的 tags: 字段
fn has_frontmatter_tag(path: &Path) -> bool {
    use std::fs::OpenOptions;
    use std::io::{BufRead, BufReader};

    let file = match OpenOptions::new().read(true).open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);
    let mut in_frontmatter = false;

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && trimmed.starts_with("tags:") {
            return true;
        }
    }

    false
}

/// 追加或创建 Markdown 文件
fn write_to_file(path: &Path, cards: &[RawCard]) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let exists = path.exists();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("打开文件失败: {}", path.display()))?;

    if exists {
        // 已存在 → 追加前加空行分隔
        writeln!(file)?;
    }

    // 收集所有非空标签，文件级 frontmatter
    let all_tags: Vec<String> = cards
        .iter()
        .flat_map(|c| c.tags.iter().cloned())
        .filter(|t| !t.is_empty())
        .collect();

    if !all_tags.is_empty() {
        let has_frontmatter = exists && has_frontmatter_tag(path);
        if !exists {
            // 新文件 → 写 frontmatter
            writeln!(file, "---")?;
            writeln!(file, "tags: [{}]", all_tags.join(", "))?;
            writeln!(file, "---")?;
            writeln!(file)?;
        } else if !has_frontmatter {
            // 已有文件但无 frontmatter → 在末尾追加 frontmatter
            writeln!(file)?;
            writeln!(file, "---")?;
            writeln!(file, "tags: [{}]", all_tags.join(", "))?;
            writeln!(file, "---")?;
            writeln!(file)?;
        }
        // 已有 frontmatter → 不重复写（用户可在文件头手动管理）
    }

    for card in cards {
        writeln!(file, "<!-- card -->")?;
        writeln!(file, "Q:: {}", card.question)?;
        writeln!(file, "A:: {}", card.answer)?;
        writeln!(file, "<!-- card -->")?;
        writeln!(file)?;
    }

    Ok(())
}
