use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 默认 AI 模型
    #[serde(default = "default_model")]
    pub model: String,

    /// 期望保留率 (0.0 - 1.0)
    #[serde(default = "default_retention")]
    pub desired_retention: f32,

    /// 默认每日复习上限
    #[serde(default = "default_daily_limit")]
    pub daily_limit: i32,

    /// AI 温度参数
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_retention() -> f32 {
    0.9
}

fn default_daily_limit() -> i32 {
    50
}

fn default_temperature() -> f32 {
    0.3
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            model: default_model(),
            desired_retention: default_retention(),
            daily_limit: default_daily_limit(),
            temperature: default_temperature(),
        }
    }
}

impl AppConfig {
    /// 从配置文件加载，不存在则返回默认值
    pub fn load(base_path: &Path) -> Self {
        let config_path = base_path.join(".fsrs/config.toml");
        if config_path.exists() {
            Self::from_file(&config_path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// 从文件加载配置
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
        let config: AppConfig = toml::from_str(&content).with_context(|| "配置文件格式错误")?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, base_path: &Path) -> Result<()> {
        let config_dir = base_path.join(".fsrs");
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self).with_context(|| "序列化配置失败")?;
        fs::write(&config_path, content)
            .with_context(|| format!("无法写入配置文件: {}", config_path.display()))?;

        Ok(())
    }
}

/// 元数据配置（写入 meta.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInfo {
    pub version: String,
    pub schema_version: i32,
    pub created_at: String,
    pub config: AppConfig,
}

impl Default for MetaInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaInfo {
    /// 创建新的元数据（使用默认值）
    pub fn new() -> Self {
        MetaInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: 1,
            created_at: Utc::now().to_rfc3339(),
            config: AppConfig::default(),
        }
    }

    /// 从 meta.json 加载
    pub fn load(base_path: &Path) -> Result<Self> {
        let path = base_path.join(".fsrs/meta.json");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("无法读取 meta.json: {}", path.display()))?;
        let meta: MetaInfo =
            serde_json::from_str(&content).with_context(|| "meta.json 格式错误")?;
        Ok(meta)
    }

    /// 保存到 meta.json
    pub fn save(&self, base_path: &Path) -> Result<()> {
        let path = base_path.join(".fsrs/meta.json");
        let content =
            serde_json::to_string_pretty(self).with_context(|| "序列化 meta.json 失败")?;
        fs::write(&path, content)
            .with_context(|| format!("无法写入 meta.json: {}", path.display()))?;
        Ok(())
    }
}
