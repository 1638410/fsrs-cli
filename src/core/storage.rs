use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

use super::types::*;

/// 存储层：封装所有 SQLite 操作
pub struct Storage {
    pub conn: Connection,
}

impl Storage {
    /// 打开或创建数据库
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("无法打开数据库: {}", db_path.display()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let storage = Storage { conn };
        storage.migrate()?;
        Ok(storage)
    }

    /// 数据库迁移
    fn migrate(&self) -> Result<()> {
        // 创建版本表
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);",
        )?;

        let current_version: i32 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < 1 {
            self.conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS cards (
                    id TEXT PRIMARY KEY,
                    source_file TEXT NOT NULL,
                    card_index INTEGER NOT NULL,
                    question TEXT NOT NULL,
                    answer TEXT NOT NULL,
                    tags TEXT,
                    question_hash TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    UNIQUE(source_file, card_index)
                );

                CREATE TABLE IF NOT EXISTS schedule (
                    card_id TEXT PRIMARY KEY REFERENCES cards(id) ON DELETE CASCADE,
                    difficulty REAL NOT NULL DEFAULT 0,
                    stability REAL NOT NULL DEFAULT 0,
                    last_review TEXT,
                    due_date TEXT NOT NULL,
                    elapsed_days INTEGER NOT NULL DEFAULT 0,
                    scheduled_days INTEGER NOT NULL DEFAULT 0,
                    reps INTEGER NOT NULL DEFAULT 0,
                    lapses INTEGER NOT NULL DEFAULT 0,
                    state INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS reviews (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
                    rating INTEGER NOT NULL,
                    elapsed_days INTEGER NOT NULL DEFAULT 0,
                    scheduled_days INTEGER NOT NULL DEFAULT 0,
                    review_time INTEGER NOT NULL DEFAULT 0,
                    reviewed_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS ai_access (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
                    access_time TEXT NOT NULL,
                    reason TEXT,
                    model_name TEXT
                );

                CREATE TABLE IF NOT EXISTS ai_drafts (
                    id TEXT PRIMARY KEY,
                    question TEXT NOT NULL,
                    answer TEXT NOT NULL,
                    tags TEXT,
                    source_file TEXT,
                    confidence REAL,
                    source_prompt TEXT,
                    created_at TEXT NOT NULL,
                    state TEXT DEFAULT 'draft'
                );

                CREATE TABLE IF NOT EXISTS ai_memory (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    tags TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS schedule_due_date ON schedule(due_date);
                CREATE INDEX IF NOT EXISTS reviews_card_id ON reviews(card_id, reviewed_at);
                CREATE INDEX IF NOT EXISTS cards_source_file ON cards(source_file, card_index);
                ",
            )?;

            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![1],
            )?;
        }

        Ok(())
    }

    // ===== Card CRUD =====

    /// 插入一张新卡片
    pub fn insert_card(&self, card: &Card) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO cards (id, source_file, card_index, question, answer, tags, question_hash, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                card.id,
                card.source_file,
                card.card_index,
                card.question,
                card.answer,
                serde_json::to_string(&card.tags).unwrap_or_default(),
                card.question_hash,
                card.created_at.to_rfc3339(),
                card.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 根据 ID 获取卡片
    pub fn get_card(&self, card_id: &str) -> Result<Option<Card>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file, card_index, question, answer, tags, question_hash, created_at, updated_at
             FROM cards WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![card_id], |row| {
            Ok(Card {
                id: row.get(0)?,
                source_file: row.get(1)?,
                card_index: row.get(2)?,
                question: row.get(3)?,
                answer: row.get(4)?,
                tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                question_hash: row.get(6)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// 获取指定文件的所有卡片
    pub fn get_cards_by_source(&self, source_file: &str) -> Result<Vec<Card>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file, card_index, question, answer, tags, question_hash, created_at, updated_at
             FROM cards WHERE source_file = ?1 ORDER BY card_index",
        )?;

        let cards = stmt
            .query_map(params![source_file], |row| {
                Ok(Card {
                    id: row.get(0)?,
                    source_file: row.get(1)?,
                    card_index: row.get(2)?,
                    question: row.get(3)?,
                    answer: row.get(4)?,
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    question_hash: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(cards)
    }

    /// 删除卡片及其关联数据
    pub fn delete_card(&self, card_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM cards WHERE id = ?1", params![card_id])?;
        Ok(())
    }

    /// 获取没有标签的卡片
    pub fn get_cards_without_tags(&self) -> Result<Vec<Card>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file, card_index, question, answer, tags, question_hash, created_at, updated_at
             FROM cards WHERE tags = '[]' OR tags = '' OR tags IS NULL
             ORDER BY created_at DESC",
        )?;

        let cards = stmt
            .query_map([], |row| {
                Ok(Card {
                    id: row.get(0)?,
                    source_file: row.get(1)?,
                    card_index: row.get(2)?,
                    question: row.get(3)?,
                    answer: row.get(4)?,
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    question_hash: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(cards)
    }

    // ===== Schedule CRUD =====

    /// 插入或更新调度状态
    pub fn upsert_schedule(&self, schedule: &ScheduleState) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schedule (card_id, difficulty, stability, last_review, due_date, elapsed_days, scheduled_days, reps, lapses, state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                schedule.card_id,
                schedule.difficulty,
                schedule.stability,
                schedule.last_review.map(|dt| dt.to_rfc3339()),
                schedule.due_date.to_rfc3339(),
                schedule.elapsed_days,
                schedule.scheduled_days,
                schedule.reps,
                schedule.lapses,
                schedule.state as i32,
            ],
        )?;
        Ok(())
    }

    /// 获取卡片的调度状态
    pub fn get_schedule(&self, card_id: &str) -> Result<Option<ScheduleState>> {
        let mut stmt = self.conn.prepare(
            "SELECT card_id, difficulty, stability, last_review, due_date, elapsed_days, scheduled_days, reps, lapses, state
             FROM schedule WHERE card_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![card_id], |row| {
            let last_review_str: Option<String> = row.get(3)?;
            let due_date_str: String = row.get(4)?;

            Ok(ScheduleState {
                card_id: row.get(0)?,
                difficulty: row.get(1)?,
                stability: row.get(2)?,
                last_review: last_review_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                }),
                due_date: chrono::DateTime::parse_from_rfc3339(&due_date_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                elapsed_days: row.get(5)?,
                scheduled_days: row.get(6)?,
                reps: row.get(7)?,
                lapses: row.get(8)?,
                state: State::from_i32(row.get(9)?),
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// 获取所有到期的卡片 ID（按 due_date 升序）
    pub fn get_due_cards(&self, limit: i32) -> Result<Vec<String>> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT card_id FROM schedule WHERE due_date <= ?1 ORDER BY due_date ASC LIMIT ?2",
        )?;

        let ids = stmt
            .query_map(params![now, limit], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    // ===== Review Log =====

    /// 插入复习记录
    pub fn insert_review(&self, log: &ReviewLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO reviews (card_id, rating, elapsed_days, scheduled_days, review_time, reviewed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                log.card_id,
                log.rating as i32,
                log.elapsed_days,
                log.scheduled_days,
                log.review_time,
                log.reviewed_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 获取卡片的复习记录
    pub fn get_reviews(&self, card_id: &str) -> Result<Vec<ReviewLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, card_id, rating, elapsed_days, scheduled_days, review_time, reviewed_at
             FROM reviews WHERE card_id = ?1 ORDER BY reviewed_at ASC",
        )?;

        let logs = stmt
            .query_map(params![card_id], |row| {
                let rating_int: i32 = row.get(2)?;
                let reviewed_at_str: String = row.get(6)?;

                Ok(ReviewLog {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    rating: Rating::from_i32(rating_int).unwrap_or(Rating::Again),
                    elapsed_days: row.get(3)?,
                    scheduled_days: row.get(4)?,
                    review_time: row.get(5)?,
                    reviewed_at: chrono::DateTime::parse_from_rfc3339(&reviewed_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(logs)
    }

    // ===== Statistics =====

    /// 获取总卡片数
    pub fn total_cards(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM cards", [], |row| row.get(0))
            .map_err(|e| e.into())
    }

    /// 获取到期卡片数
    pub fn due_cards_count(&self) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM schedule WHERE due_date <= ?1",
                params![now],
                |row| row.get(0),
            )
            .map_err(|e| e.into())
    }

    /// 获取总复习次数
    pub fn total_reviews(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))
            .map_err(|e| e.into())
    }

    // ===== AI Access =====

    /// 记录 AI 访问
    pub fn insert_ai_access(&self, log: &AiAccessLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ai_access (card_id, access_time, reason, model_name)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                log.card_id,
                log.access_time.to_rfc3339(),
                log.reason,
                log.model_name,
            ],
        )?;
        Ok(())
    }

    /// 获取卡片的 AI 访问次数
    pub fn ai_access_count(&self, card_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM ai_access WHERE card_id = ?1",
                params![card_id],
                |row| row.get(0),
            )
            .map_err(|e| e.into())
    }

    // ===== AI Memory =====

    /// 插入 AI 记忆
    pub fn insert_ai_memory(&self, memory: &AiMemory) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ai_memory (id, content, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                memory.id,
                memory.content,
                serde_json::to_string(&memory.tags).unwrap_or_default(),
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 获取所有 AI 记忆
    pub fn get_ai_memories(&self) -> Result<Vec<AiMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, created_at, updated_at FROM ai_memory ORDER BY created_at DESC",
        )?;

        let memories = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(2)?;
                Ok(AiMemory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(memories)
    }

    /// 删除 AI 记忆
    pub fn delete_ai_memory(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM ai_memory WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ===== Schedule =====

    /// 删除卡片的调度状态
    pub fn delete_schedule(&self, card_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM schedule WHERE card_id = ?1", params![card_id])?;
        Ok(())
    }

    // ===== Statistics with period =====

    /// 获取指定天数内的复习次数
    pub fn reviews_in_period(&self, days: i32) -> Result<i64> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE reviewed_at >= ?1",
                params![cutoff],
                |row| row.get(0),
            )
            .map_err(|e| e.into())
    }

    /// 获取指定天数内新增的卡片数
    pub fn new_cards_in_period(&self, days: i32) -> Result<i64> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM cards WHERE created_at >= ?1",
                params![cutoff],
                |row| row.get(0),
            )
            .map_err(|e| e.into())
    }

    /// 获取指定天数内有复习记录的调度状态
    pub fn schedules_with_reviews_in_period(&self, days: i32) -> Result<Vec<ScheduleState>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT card_id, difficulty, stability, last_review, due_date, elapsed_days, scheduled_days, reps, lapses, state
             FROM schedule WHERE last_review IS NOT NULL AND last_review >= ?1",
        )?;

        let schedules = stmt
            .query_map(params![cutoff], |row| {
                let last_review_str: Option<String> = row.get(3)?;
                let due_date_str: String = row.get(4)?;

                Ok(ScheduleState {
                    card_id: row.get(0)?,
                    difficulty: row.get(1)?,
                    stability: row.get(2)?,
                    last_review: last_review_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    }),
                    due_date: chrono::DateTime::parse_from_rfc3339(&due_date_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    elapsed_days: row.get(5)?,
                    scheduled_days: row.get(6)?,
                    reps: row.get(7)?,
                    lapses: row.get(8)?,
                    state: State::from_i32(row.get(9)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(schedules)
    }

    /// 获取指定天数内的平均可检索性（基于最近一次复习到现在的时间）
    pub fn avg_retention_in_period(&self, days: i32) -> Result<f64> {
        use crate::core::scheduler::Scheduler;
        let schedules = self.schedules_with_reviews_in_period(days)?;
        if schedules.is_empty() {
            return Ok(1.0);
        }
        let scheduler = Scheduler::new();
        let now = chrono::Utc::now();
        let sum: f64 = schedules
            .iter()
            .map(|s| scheduler.get_retrievability(s, now))
            .sum();
        Ok(sum / schedules.len() as f64)
    }

    /// 获取最近 N 天每天的复习次数
    pub fn daily_review_counts(&self, days: i32) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(reviewed_at) as day, COUNT(*) as cnt
             FROM reviews
             WHERE reviewed_at >= ?1
             GROUP BY day
             ORDER BY day ASC",
        )?;

        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let counts = stmt
            .query_map(params![cutoff], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(counts)
    }
}
