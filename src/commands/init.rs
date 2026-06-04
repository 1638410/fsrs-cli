use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::path::Path;

use crate::config::MetaInfo;
use crate::core::storage::Storage;
use crate::search::SearchEngine;

/// 初始化记忆库：创建 .fsrs/ 目录和数据库
pub fn run(base_path: &Path) -> Result<()> {
    let fsrs_dir = base_path.join(".fsrs");

    if fsrs_dir.exists() {
        println!(
            "{} 记忆库已存在于 {}",
            style("⚠").yellow(),
            fsrs_dir.display()
        );
        return Ok(());
    }

    // 创建目录
    fs::create_dir_all(&fsrs_dir)
        .with_context(|| format!("无法创建目录: {}", fsrs_dir.display()))?;

    // 创建数据库
    let db_path = fsrs_dir.join("memory.db");
    let _storage = Storage::open(&db_path)?;

    // 创建 tantivy 索引（同时创建目录与空索引）
    let index_dir = fsrs_dir.join("tantivy");
    let _search_engine = SearchEngine::open(&index_dir)?;

    // 写入 meta.json
    let meta = MetaInfo::new();
    meta.save(base_path)?;

    println!(
        "{} 记忆库已初始化于 {}",
        style("✓").green().bold(),
        fsrs_dir.display()
    );
    println!("  数据库:   {}", db_path.display());
    println!("  索引:     {}", index_dir.display());
    println!("  元数据:   {}", fsrs_dir.join("meta.json").display());

    Ok(())
}
