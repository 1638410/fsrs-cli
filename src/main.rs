use clap::{Parser, Subcommand};
use fsrs_lib::commands;
use fsrs_lib::AiCommands;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "fsrs-cli",
    about = "基于 FSRS 算法的命令行记忆管理工具",
    version,
    long_about = "fsrs-cli 是一个基于 Markdown、采用 FSRS 间隔重复算法的命令行记忆管理工具。\n支持从 Markdown 导入卡片、交互式复习、全文搜索、内容更新与参数优化。"
)]
struct Cli {
    /// 记忆库路径（默认当前目录）
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 初始化记忆库
    Init,

    /// 从 Markdown 文件或目录导入卡片
    Import {
        /// 要导入的文件或目录路径
        source: PathBuf,

        /// 导入后的目标路径（默认当前目录）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 从 tab 分隔的 CSV 文件导入（无表头）
    CsvImport {
        /// CSV 文件路径
        source: PathBuf,
    },

    /// 交互式制作卡组（多行答案 + Markdown，输出到指定文件后自动 import）
    Make {
        /// 输出 Markdown 文件路径（相对记忆库根目录）
        output: PathBuf,
    },

    /// 交互式复习到期卡片
    Review {
        /// 最多复习多少张
        #[arg(short, long, default_value_t = 20)]
        limit: i32,
    },

    /// 全文搜索卡片
    Search {
        /// 搜索关键词
        query: String,

        /// 最多返回多少条
        #[arg(short, long, default_value_t = 10)]
        limit: i32,
    },

    /// 打开编辑器修改卡片
    Edit {
        /// 卡片 ID 或文件路径
        target: String,
    },

    /// 更新文件中的卡片
    Update {
        /// 要更新的文件路径
        source: PathBuf,

        /// 即使问题变了也保留复习进度
        #[arg(long)]
        keep_schedule: bool,
    },

    /// 查看学习统计
    Stats {
        /// 统计最近 N 天
        #[arg(short = 'd', long)]
        period: Option<i32>,
    },

    /// 用复习数据优化 FSRS 参数
    Optimize,

    /// 审核 AI 生成的卡片草案
    #[cfg(feature = "ai")]
    ReviewDrafts,

    /// AI 辅助命令
    #[command(subcommand)]
    Ai(AiCommands),
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // 确定记忆库路径
    let base_path = cli
        .path
        .unwrap_or_else(|| std::env::current_dir().expect("无法获取当前目录"));

    match &cli.command {
        Commands::Init => {
            commands::init::run(&base_path)?;
        }
        Commands::Import { source, output } => {
            let output_path = output.as_ref().unwrap_or(&base_path);
            commands::import::run(source, output_path)?;
        }
        Commands::CsvImport { source } => {
            commands::csv_import::run(&base_path, source)?;
        }
        Commands::Make { output } => {
            commands::make::run(&base_path, output)?;
        }
        Commands::Review { limit } => {
            commands::review::run(&base_path, *limit)?;
        }
        Commands::Search { query, limit } => {
            commands::search::run(&base_path, query, *limit)?;
        }
        Commands::Edit { target } => {
            commands::edit::run(&base_path, target)?;
        }
        Commands::Update {
            source,
            keep_schedule,
        } => {
            commands::update::run(&base_path, source, *keep_schedule)?;
        }
        Commands::Stats { period } => {
            commands::stats::run(&base_path, *period)?;
        }
        Commands::Optimize => {
            commands::optimize::run(&base_path)?;
        }
        #[cfg(feature = "ai")]
        Commands::ReviewDrafts => {
            commands::review_drafts::run(&base_path)?;
        }
        Commands::Ai(ai_cmd) => {
            #[cfg(feature = "ai")]
            {
                commands::ai::run(&base_path, ai_cmd)?;
            }
            #[cfg(not(feature = "ai"))]
            {
                let _ = ai_cmd;
                anyhow::bail!("AI 功能需要启用 --features ai 编译");
            }
        }
    }

    Ok(())
}
