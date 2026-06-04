use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::core::storage::Storage;
use crate::search::SearchEngine;

/// 打开编辑器修改卡片
pub fn run(base_path: &Path, target: &str) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;
    let index_path = base_path.join(".fsrs/tantivy");
    let search_engine = SearchEngine::open(&index_path)?;

    // 查找卡片：支持 ID 前缀匹配或问题关键词搜索
    let card = find_card(&storage, target)?;

    println!("{} 编辑卡片 [{}]", style("✏️").blue(), &card.id[..8]);
    println!("  Q: {}", card.question);
    println!();

    // 创建临时文件
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("fsrs-edit-{}.md", &card.id[..8]));

    // 写入临时文件内容
    let content = format!(
        "# 问题\n{}\n\n# 答案\n{}\n\n# 标签\n{}\n",
        card.question,
        card.answer,
        card.tags.join(", ")
    );
    fs::write(&temp_file, &content).context("无法创建临时文件")?;

    // 打开编辑器
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(&editor)
        .arg(temp_file.to_str().unwrap())
        .status()
        .with_context(|| format!("无法打开编辑器: {}", editor))?;

    if !status.success() {
        fs::remove_file(&temp_file).ok();
        anyhow::bail!("编辑器退出，未保存");
    }

    // 读取编辑后的内容
    let edited = fs::read_to_string(&temp_file).context("无法读取编辑后的文件")?;
    fs::remove_file(&temp_file).ok();

    // 解析编辑后的内容
    let (new_question, new_answer, new_tags) = parse_edited_content(&edited);

    if new_question.is_empty() {
        anyhow::bail!("问题不能为空");
    }

    // 更新卡片
    let now = chrono::Utc::now();
    let mut updated_card = card.clone();
    updated_card.question = new_question;
    updated_card.answer = new_answer;
    updated_card.tags = new_tags;
    updated_card.updated_at = now;
    updated_card.question_hash = crate::core::hash::question_hash(&updated_card.question);

    storage.insert_card(&updated_card)?;

    // 同步 Tantivy 索引
    let _ = search_engine.remove_card(&card.id);
    let _ = search_engine.index_card(&updated_card);

    // 如果问题变了，重置调度
    if updated_card.question_hash != card.question_hash {
        let scheduler = crate::core::scheduler::Scheduler::new();
        let schedule = scheduler.new_card_schedule(&card.id);
        storage.upsert_schedule(&schedule)?;
        println!("{} 问题已变更，调度已重置", style("⚠").yellow());
    }

    println!("{} 卡片已更新", style("✓").green());
    println!("  Q: {}", updated_card.question);
    println!("  A: {}", updated_card.answer);

    Ok(())
}

/// 根据 ID 前缀或关键词查找卡片
fn find_card(storage: &Storage, target: &str) -> Result<crate::core::types::Card> {
    // 先尝试 ID 前缀匹配
    if let Some(card) = storage.get_card(target)? {
        return Ok(card);
    }

    // 搜索匹配的卡片
    let mut stmt = storage.conn.prepare(
        "SELECT id, source_file, card_index, question, answer, tags, question_hash, created_at, updated_at
         FROM cards WHERE id LIKE ?1 OR question LIKE ?1 OR answer LIKE ?1
         LIMIT 1",
    )?;

    let pattern = format!("%{}%", target);
    let mut rows = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok(crate::core::types::Card {
            id: row.get(0)?,
            source_file: row.get(1)?,
            card_index: row.get(2)?,
            question: row.get(3)?,
            answer: row.get(4)?,
            tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
            question_hash: row.get(6)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    })?;

    match rows.next() {
        Some(row) => Ok(row?),
        None => anyhow::bail!("未找到匹配的卡片: {}", target),
    }
}

/// 解析编辑后的内容
fn parse_edited_content(content: &str) -> (String, String, Vec<String>) {
    let mut question = String::new();
    let mut answer = String::new();
    let mut tags = Vec::new();

    let mut section = "";
    for line in content.lines() {
        if line.starts_with("# 问题") {
            section = "question";
            continue;
        } else if line.starts_with("# 答案") {
            section = "answer";
            continue;
        } else if line.starts_with("# 标签") {
            section = "tags";
            continue;
        }

        match section {
            "question" => {
                if !question.is_empty() {
                    question.push('\n');
                }
                question.push_str(line);
            }
            "answer" => {
                if !answer.is_empty() {
                    answer.push('\n');
                }
                answer.push_str(line);
            }
            "tags" => {
                for tag in line.split(',') {
                    let tag = tag.trim().to_string();
                    if !tag.is_empty() {
                        tags.push(tag);
                    }
                }
            }
            _ => {}
        }
    }

    // 去除首尾空白
    let question = question.trim().to_string();
    let answer = answer.trim().to_string();

    (question, answer, tags)
}
