// SPDX-License-Identifier: GPL-3.0-or-later
//! Workout library loading: cache → demo → empty (Studio `WorkoutLibraryLoader`).

use rowplay_core::demo::demo_details;
use rowplay_core::models::WorkoutDetail;

use crate::workout_cache::{CacheError, WorkoutCache};

/// Which data source the library loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkoutLibrarySource {
    /// The persistent cache.
    Cache,
    /// Deterministic demo fixtures.
    Demo,
    /// Nothing: the cache is empty and demo mode is off.
    Empty,
}

/// An immutable snapshot of loaded workout details.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkoutLibrarySnapshot {
    /// Loaded details, newest first.
    pub details: Vec<WorkoutDetail>,
    /// Where they came from.
    pub source: WorkoutLibrarySource,
}

/// Load workouts from the cache, falling back to demo data or an empty
/// library. Cache failures propagate; they never silently become demo data.
pub fn load_library(
    cache: &dyn WorkoutCache,
    demo_mode_enabled: bool,
) -> Result<WorkoutLibrarySnapshot, CacheError> {
    cache.migrate()?;
    let workouts = cache.list_workouts()?;
    if !workouts.is_empty() {
        let ids: Vec<i64> = workouts.iter().map(|w| w.id).collect();
        let mut by_id = cache.details(&ids)?;
        let details = workouts
            .into_iter()
            .map(|workout| {
                by_id.remove(&workout.id).unwrap_or_else(|| WorkoutDetail {
                    workout,
                    strokes: Vec::new(),
                    splits: Vec::new(),
                })
            })
            .collect();
        return Ok(WorkoutLibrarySnapshot {
            details,
            source: WorkoutLibrarySource::Cache,
        });
    }
    if demo_mode_enabled {
        return Ok(WorkoutLibrarySnapshot {
            details: demo_details(),
            source: WorkoutLibrarySource::Demo,
        });
    }
    Ok(WorkoutLibrarySnapshot {
        details: Vec::new(),
        source: WorkoutLibrarySource::Empty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workout_cache::{FailingWorkoutCache, InMemoryWorkoutCache};

    #[test]
    fn empty_cache_with_demo_mode_serves_demo_data() {
        let cache = InMemoryWorkoutCache::default();
        let snapshot = load_library(&cache, true).unwrap();
        assert_eq!(snapshot.source, WorkoutLibrarySource::Demo);
        assert_eq!(snapshot.details.len(), 17);
        assert!(cache.is_migrated());
    }

    #[test]
    fn empty_cache_without_demo_mode_is_empty() {
        let snapshot = load_library(&InMemoryWorkoutCache::default(), false).unwrap();
        assert_eq!(snapshot.source, WorkoutLibrarySource::Empty);
        assert!(snapshot.details.is_empty());
    }

    #[test]
    fn cached_workouts_win_over_demo_data() {
        let details: Vec<WorkoutDetail> = demo_details().into_iter().take(3).collect();
        let cache = InMemoryWorkoutCache::with_details(&details);
        let snapshot = load_library(&cache, true).unwrap();
        assert_eq!(snapshot.source, WorkoutLibrarySource::Cache);
        assert_eq!(snapshot.details.len(), 3);
        assert_eq!(snapshot.details[0].id(), details[0].id());
    }

    #[test]
    fn cache_errors_propagate_instead_of_falling_back() {
        let cache = FailingWorkoutCache {
            error: CacheError::Migration("bad schema".into()),
        };
        assert_eq!(
            load_library(&cache, true).unwrap_err(),
            CacheError::Migration("bad schema".into())
        );
    }
}
