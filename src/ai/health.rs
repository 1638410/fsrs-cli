/// AI 健康检查
use anyhow::Result;
use console::style;

use crate::core::storage::Storage;

/// 健康检查报告
pub struct HealthReport {
    pub short_answers: Vec<(String, String)>, // (card_id, question)
    pub duplicates: Vec<(String, String, f64)>, // (card_id1, card_id2, similarity)
    pub stale_cards: Vec<(String, String)>,   // (card_id, question)
}

/// 扫描知识库，输出内容质量报告
pub fn run_health_check(storage: &Storage) -> Result<HealthReport> {
    let mut report = HealthReport {
        short_answers: Vec::new(),
        duplicates: Vec::new(),
        stale_cards: Vec::new(),
    };

    // 获取所有卡片
    let mut stmt = storage
        .conn
        .prepare("SELECT id, question, answer FROM cards ORDER BY source_file, card_index")?;

    let cards: Vec<(String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 检查答案过短
    for (id, question, answer) in &cards {
        if answer.len() < 20 {
            report.short_answers.push((id.clone(), question.clone()));
        }
    }

    // 简单的重复检测（基于问题相似度）
    for i in 0..cards.len() {
        for j in (i + 1)..cards.len() {
            let similarity = jaccard_similarity(&cards[i].1, &cards[j].1);
            if similarity > 0.8 {
                report
                    .duplicates
                    .push((cards[i].0.clone(), cards[j].0.clone(), similarity));
            }
        }
    }

    // 检查长时间未复习的卡片
    let now = chrono::Utc::now();
    let mut stmt = storage.conn.prepare(
        "SELECT c.id, c.question, s.due_date, s.last_review
         FROM cards c
         LEFT JOIN schedule s ON c.id = s.card_id
         WHERE s.last_review IS NOT NULL AND s.due_date < ?1",
    )?;

    let stale: Vec<(String, String)> = stmt
        .query_map([now.to_rfc3339()], |row| {
            let due_str: String = row.get(2)?;
            let due = chrono::DateTime::parse_from_rfc3339(&due_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| now);
            let days_overdue = (now - due).num_days();
            if days_overdue > 30 {
                Ok(Some((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            } else {
                Ok(None)
            }
        })?
        .filter_map(|r| r.ok().flatten())
        .collect();

    report.stale_cards = stale;

    Ok(report)
}

/// 打印健康报告
pub fn print_report(report: &HealthReport) {
    println!("{}", style("🏥 知识库健康检查报告").cyan().bold());
    println!();

    if report.short_answers.is_empty()
        && report.duplicates.is_empty()
        && report.stale_cards.is_empty()
    {
        println!("{} 知识库状态良好，未发现明显问题", style("✓").green());
        return;
    }

    if !report.short_answers.is_empty() {
        println!(
            "{} 答案过短（<20字符）：{} 张",
            style("⚠").yellow(),
            report.short_answers.len()
        );
        for (id, q) in &report.short_answers {
            println!("  - [{}] {}", &id[..8], q);
        }
        println!();
    }

    if !report.duplicates.is_empty() {
        println!(
            "{} 高度相似的卡片：{} 对",
            style("⚠").yellow(),
            report.duplicates.len()
        );
        for (id1, id2, sim) in &report.duplicates {
            println!(
                "  - [{}] ↔ [{}] 相似度: {:.0}%",
                &id1[..8],
                &id2[..8],
                sim * 100.0
            );
        }
        println!();
    }

    if !report.stale_cards.is_empty() {
        println!(
            "{} 长时间未复习（>30天）：{} 张",
            style("⚠").yellow(),
            report.stale_cards.len()
        );
        for (id, q) in &report.stale_cards {
            println!("  - [{}] {}", &id[..8], q);
        }
        println!();
    }
}

/// Jaccard 相似度计算
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let a_chars: std::collections::HashSet<char> = a.chars().collect();
    let b_chars: std::collections::HashSet<char> = b.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}
