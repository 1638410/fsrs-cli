use anyhow::Result;
use console::style;
use std::path::Path;

use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;
use crate::search::SearchEngine;

/// 全文搜索卡片
pub fn run(base_path: &Path, query: &str, limit: i32) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;
    let index_path = base_path.join(".fsrs/tantivy");

    // 尝试使用 Tantivy 搜索
    let results = match SearchEngine::open(&index_path) {
        Ok(engine) => engine.search(query, limit as usize).unwrap_or_default(),
        Err(_) => {
            // Tantivy 不可用，回退到简单搜索
            return simple_search(&storage, query, limit);
        }
    };

    if results.is_empty() {
        println!("{} 未找到匹配的卡片", style("⚠").yellow());
        return Ok(());
    }

    println!("{} 找到 {} 张匹配的卡片", style("🔍").blue(), results.len());
    println!();

    let scheduler = Scheduler::new();

    for result in &results {
        let card = match storage.get_card(&result.card_id)? {
            Some(c) => c,
            None => continue,
        };

        // 获取调度状态
        let due_info = match storage.get_schedule(&result.card_id)? {
            Some(s) => {
                let now = chrono::Utc::now();
                if s.due_date <= now {
                    style("DUE").red().bold().to_string()
                } else {
                    let days_left = (s.due_date - now).num_days();
                    format!("{}天后到期", days_left)
                }
            }
            None => style("New").cyan().to_string(),
        };

        // 计算可检索性
        let retrievability = match storage.get_schedule(&result.card_id)? {
            Some(s) => {
                let r = scheduler.get_retrievability(&s, chrono::Utc::now());
                format!(" 可检索性: {}%", (r * 100.0) as i32)
            }
            None => String::new(),
        };

        println!(
            "  {} [{}] {} {}{}",
            style("Q:").cyan(),
            card.card_index + 1,
            style(&card.question).white(),
            style(format!("({})", due_info)).dim(),
            style(retrievability).dim()
        );
        println!("    {} {}", style("来源:").dim(), card.source_file);
        if !card.tags.is_empty() {
            println!("    {} {}", style("标签:").dim(), card.tags.join(", "));
        }
        println!();
    }

    Ok(())
}

/// 简单 LIKE 搜索（回退方案）
fn simple_search(storage: &Storage, query: &str, limit: i32) -> Result<()> {
    let mut stmt = storage.conn.prepare(
        "SELECT id, source_file, card_index, question, answer, tags
         FROM cards
         WHERE question LIKE ?1 OR answer LIKE ?2
         LIMIT ?3",
    )?;

    let pattern = format!("%{}%", query);
    let cards = stmt
        .query_map(rusqlite::params![pattern, pattern, limit], |row| {
            let tags_str: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                tags,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if cards.is_empty() {
        println!("{} 未找到匹配的卡片", style("⚠").yellow());
        return Ok(());
    }

    println!(
        "{} 找到 {} 张匹配的卡片（简单搜索模式）",
        style("🔍").blue(),
        cards.len()
    );
    println!();

    for (_id, source_file, card_index, question, _answer, tags) in &cards {
        println!(
            "  {} [{}] {}",
            style("Q:").cyan(),
            card_index + 1,
            style(question).white(),
        );
        println!("    {} {}", style("来源:").dim(), source_file);
        if !tags.is_empty() {
            println!("    {} {}", style("标签:").dim(), tags.join(", "));
        }
        println!();
    }

    Ok(())
}
