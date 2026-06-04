use anyhow::Result;
use console::style;
use std::path::Path;

use crate::core::storage::Storage;

/// 查看学习统计
pub fn run(base_path: &Path, period: Option<i32>) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;

    let total = storage.total_cards()?;
    let due = storage.due_cards_count()?;
    let reviews = storage.total_reviews()?;

    println!("{}", style("📊 学习统计").cyan().bold());
    println!();
    println!("  总卡片数:     {}", total);
    println!("  待复习:       {}", due);
    println!("  总复习次数:   {}", reviews);

    if let Some(days) = period {
        println!();
        println!("{} 最近 {} 天", style("📈").blue(), days);

        let period_reviews = storage.reviews_in_period(days)?;
        let new_cards = storage.new_cards_in_period(days)?;
        let avg_retention = storage.avg_retention_in_period(days)?;

        println!("  新增卡片:     {}", new_cards);
        println!("  复习次数:     {}", period_reviews);
        println!("  平均可检索性: {:.1}%", avg_retention * 100.0);

        // 每日复习统计
        let daily = storage.daily_review_counts(days)?;
        if !daily.is_empty() {
            println!();
            println!("  每日复习:");
            for (day, count) in &daily {
                println!("    {}: {} 次", day, count);
            }
        }
    }

    Ok(())
}
