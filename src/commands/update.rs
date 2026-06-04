use anyhow::{Context, Result};
use chrono::Utc;
use console::style;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::core::hash::question_hash;
use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;
use crate::core::types::Card;
use crate::parser;
use crate::search::SearchEngine;

/// 更新文件中的卡片
pub fn run(source: &Path, base_path: &Path, keep_schedule: bool) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;
    let index_path = base_path.join(".fsrs/tantivy");
    let search_engine = SearchEngine::open(&index_path)?;

    let files = collect_markdown_files(source)?;
    if files.is_empty() {
        println!("{} 未找到 Markdown 文件", style("⚠").yellow());
        return Ok(());
    }

    println!(
        "{} 找到 {} 个 Markdown 文件",
        style("📁").blue(),
        files.len()
    );

    let mut total_new = 0;
    let mut total_updated = 0;
    let mut total_skipped = 0;

    for file in &files {
        let result = update_file(&storage, &search_engine, file, base_path, keep_schedule);
        match result {
            Ok((new, updated, skipped)) => {
                total_new += new;
                total_updated += updated;
                total_skipped += skipped;
            }
            Err(e) => {
                eprintln!("{} 更新 {} 失败: {}", style("✗").red(), file.display(), e);
            }
        }
    }

    println!();
    println!(
        "{} 更新完成 (keep_schedule={})",
        style("✓").green().bold(),
        keep_schedule
    );
    println!("  新增: {} 张", total_new);
    println!("  更新: {} 张", total_updated);
    println!("  跳过: {} 张", total_skipped);

    Ok(())
}

/// 收集路径下所有 Markdown 文件
fn collect_markdown_files(source: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if source.is_file() {
        if source.extension().map(|e| e == "md").unwrap_or(false) {
            files.push(source.to_path_buf());
        } else {
            anyhow::bail!("不是 Markdown 文件: {}", source.display());
        }
    } else if source.is_dir() {
        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path.to_path_buf());
            }
        }
        files.sort();
    } else {
        anyhow::bail!("路径不存在: {}", source.display());
    }

    Ok(files)
}

/// 更新单个文件，返回 (新增数, 更新数, 跳过数)
fn update_file(
    storage: &Storage,
    search_engine: &SearchEngine,
    file_path: &Path,
    base_path: &Path,
    keep_schedule: bool,
) -> Result<(i32, i32, i32)> {
    let source_file = file_path
        .strip_prefix(base_path)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let parsed_cards = parser::parse_file(file_path)
        .with_context(|| format!("解析失败: {}", file_path.display()))?;

    let existing_cards = storage.get_cards_by_source(&source_file)?;
    let existing_map: std::collections::HashMap<i32, Card> = existing_cards
        .into_iter()
        .map(|c| (c.card_index, c))
        .collect();

    let scheduler = Scheduler::new();
    let mut new_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;

    for parsed in &parsed_cards {
        let new_hash = question_hash(&parsed.question);

        if let Some(existing) = existing_map.get(&parsed.card_index) {
            // 已存在，检查是否需要更新
            if existing.question_hash == new_hash {
                skipped_count += 1;
                continue;
            }

            // 问题变了，更新卡片
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

            // 根据 keep_schedule 决定是否重置调度
            if !keep_schedule {
                let schedule = scheduler.new_card_schedule(&existing.id);
                storage.upsert_schedule(&schedule)?;
            }

            // 更新 Tantivy 索引
            let _ = search_engine.remove_card(&existing.id);
            let _ = search_engine.index_card(&updated_card);

            updated_count += 1;
        } else {
            // 新卡片
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

            // 索引到 Tantivy
            let _ = search_engine.index_card(&card);

            // 初始化调度状态
            let schedule = scheduler.new_card_schedule(&id);
            storage.upsert_schedule(&schedule)?;

            new_count += 1;
        }
    }

    if new_count > 0 || updated_count > 0 {
        println!(
            "  {} {} - 新增 {}, 更新 {}",
            style("✓").green(),
            file_path.display(),
            new_count,
            updated_count
        );
    }

    Ok((new_count, updated_count, skipped_count))
}
