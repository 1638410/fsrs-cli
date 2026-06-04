use chrono::{DateTime, Utc};
use rs_fsrs::{Card as FsrsCard, Rating as FsrsRating, State as FsrsState, FSRS};

use crate::core::types::{Rating, ScheduleState, State};

/// FSRS 调度器封装
pub struct Scheduler {
    fsrs: FSRS,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    /// 创建默认调度器
    pub fn new() -> Self {
        Scheduler {
            fsrs: FSRS::default(),
        }
    }

    /// 将我们的 ScheduleState 转换为 rs-fsrs 的 Card
    fn to_fsrs_card(&self, schedule: &ScheduleState) -> FsrsCard {
        FsrsCard {
            due: schedule.due_date,
            stability: schedule.stability,
            difficulty: schedule.difficulty,
            elapsed_days: schedule.elapsed_days,
            scheduled_days: schedule.scheduled_days,
            reps: schedule.reps,
            lapses: schedule.lapses,
            state: match schedule.state {
                State::New => FsrsState::New,
                State::Learning => FsrsState::Learning,
                State::Review => FsrsState::Review,
                State::Relearning => FsrsState::Relearning,
            },
            last_review: schedule.last_review.unwrap_or_else(Utc::now),
        }
    }

    /// 将 rs-fsrs 的 Card 转换为我们的 ScheduleState
    fn to_schedule_state(&self, card_id: &str, fsrs_card: FsrsCard) -> ScheduleState {
        ScheduleState {
            card_id: card_id.to_string(),
            difficulty: fsrs_card.difficulty,
            stability: fsrs_card.stability,
            last_review: Some(fsrs_card.last_review),
            due_date: fsrs_card.due,
            elapsed_days: fsrs_card.elapsed_days,
            scheduled_days: fsrs_card.scheduled_days,
            reps: fsrs_card.reps,
            lapses: fsrs_card.lapses,
            state: match fsrs_card.state {
                FsrsState::New => State::New,
                FsrsState::Learning => State::Learning,
                FsrsState::Review => State::Review,
                FsrsState::Relearning => State::Relearning,
            },
        }
    }

    /// 将我们的 Rating 转换为 rs-fsrs 的 Rating
    fn to_fsrs_rating(rating: Rating) -> FsrsRating {
        match rating {
            Rating::Again => FsrsRating::Again,
            Rating::Hard => FsrsRating::Hard,
            Rating::Good => FsrsRating::Good,
            Rating::Easy => FsrsRating::Easy,
        }
    }

    /// 为新卡片创建初始调度状态
    pub fn new_card_schedule(&self, card_id: &str) -> ScheduleState {
        let now = Utc::now();
        ScheduleState {
            card_id: card_id.to_string(),
            difficulty: 0.0,
            stability: 0.0,
            last_review: None,
            due_date: now,
            elapsed_days: 0,
            scheduled_days: 0,
            reps: 0,
            lapses: 0,
            state: State::New,
        }
    }

    /// 复习卡片，返回新的调度状态
    pub fn review_card(
        &self,
        schedule: &ScheduleState,
        rating: Rating,
    ) -> Result<ScheduleState, String> {
        let fsrs_card = self.to_fsrs_card(schedule);
        let fsrs_rating = Self::to_fsrs_rating(rating);

        let record_log = self.fsrs.repeat(fsrs_card, Utc::now());

        let item = record_log
            .get(&fsrs_rating)
            .ok_or_else(|| format!("无法获取 rating {:?} 的调度结果", rating))?;

        Ok(self.to_schedule_state(&schedule.card_id, item.card.clone()))
    }

    /// 获取卡片的可检索性（retention probability）
    pub fn get_retrievability(&self, schedule: &ScheduleState, now: DateTime<Utc>) -> f64 {
        let fsrs_card = self.to_fsrs_card(schedule);
        fsrs_card.get_retrievability(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_card_schedule() {
        let scheduler = Scheduler::new();
        let schedule = scheduler.new_card_schedule("test-card");
        assert_eq!(schedule.reps, 0);
        assert_eq!(schedule.state, State::New);
    }

    #[test]
    fn test_review_card_good() {
        let scheduler = Scheduler::new();
        let schedule = scheduler.new_card_schedule("test-card");
        let new_schedule = scheduler.review_card(&schedule, Rating::Good).unwrap();
        assert!(new_schedule.reps > 0);
        assert!(new_schedule.stability > 0.0);
    }

    #[test]
    fn test_review_card_again() {
        let scheduler = Scheduler::new();
        let schedule = scheduler.new_card_schedule("test-card");
        let new_schedule = scheduler.review_card(&schedule, Rating::Again).unwrap();
        assert_ne!(new_schedule.state, State::New);
    }
}
