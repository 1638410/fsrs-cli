use anyhow::{Context, Result};
use chrono::Utc;
use console::style;
use std::path::Path;
use uuid::Uuid;

use crate::core::hash::question_hash;
use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;
use crate::core::types::Card;
use crate::parser;
use crate::search::SearchEngine;

/// CSV 导入命令：直接解析 CSV 并插入数据库（不走临时 Markdown）
///
/// 之所以不转 .md 再 import：转 .md 后 source_file 会被错误地记录为临时 .md 路径。
/// 这里直接走 import 流程的核心逻辑，保证 source_file = 原始 CSV 路径。
pub fn run(base_path: &Path, source: &Path) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    if !source.is_file() {
        anyhow::bail!("CSV 文件不存在: {}", source.display());
    }
    if source.extension().map(|e| e != "csv").unwrap_or(true) {
        anyhow::bail!("不是 CSV 文件: {}", source.display());
    }

    let storage = Storage::open(&db_path)?;
    let index_path = base_path.join(".fsrs/tantivy");
    let search_engine = SearchEngine::open(&index_path)?;

    let cards = parser::parse_csv_file(source)
        .with_context(|| format!("解析 CSV 失败: {}", source.display()))?;

    if cards.is_empty() {
        println!("{} CSV 中未发现有效卡片", style("⚠").yellow());
        return Ok(());
    }

    let source_file = source
        .strip_prefix(base_path)
        .unwrap_or(source)
        .to_string_lossy()
        .to_string();

    println!(
        "{} 解析 {} 张卡片（来源: {}）",
        style("📥").blue(),
        cards.len(),
        source_file
    );

    // 收集已有卡片（按 card_index）
    let existing_cards = storage.get_cards_by_source(&source_file)?;
    let existing_map: std::collections::HashMap<i32, Card> = existing_cards
        .into_iter()
        .map(|c| (c.card_index, c))
        .collect();

    let scheduler = Scheduler::new();
    let mut new_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;

    for parsed in &cards {
        let new_hash = question_hash(&parsed.question);

        if let Some(existing) = existing_map.get(&parsed.card_index) {
            if existing.question_hash == new_hash {
                skipped_count += 1;
                continue;
            }

            let now = Utc::now();
            let updated_card = Card {
                id: existing.id.clone(),
                source_file: parsed.source_file.clone(),
                card_index: parsed.card_index,
                question: parsed.question.clone(),
                answer: parsed.answer.clone(),
                tags: parsed.tags.clone(),
                question_hash: new_hash,
                created_at: existing.created_at,
                updated_at: now,
            };

            storage.insert_card(&updated_card)?;
            let schedule = scheduler.new_card_schedule(&existing.id);
            storage.upsert_schedule(&schedule)?;

            updated_count += 1;
        } else {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now();

            let card = Card {
                id: id.clone(),
                source_file: parsed.source_file.clone(),
                card_index: parsed.card_index,
                question: parsed.question.clone(),
                answer: parsed.answer.clone(),
                tags: parsed.tags.clone(),
                question_hash: new_hash,
                created_at: now,
                updated_at: now,
            };

            storage.insert_card(&card)?;
            let _ = search_engine.index_card(&card);
            let schedule = scheduler.new_card_schedule(&id);
            storage.upsert_schedule(&schedule)?;

            new_count += 1;
        }
    }

    println!();
    println!("{} CSV 导入完成", style("✓").green().bold());
    println!("  新增: {} 张", new_count);
    println!("  更新: {} 张", updated_count);
    println!("  跳过: {} 张", skipped_count);
    println!("  来源: {}", source_file);

    Ok(())
}
