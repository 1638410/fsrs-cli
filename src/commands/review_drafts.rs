use anyhow::Result;
use console::style;
use std::path::Path;

use crate::core::storage::Storage;

/// 审核 AI 生成的卡片草案
pub fn run(base_path: &Path) -> Result<()> {
    let db_path = base_path.join(".fsrs/memory.db");
    if !db_path.exists() {
        anyhow::bail!("记忆库未初始化，请先运行 {}", style("fsrs-cli init").cyan());
    }

    let storage = Storage::open(&db_path)?;

    #[cfg(feature = "ai")]
    {
        let drafts = crate::ai::draft::get_pending_drafts(&storage)?;

        if drafts.is_empty() {
            println!("{} 没有待审核的草案", style("✓").green());
            return Ok(());
        }

        println!("{} {} 张草案待审核", style("📋").blue(), drafts.len());
        println!();

        let mut approved = 0;
        let mut rejected = 0;

        for draft in &drafts {
            println!("{}", "─".repeat(40));
            println!("  {} [{}]", style("Q:").cyan().bold(), &draft.id[..8]);
            println!("  {}", style(&draft.question).white());
            println!();
            println!("  {}", style("A:").green().bold());
            println!("  {}", style(&draft.answer).green());

            if let Some(source) = &draft.source_file {
                println!();
                println!("    {} {}", style("来源:").dim(), source);
            }
            println!();

            println!("  操作: [y] 通过  [n] 拒绝  [q] 退出  [Enter] 跳过");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            match input.trim() {
                "y" | "Y" => {
                    let card = crate::ai::draft::approve_draft(&storage, &draft.id)?;
                    println!(
                        "  {} 已转为正式卡片 [{}]",
                        style("✓").green(),
                        &card.id[..8]
                    );
                    approved += 1;
                }
                "n" | "N" => {
                    crate::ai::draft::reject_draft(&storage, &draft.id)?;
                    println!("  {} 已拒绝", style("✗").red());
                    rejected += 1;
                }
                "q" | "Q" => {
                    println!();
                    println!("已审核 {} 张，退出。", approved + rejected);
                    break;
                }
                _ => {
                    println!("  跳过");
                }
            }
            println!();
        }

        println!(
            "{} 审核完成: 通过 {}, 拒绝 {}",
            style("✓").green().bold(),
            approved,
            rejected
        );
    }

    #[cfg(not(feature = "ai"))]
    {
        println!("{} AI 功能需要启用 --features ai 编译", style("⚠").yellow());
    }

    Ok(())
}
