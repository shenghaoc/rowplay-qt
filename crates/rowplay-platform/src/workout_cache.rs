// SPDX-License-Identifier: GPL-3.0-or-later
//! Local workout cache boundary (native-only offline capability; the web app
//! is stateless).

use std::collections::BTreeMap;
use std::sync::Mutex;

use rowplay_core::models::{Workout, WorkoutDetail};

/// Cache failures. Messages must never contain workout payloads.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CacheError {
    /// The store could not be opened.
    #[error("cache could not be opened: {0}")]
    Open(String),
    /// Schema migration failed.
    #[error("cache migration failed: {0}")]
    Migration(String),
    /// A query failed.
    #[error("cache query failed: {0}")]
    Query(String),
    /// A stored row could not be decoded.
    #[error("cache row {id} could not be decoded")]
    Decode {
        /// Workout id of the undecodable row.
        id: i64,
    },
}

/// Persistent store of workout details, keyed by Concept2 result id.
pub trait WorkoutCache: Send + Sync {
    /// Create or upgrade the schema. Idempotent.
    fn migrate(&self) -> Result<(), CacheError>;
    /// Every cached summary, newest first.
    fn list_workouts(&self) -> Result<Vec<Workout>, CacheError>;
    /// Details for the requested ids; ids without a row are simply absent.
    fn details(&self, ids: &[i64]) -> Result<BTreeMap<i64, WorkoutDetail>, CacheError>;
    /// Insert or replace details; returns how many rows were written.
    fn save_details(&self, details: &[WorkoutDetail]) -> Result<usize, CacheError>;
    /// Remove every row (disconnect / delete local data).
    fn clear(&self) -> Result<(), CacheError>;
}

/// Process-local cache for tests and demo mode.
#[derive(Debug, Default)]
pub struct InMemoryWorkoutCache {
    rows: Mutex<BTreeMap<i64, WorkoutDetail>>,
    migrated: Mutex<bool>,
}

impl InMemoryWorkoutCache {
    /// A cache pre-populated with `details`.
    #[must_use]
    pub fn with_details(details: &[WorkoutDetail]) -> Self {
        let cache = Self::default();
        cache
            .save_details(details)
            .expect("in-memory save cannot fail");
        cache
    }

    /// Whether `migrate` has been called.
    #[must_use]
    pub fn is_migrated(&self) -> bool {
        *self.migrated.lock().expect("cache lock")
    }
}

impl WorkoutCache for InMemoryWorkoutCache {
    fn migrate(&self) -> Result<(), CacheError> {
        *self.migrated.lock().expect("cache lock") = true;
        Ok(())
    }

    fn list_workouts(&self) -> Result<Vec<Workout>, CacheError> {
        let rows = self.rows.lock().expect("cache lock");
        let mut list: Vec<Workout> = rows.values().map(WorkoutDetail::summary).collect();
        list.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.id.cmp(&a.id)));
        Ok(list)
    }

    fn details(&self, ids: &[i64]) -> Result<BTreeMap<i64, WorkoutDetail>, CacheError> {
        let rows = self.rows.lock().expect("cache lock");
        Ok(ids
            .iter()
            .filter_map(|id| rows.get(id).map(|d| (*id, d.clone())))
            .collect())
    }

    fn save_details(&self, details: &[WorkoutDetail]) -> Result<usize, CacheError> {
        let mut rows = self.rows.lock().expect("cache lock");
        for detail in details {
            rows.insert(detail.id(), detail.clone());
        }
        Ok(details.len())
    }

    fn clear(&self) -> Result<(), CacheError> {
        self.rows.lock().expect("cache lock").clear();
        Ok(())
    }
}

/// A cache whose every operation fails with the configured error (Studio
/// `ThrowingWorkoutCache`), for verifying that failures propagate.
#[derive(Debug, Clone)]
pub struct FailingWorkoutCache {
    /// Error returned by every call.
    pub error: CacheError,
}

impl WorkoutCache for FailingWorkoutCache {
    fn migrate(&self) -> Result<(), CacheError> {
        Err(self.error.clone())
    }

    fn list_workouts(&self) -> Result<Vec<Workout>, CacheError> {
        Err(self.error.clone())
    }

    fn details(&self, _ids: &[i64]) -> Result<BTreeMap<i64, WorkoutDetail>, CacheError> {
        Err(self.error.clone())
    }

    fn save_details(&self, _details: &[WorkoutDetail]) -> Result<usize, CacheError> {
        Err(self.error.clone())
    }

    fn clear(&self) -> Result<(), CacheError> {
        Err(self.error.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowplay_core::demo::demo_details;

    #[test]
    fn round_trips_details_newest_first() {
        let cache = InMemoryWorkoutCache::default();
        cache.migrate().unwrap();
        assert!(cache.is_migrated());
        assert!(cache.list_workouts().unwrap().is_empty());
        let details = demo_details();
        assert_eq!(cache.save_details(&details).unwrap(), details.len());
        let listed = cache.list_workouts().unwrap();
        assert_eq!(listed.len(), details.len());
        assert!(listed.windows(2).all(|pair| pair[0].date >= pair[1].date));
        let fetched = cache.details(&[1001, 424_242]).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(
            fetched[&1001].strokes.len(),
            details
                .iter()
                .find(|d| d.id() == 1001)
                .unwrap()
                .strokes
                .len()
        );
        cache.clear().unwrap();
        assert!(cache.list_workouts().unwrap().is_empty());
    }

    #[test]
    fn saving_the_same_id_replaces_the_row() {
        let cache = InMemoryWorkoutCache::with_details(&demo_details());
        let mut updated = cache.details(&[1001]).unwrap().remove(&1001).unwrap();
        updated.workout.comments = Some("edited".into());
        cache.save_details(std::slice::from_ref(&updated)).unwrap();
        assert_eq!(cache.list_workouts().unwrap().len(), 17);
        assert_eq!(
            cache.details(&[1001]).unwrap()[&1001]
                .workout
                .comments
                .as_deref(),
            Some("edited")
        );
    }

    #[test]
    fn failing_cache_propagates_errors() {
        let cache = FailingWorkoutCache {
            error: CacheError::Open("disk full".into()),
        };
        assert_eq!(cache.migrate(), Err(CacheError::Open("disk full".into())));
        assert!(cache.list_workouts().is_err());
        assert!(cache.details(&[1]).is_err());
        assert!(cache.save_details(&[]).is_err());
        assert!(cache.clear().is_err());
        assert_eq!(
            CacheError::Decode { id: 7 }.to_string(),
            "cache row 7 could not be decoded"
        );
    }
}
