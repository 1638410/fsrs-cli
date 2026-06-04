/// AI 热度推荐
use anyhow::Result;
use console::style;

use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;

/// 热度卡片
pub struct HotCard {
    pub card_id: String,
    pub question: String,
    pub access_count: i64,
    pub retrievability: f64,
    pub heat_score: f64,
}

/// 基于 AI 访问频率和 FSRS 可检索性，推荐需要复习的卡片
pub fn suggest_review(storage: &Storage, top_n: usize) -> Result<Vec<HotCard>> {
    let now = chrono::Utc::now();
    let scheduler = Scheduler::new();

    // 1. 查询所有卡片的 AI 访问次数
    let mut stmt = storage.conn.prepare(
        "SELECT c.id, c.question,
                COALESCE(a.access_count, 0) as access_count
         FROM cards c
         LEFT JOIN (
             SELECT card_id, COUNT(*) as access_count
             FROM ai_access
             GROUP BY card_id
         ) a ON c.id = a.card_id
         ORDER BY access_count DESC
         LIMIT ?1",
    )?;

    let rows: Vec<(String, String, i64)> = stmt
        .query_map(rusqlite::params![top_n as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut hot_cards = Vec::new();

    for (card_id, question, access_count) in rows {
        // 获取可检索性
        let retrievability = match storage.get_schedule(&card_id)? {
            Some(schedule) => scheduler.get_retrievability(&schedule, now),
            None => 1.0,
        };

        // 热度公式: 访问次数 × (1 - 可检索性)
        // 访问多且可检索性低的卡片 → 更需要复习
        let heat_score = (access_count as f64) * (1.0 - retrievability);

        hot_cards.push(HotCard {
            card_id,
            question,
            access_count,
            retrievability,
            heat_score,
        });
    }

    // 按热度降序排列
    hot_cards.sort_by(|a, b| b.heat_score.partial_cmp(&a.heat_score).unwrap());

    Ok(hot_cards)
}

/// 打印热度推荐
pub fn print_suggestions(cards: &[HotCard]) {
    println!("{}", style("🔥 AI 热度推荐").cyan().bold());
    println!();

    if cards.is_empty() {
        println!(
            "{} 没有推荐的卡片（需要先通过 AI 生成或查询卡片）",
            style("ℹ").blue()
        );
        return;
    }

    for (i, card) in cards.iter().enumerate() {
        let retrievability_pct = card.retrievability * 100.0;
        println!(
            "  {}. [{}] 热度: {:.1} | 访问: {}次 | 可检索: {:.0}%",
            style(i + 1).cyan(),
            &card.card_id[..8],
            style(format!("{:.1}", card.heat_score)).yellow().bold(),
            card.access_count,
            retrievability_pct,
        );
        println!("     {}", &card.question);
        println!();
    }

    println!("{} 热度 = 访问次数 × (1 - 可检索性)", style("💡").dim());
    println!("    热度越高 = 被 AI 访问过但你尚未掌握的卡片");
}
