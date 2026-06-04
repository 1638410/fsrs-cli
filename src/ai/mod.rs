/// AI 辅助模块
///
/// TODO: 实现 AI 功能
/// - ai generate: 调用 LLM 生成卡片草案
/// - ai tag: 自动推荐标签
/// - ai health: 健康检查
/// - ai expand: 扩充答案
/// - ai review-suggest: 热度推荐
pub mod draft;
pub mod generate;
pub mod health;
pub mod review_suggest;
pub mod tag;
