use anyhow::Result;
use chrono::Utc;
use console::style;
use std::path::Path;

use crate::AiCommands;

/// AI 辅助命令入口
pub fn run(base_path: &Path, cmd: &AiCommands) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = crate::core::storage::Storage::open(&db_path)?;

    match cmd {
        AiCommands::Generate { source, pdf } => {
            if *pdf {
                println!(
                    "{} PDF 导入功能需要启用 --features pdf",
                    style("⚠").yellow()
                );
                return Ok(());
            }

            println!("{} 正在分析 {} ...", style("🤖").blue(), source);

            // 创建 tokio 运行时
            let rt = tokio::runtime::Runtime::new()?;
            let qa_list = rt.block_on(async {
                // 判断是文件还是主题
                let path = std::path::Path::new(source);
                if path.is_file() {
                    crate::ai::generate::generate_from_file(source).await
                } else {
                    crate::ai::generate::generate_from_topic(source).await
                }
            })?;

            if qa_list.is_empty() {
                println!("{} 未生成任何卡片", style("⚠").yellow());
                return Ok(());
            }

            // 存入草稿
            let mut count = 0;
            for qa in &qa_list {
                crate::ai::draft::create_draft(
                    &storage,
                    &qa.question,
                    &qa.answer,
                    Some(source),
                    None,
                    None,
                )?;
                count += 1;
            }

            println!("{} 生成了 {} 张卡片草案", style("✓").green().bold(), count);
            println!("  使用 {} 审核草稿", style("fsrs-cli review-drafts").cyan());
        }

        AiCommands::Tag { card_id, batch } => {
            let rt = tokio::runtime::Runtime::new()?;

            if *batch {
                println!("{} 批量生成标签...", style("🤖").blue());
                // 获取所有没有标签的卡片
                let all_cards = storage.get_cards_without_tags()?;
                if all_cards.is_empty() {
                    println!("{} 没有需要标签的卡片", style("✓").green());
                    return Ok(());
                }

                println!("  找到 {} 张需要标签的卡片", all_cards.len());

                let mut tagged = 0;
                for card in &all_cards {
                    let tags = rt.block_on(async {
                        crate::ai::tag::suggest_tags(&card.question, &card.answer).await
                    })?;

                    if !tags.is_empty() {
                        let mut updated_card = card.clone();
                        updated_card.tags = tags.clone();
                        updated_card.updated_at = Utc::now();
                        storage.insert_card(&updated_card)?;

                        // 记录访问
                        let access_log = crate::core::types::AiAccessLog {
                            id: None,
                            card_id: card.id.clone(),
                            access_time: Utc::now(),
                            reason: "ai:tag:batch".to_string(),
                            model_name: Some("gpt-4o-mini".to_string()),
                        };
                        let _ = storage.insert_ai_access(&access_log);

                        println!(
                            "  [{}] {} → {}",
                            &card.id[..8],
                            &card.question[..card.question.len().min(30)],
                            tags.join(", ")
                        );
                        tagged += 1;
                    }
                }

                println!(
                    "{} 批量标签完成，标记了 {} 张卡片",
                    style("✓").green().bold(),
                    tagged
                );
            } else if let Some(id) = card_id {
                let card = storage
                    .get_card(id)?
                    .ok_or_else(|| anyhow::anyhow!("卡片不存在: {}", id))?;

                println!(
                    "{} 为卡片生成标签: {}",
                    style("🤖").blue(),
                    &card.question[..card.question.len().min(50)]
                );

                let tags = rt.block_on(async {
                    crate::ai::tag::suggest_tags(&card.question, &card.answer).await
                })?;

                if tags.is_empty() {
                    println!("{} 未生成标签", style("⚠").yellow());
                } else {
                    // 更新卡片标签
                    let mut updated_card = card.clone();
                    updated_card.tags = tags.clone();
                    updated_card.updated_at = Utc::now();
                    storage.insert_card(&updated_card)?;

                    // 记录 AI 访问
                    let access_log = crate::core::types::AiAccessLog {
                        id: None,
                        card_id: card.id.clone(),
                        access_time: Utc::now(),
                        reason: "ai:tag".to_string(),
                        model_name: Some("gpt-4o-mini".to_string()),
                    };
                    let _ = storage.insert_ai_access(&access_log);

                    println!("{} 推荐标签: {}", style("✓").green(), tags.join(", "));
                }
            }
        }

        AiCommands::Health => {
            let report = crate::ai::health::run_health_check(&storage)?;
            crate::ai::health::print_report(&report);
        }

        AiCommands::Expand { card_id } => {
            let card = storage
                .get_card(card_id)?
                .ok_or_else(|| anyhow::anyhow!("卡片不存在: {}", card_id))?;

            println!(
                "{} 为卡片补充详细答案: {}",
                style("🤖").blue(),
                &card.question[..card.question.len().min(50)]
            );

            let rt = tokio::runtime::Runtime::new()?;
            let qa_list = rt.block_on(async {
                let prompt = format!(
                    "请为以下问题提供一个详细、准确的答案：\n\n问题：{}\n\n当前答案：{}\n\n请提供一个更详细的版本：",
                    card.question, card.answer
                );
                crate::ai::generate::generate_from_text(&prompt).await
            })?;

            if let Some(qa) = qa_list.first() {
                let mut updated_card = card.clone();
                updated_card.answer = qa.answer.clone();
                updated_card.updated_at = Utc::now();
                storage.insert_card(&updated_card)?;

                // 记录 AI 访问
                let access_log = crate::core::types::AiAccessLog {
                    id: None,
                    card_id: card.id.clone(),
                    access_time: Utc::now(),
                    reason: "ai:expand".to_string(),
                    model_name: Some("gpt-4o-mini".to_string()),
                };
                let _ = storage.insert_ai_access(&access_log);

                println!("{} 答案已更新", style("✓").green());
                println!("  新答案: {}", qa.answer);
            } else {
                println!("{} 未能生成更详细的答案", style("⚠").yellow());
            }
        }

        AiCommands::ReviewSuggest => {
            let hot_cards = crate::ai::review_suggest::suggest_review(&storage, 10)?;
            crate::ai::review_suggest::print_suggestions(&hot_cards);
        }
    }

    Ok(())
}
