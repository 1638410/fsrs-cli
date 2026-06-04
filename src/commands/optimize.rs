use anyhow::Result;
use console::style;
use std::path::Path;

use crate::core::storage::Storage;

/// 用复习数据优化 FSRS 参数
pub fn run(base_path: &Path) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;

    let total = storage.total_cards()?;
    let reviews = storage.total_reviews()?;

    println!("{}", style("⚙️  FSRS 参数优化").cyan().bold());
    println!();

    // 显示当前状态
    println!("  当前状态:");
    println!("    总卡片数:   {}", total);
    println!("    总复习次数: {}", reviews);
    println!();

    // 显示默认参数
    println!("  默认 FSRS 参数:");
    println!("    期望保留率: 90%");
    println!("    调度算法:   FSRS v4 (rs-fsrs)");
    println!();

    if reviews < 10 {
        println!("{} 复习数据不足（需要至少 10 条记录）", style("⚠").yellow());
        println!("  继续复习以积累更多数据，然后重新运行此命令。");
    } else {
        println!("{} 已积累 {} 条复习记录", style("✓").green(), reviews);
        println!();

        // 计算当前的统计指标
        let due = storage.due_cards_count()?;
        let avg_retention = storage.avg_retention_in_period(30)?;

        println!("  当前指标（最近 30 天）:");
        println!("    待复习卡片: {}", due);
        println!("    平均可检索性: {:.1}%", avg_retention * 100.0);
        println!();

        // 简单的参数建议
        println!("  优化建议:");
        if avg_retention < 0.8 {
            println!("    - 可检索性较低，考虑缩短复习间隔");
            println!("    - 增加复习频率，特别是对困难卡片");
        } else if avg_retention > 0.95 {
            println!("    - 可检索性很高，可以适当延长复习间隔");
            println!("    - 当前复习可能过于频繁");
        } else {
            println!("    - 当前参数表现良好");
        }

        if due as f64 > total as f64 * 0.3 {
            println!("    - 待复习卡片较多，建议先完成复习再优化");
        }
    }

    println!();
    println!(
        "{} 注意: rs-fsrs 1.2 是轻量级实现，不包含完整的参数训练功能。",
        style("ℹ").blue()
    );
    println!("  如需完整的 FSRS 参数优化，请使用 fsrs crate 的训练功能。");
    println!("  当前使用默认参数，对大多数用户已经足够。");

    Ok(())
}
