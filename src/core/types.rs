use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// FSRS 过渡状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    New = 0,
    Learning = 1,
    Review = 2,
    Relearning = 3,
}

impl State {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => State::New,
            1 => State::Learning,
            2 => State::Review,
            3 => State::Relearning,
            _ => State::New,
        }
    }
}

/// 评分等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Rating {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            1 => Some(Rating::Again),
            2 => Some(Rating::Hard),
            3 => Some(Rating::Good),
            4 => Some(Rating::Easy),
            _ => None,
        }
    }
}

/// 一张知识卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub source_file: String,
    pub card_index: i32,
    pub question: String,
    pub answer: String,
    pub tags: Vec<String>,
    pub question_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 卡片的 FSRS 调度状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleState {
    pub card_id: String,
    pub difficulty: f64,
    pub stability: f64,
    pub last_review: Option<DateTime<Utc>>,
    pub due_date: DateTime<Utc>,
    pub elapsed_days: i64,
    pub scheduled_days: i64,
    pub reps: i32,
    pub lapses: i32,
    pub state: State,
}

/// 一条复习记录（不可变）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewLog {
    pub id: Option<i64>,
    pub card_id: String,
    pub rating: Rating,
    pub elapsed_days: i32,
    pub scheduled_days: i32,
    pub review_time: i64,
    pub reviewed_at: DateTime<Utc>,
}

/// AI 访问日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAccessLog {
    pub id: Option<i64>,
    pub card_id: String,
    pub access_time: DateTime<Utc>,
    pub reason: String,
    pub model_name: Option<String>,
}

/// AI 生成的卡片草案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCardDraft {
    pub id: String,
    pub question: String,
    pub answer: String,
    pub tags: Vec<String>,
    pub source_file: Option<String>,
    pub confidence: Option<f64>,
    pub source_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub state: DraftState,
}

/// 草稿状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DraftState {
    Draft,
    Approved,
    Rejected,
}

impl DraftState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DraftState::Draft => "draft",
            DraftState::Approved => "approved",
            DraftState::Rejected => "rejected",
        }
    }

    pub fn from_state_str(s: &str) -> Self {
        match s {
            "approved" => DraftState::Approved,
            "rejected" => DraftState::Rejected,
            _ => DraftState::Draft,
        }
    }
}

/// AI 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMemory {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
