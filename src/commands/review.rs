use anyhow::Result;
use chrono::Utc;
use console::style;
use std::path::Path;
use std::time::Instant;

use crate::config::AppConfig;
use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;
use crate::core::types::{AiAccessLog, Rating, ReviewLog};

/// 交互式复习到期卡片
pub fn run(base_path: &Path, limit: i32) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;
    let config = AppConfig::load(base_path);
    let scheduler = Scheduler::new();

    // 强制 daily_limit：取 min(user_limit, daily_limit - today_reviewed)
    let today_reviewed = storage.reviews_in_period(1)? as i32;
    let remaining = (config.daily_limit - today_reviewed).max(0);
    let effective_limit = limit.min(remaining);

    if effective_limit <= 0 {
        println!(
            "{} 今日已达每日上限 ({}/{})，请明天再来",
            style("⚠").yellow(),
            today_reviewed,
            config.daily_limit
        );
        return Ok(());
    }

    let due_card_ids = storage.get_due_cards(effective_limit)?;

    if due_card_ids.is_empty() {
        println!("{} 没有到期的卡片", style("✓").green());
        return Ok(());
    }

    println!("{} {} 张卡片待复习 (今日已复习 {}/{}，上限 {})",
        style("📚").blue(),
        due_card_ids.len(),
        today_reviewed,
        config.daily_limit,
        effective_limit
    );
    println!();

    let mut reviewed = 0;

    for card_id in &due_card_ids {
        let card = match storage.get_card(card_id)? {
            Some(c) => c,
            None => continue,
        };

        // 获取当前调度状态
        let schedule = storage
            .get_schedule(card_id)?
            .unwrap_or_else(|| scheduler.new_card_schedule(card_id));

        // 显示可检索性
        let retrievability = scheduler.get_retrievability(&schedule, Utc::now());
        let retrievability_pct = (retrievability * 100.0) as i32;

        // 显示问题
        println!("{}", "─".repeat(40));
        println!(
            "  {} [{}]  可检索性: {}%",
            style("Q:").cyan().bold(),
            card.card_index + 1,
            retrievability_pct
        );
        println!("  {}", style(&card.question).white());
        println!();

        // 记录开始时间
        let review_start = Instant::now();

        // 等待用户按键显示答案
        println!("  {} 按 Enter 显示答案...", style("→").dim());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // 显示答案
        println!();
        println!("  {}", style("A:").green().bold());
        println!("  {}", style(&card.answer).green());
        println!();

        // 评分
        println!("  评分: [1] 忘了  [2] 困难  [3] 一般  [4] 简单  [q] 退出");
        let mut rating_input = String::new();
        std::io::stdin().read_line(&mut rating_input)?;

        let rating_input = rating_input.trim();
        if rating_input == "q" || rating_input == "Q" {
            println!();
            println!("已复习 {} 张卡片，进度已保存。", reviewed);
            break;
        }

        let rating = match rating_input.parse::<i32>() {
            Ok(1) => Rating::Again,
            Ok(2) => Rating::Hard,
            Ok(3) => Rating::Good,
            Ok(4) => Rating::Easy,
            _ => {
                println!("  {} 无效评分，跳过", style("⚠").yellow());
                continue;
            }
        };

        // 使用 FSRS 调度器计算新状态
        let new_schedule = scheduler
            .review_card(&schedule, rating)
            .map_err(|e| anyhow::anyhow!(e))?;

        // 计算 elapsed_days 和 scheduled_days
        let elapsed_days = schedule
            .last_review
            .map(|lr| (Utc::now() - lr).num_days())
            .unwrap_or(0);
        let scheduled_days = new_schedule.scheduled_days;

        // 计算实际复习耗时（毫秒）
        let review_time_ms = review_start.elapsed().as_millis() as i64;

        // 保存新的调度状态
        storage.upsert_schedule(&new_schedule)?;

        // 记录复习日志
        let log = ReviewLog {
            id: None,
            card_id: card_id.clone(),
            rating,
            elapsed_days: elapsed_days as i32,
            scheduled_days: scheduled_days as i32,
            review_time: review_time_ms,
            reviewed_at: Utc::now(),
        };
        storage.insert_review(&log)?;

        // 记录访问日志
        let access_log = AiAccessLog {
            id: None,
            card_id: card_id.clone(),
            access_time: Utc::now(),
            reason: format!("review:{:?}", rating),
            model_name: None,
        };
        let _ = storage.insert_ai_access(&access_log);

        // 显示下次复习时间
        let next_review = new_schedule.due_date;
        let days_until = (next_review - Utc::now()).num_days();
        let next_info = if days_until <= 1 {
            "明天".to_string()
        } else {
            format!("{}天后", days_until)
        };

        reviewed += 1;
        println!(
            "  {} 已评分 → 下次复习: {}",
            style("✓").green(),
            style(next_info).cyan()
        );
        println!();
    }

    println!(
        "{} 复习完成，共复习 {} 张卡片",
        style("✓").green().bold(),
        reviewed
    );

    Ok(())
}
