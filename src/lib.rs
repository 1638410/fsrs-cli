pub mod commands;
pub mod config;
pub mod core;
pub mod parser;
pub mod search;
pub mod tui;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum AiCommands {
    /// 用 AI 生成卡片草案
    #[command(long_about = "用 AI 根据给定的 source 生成卡片草案。\n\
        支持输入：\n  \
        - 主题词（如 \"Rust 生命周期\"）：生成相关 Q&A 卡片\n  \
        - 文件路径：解析文本内容生成\n  \
        - URL（需启用 --features pdf）：抓取并解析\n\n\
        输出：草稿卡片入库，需用 'fsrs-cli review-drafts' 审核通过后才能进入复习流程。\n\
        不会直接修改 schedules / reviews 表。")]
    Generate {
        /// 主题、文件或 URL
        source: String,

        /// 从 PDF 导入（需要 --features pdf）
        #[arg(long)]
        pdf: bool,
    },

    /// 为卡片自动推荐标签
    #[command(long_about = "为卡片自动推荐标签。\n\
        两种模式：\n  \
        - 指定 card_id：为该单张卡片推荐标签\n  \
        - 使用 --batch：批量处理所有无标签的卡片（card_id 忽略）\n\n\
        输出：推荐的标签直接写入数据库的 tags 字段。\n\
        已有标签会被覆盖（建议先 backup）。")]
    Tag {
        /// 卡片 ID
        card_id: Option<String>,

        /// 批量处理所有未标记的卡片
        #[arg(long)]
        batch: bool,
    },

    /// 扫描知识库，输出内容质量报告
    #[command(long_about = "扫描整个知识库，输出内容质量报告（只读，不修改数据）。\n\
        检查维度：\n  \
        - 答案过短（< 5 字符）的卡片\n  \
        - Jaccard 相似度 > 0.8 的重复卡片\n  \
        - 长期未复习的 stale 卡片\n\n\
        输出：分级报告（严重 / 警告 / 提示），含具体卡片 ID 和原因。")]
    Health,

    /// 为一句话卡片补充详细答案
    #[command(long_about = "为一句话卡片补充详细答案。\n\
        输入：card_id\n\
        输出：用 AI 生成的更长答案替换原 answer 字段。\n\
        注意：操作不可逆，建议先 backup 原文件或用 'edit' 命令查看原文。")]
    Expand {
        /// 卡片 ID
        card_id: String,
    },

    /// 基于 AI 访问频率，列出推荐复习的卡片
    #[command(long_about = "基于 AI 访问频率，推荐优先复习的卡片。\n\
        算法：heat = access_count × (1 - retrievability)\n  \
        其中 retrievability 来自 FSRS 当前状态。\n\n\
        输出：按 heat 降序排列的卡片列表（含 ID、问题摘要、heat 值），不含复习数据修改。")]
    ReviewSuggest,
}

#[cfg(feature = "ai")]
pub mod ai;
