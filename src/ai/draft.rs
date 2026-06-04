/// AI 卡片草案管理
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::core::hash::question_hash;
use crate::core::scheduler::Scheduler;
use crate::core::storage::Storage;
use crate::core::types::{AiCardDraft, Card, DraftState};

/// 创建新的 AI 卡片草案
pub fn create_draft(
    storage: &Storage,
    question: &str,
    answer: &str,
    source_file: Option<&str>,
    confidence: Option<f64>,
    source_prompt: Option<&str>,
) -> Result<AiCardDraft> {
    let draft = AiCardDraft {
        id: Uuid::new_v4().to_string(),
        question: question.to_string(),
        answer: answer.to_string(),
        tags: Vec::new(),
        source_file: source_file.map(|s| s.to_string()),
        confidence,
        source_prompt: source_prompt.map(|s| s.to_string()),
        created_at: Utc::now(),
        state: DraftState::Draft,
    };

    storage.conn.execute(
        "INSERT INTO ai_drafts (id, question, answer, tags, source_file, confidence, source_prompt, created_at, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            draft.id,
            draft.question,
            draft.answer,
            serde_json::to_string(&draft.tags).unwrap_or_default(),
            draft.source_file,
            draft.confidence,
            draft.source_prompt,
            draft.created_at.to_rfc3339(),
            draft.state.as_str(),
        ],
    )?;

    Ok(draft)
}

/// 审核草稿：通过并转为正式卡片
pub fn approve_draft(storage: &Storage, draft_id: &str) -> Result<Card> {
    let draft =
        get_draft(storage, draft_id)?.ok_or_else(|| anyhow::anyhow!("草稿不存在: {}", draft_id))?;

    let now = Utc::now();
    let card_id = Uuid::new_v4().to_string();

    // 创建正式卡片
    let card = Card {
        id: card_id.clone(),
        source_file: draft
            .source_file
            .clone()
            .unwrap_or_else(|| "ai-generated".to_string()),
        card_index: 0,
        question: draft.question.clone(),
        answer: draft.answer.clone(),
        tags: draft.tags.clone(),
        question_hash: question_hash(&draft.question),
        created_at: now,
        updated_at: now,
    };

    storage.insert_card(&card)?;

    // 初始化调度状态
    let scheduler = Scheduler::new();
    let schedule = scheduler.new_card_schedule(&card_id);
    storage.upsert_schedule(&schedule)?;

    // 更新草稿状态
    storage.conn.execute(
        "UPDATE ai_drafts SET state = 'approved' WHERE id = ?1",
        rusqlite::params![draft_id],
    )?;

    Ok(card)
}

/// 审核草稿：拒绝
pub fn reject_draft(storage: &Storage, draft_id: &str) -> Result<()> {
    storage.conn.execute(
        "UPDATE ai_drafts SET state = 'rejected' WHERE id = ?1",
        rusqlite::params![draft_id],
    )?;
    Ok(())
}

/// 获取单个草稿
fn get_draft(storage: &Storage, draft_id: &str) -> Result<Option<AiCardDraft>> {
    let mut stmt = storage.conn.prepare(
        "SELECT id, question, answer, tags, source_file, confidence, source_prompt, created_at, state
         FROM ai_drafts WHERE id = ?1",
    )?;

    let mut rows = stmt.query_map(rusqlite::params![draft_id], |row| {
        let tags_str: String = row.get(3)?;
        let state_str: String = row.get(8)?;

        Ok(AiCardDraft {
            id: row.get(0)?,
            question: row.get(1)?,
            answer: row.get(2)?,
            tags: serde_json::from_str(&tags_str).unwrap_or_default(),
            source_file: row.get(4)?,
            confidence: row.get(5)?,
            source_prompt: row.get(6)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            state: DraftState::from_state_str(&state_str),
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// 获取所有待审核的草稿
pub fn get_pending_drafts(storage: &Storage) -> Result<Vec<AiCardDraft>> {
    let mut stmt = storage.conn.prepare(
        "SELECT id, question, answer, tags, source_file, confidence, source_prompt, created_at, state
         FROM ai_drafts WHERE state = 'draft' ORDER BY created_at ASC",
    )?;

    let drafts = stmt
        .query_map([], |row| {
            let tags_str: String = row.get(3)?;
            let state_str: String = row.get(8)?;

            Ok(AiCardDraft {
                id: row.get(0)?,
                question: row.get(1)?,
                answer: row.get(2)?,
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                source_file: row.get(4)?,
                confidence: row.get(5)?,
                source_prompt: row.get(6)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                state: DraftState::from_state_str(&state_str),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(drafts)
}
