// SPDX-License-Identifier: GPL-3.0-or-later
//! SQLite-backed workout cache.
//!
//! Port of rowplay-studio's `Storage/SQLiteWorkoutCache.swift` and
//! `Storage/SQLiteWorkoutCacheMigration.swift`, on `rusqlite` with the
//! `bundled` feature so no system SQLite is needed on any target.
//!
//! The schema mirrors Studio's single `workouts` table: one row per Concept2
//! result id, a summary column per scalar field, the whole `WorkoutDetail` as
//! JSON, and an index on the date. `PRAGMA user_version` tracks migrations.
//!
//! Deliberate divergence: Studio rebuilds summary rows from the columns, which
//! silently drops the fields its column list does not cover (`privacy`,
//! `hr_min`/`hr_max`, `timezone`, `date_utc`, `weight_class`, `rest_time`,
//! `rest_distance`, `targets`, `metadata`, heart-rate detail). The Rust cache
//! decodes `detail_json` instead, so a summary round-trips exactly; the columns
//! are still written in full for schema parity and for SQL-level filtering, and
//! a test asserts the two representations agree. See `docs/source-map.md`.
//!
//! On Unix the containing directory is `0700` and the database file `0600`.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rowplay_core::models::{Workout, WorkoutDetail};
use rusqlite::{Connection, params_from_iter};

use super::CacheError;
use crate::paths;

/// Current schema version, stored in `PRAGMA user_version`.
pub const SCHEMA_VERSION: i32 = 1;

/// Version 1 of the schema (Studio's column set, with `date` as the logbook
/// string because the core model keeps it verbatim rather than as an instant).
const CREATE_SCHEMA_V1: &str = "
CREATE TABLE IF NOT EXISTS workouts (
    id INTEGER PRIMARY KEY,
    sport TEXT NOT NULL,
    date TEXT NOT NULL,
    workout_type TEXT,
    distance REAL NOT NULL,
    time REAL NOT NULL,
    pace REAL NOT NULL,
    stroke_rate REAL,
    stroke_count REAL,
    heart_rate_avg REAL,
    calories_total REAL,
    watt_minutes REAL,
    drag_factor REAL,
    comments TEXT,
    source TEXT,
    verified INTEGER,
    has_stroke_data INTEGER NOT NULL DEFAULT 0,
    is_interval INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT NOT NULL,
    updated_at REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_workouts_date ON workouts (date DESC);
";

/// Upsert keyed by the Concept2 result id (`INSERT OR REPLACE`, like Studio).
const UPSERT_WORKOUT: &str = "
INSERT OR REPLACE INTO workouts (
    id, sport, date, workout_type, distance, time, pace,
    stroke_rate, stroke_count, heart_rate_avg, calories_total,
    watt_minutes, drag_factor, comments, source, verified,
    has_stroke_data, is_interval, detail_json, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
          ?17, ?18, ?19, ?20);
";

/// Newest first, with the result id breaking ties like the in-memory cache.
const SELECT_LIST: &str = "SELECT detail_json FROM workouts ORDER BY date DESC, id DESC;";

/// SQLite-backed [`super::WorkoutCache`].
///
/// A single connection behind a `Mutex` (`rusqlite::Connection` is `Send` but
/// not `Sync`). Call [`super::WorkoutCache::migrate`] after opening and before
/// any other operation.
pub struct SqliteWorkoutCache {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl std::fmt::Debug for SqliteWorkoutCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteWorkoutCache")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteWorkoutCache {
    /// Open (creating the file with owner-only permissions) a cache at `path`.
    ///
    /// The schema is not created here; call
    /// [`super::WorkoutCache::migrate`] first.
    pub fn open(path: &Path) -> Result<Self, CacheError> {
        paths::ensure_private_file(path).map_err(|error| {
            CacheError::Open(format!("could not prepare {}: {error}", path.display()))
        })?;
        let connection =
            Connection::open(path).map_err(|error| CacheError::Open(format!("sqlite: {error}")))?;
        Ok(SqliteWorkoutCache {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
        })
    }

    /// Open a cache at the platform's default location.
    pub fn open_default() -> Result<Self, CacheError> {
        let path = paths::default_workout_cache_path()
            .map_err(|error| CacheError::Open(format!("no data directory: {error}")))?;
        Self::open(&path)
    }

    /// Path of the database file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, CacheError> {
        self.connection
            .lock()
            .map_err(|_| CacheError::Query("cache lock poisoned".to_owned()))
    }
}

impl super::WorkoutCache for SqliteWorkoutCache {
    fn migrate(&self) -> Result<(), CacheError> {
        let mut connection = self.lock()?;
        let version: i32 = connection
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|error| query_error(&error))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }

        let transaction = connection
            .transaction()
            .map_err(|error| query_error(&error))?;
        if version < 1 {
            transaction
                .execute_batch(CREATE_SCHEMA_V1)
                .map_err(|error| CacheError::Migration(format!("schema v1: {error}")))?;
        }
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|error| CacheError::Migration(format!("user_version: {error}")))?;
        transaction
            .commit()
            .map_err(|error| CacheError::Migration(format!("commit: {error}")))?;
        Ok(())
    }

    fn list_workouts(&self) -> Result<Vec<Workout>, CacheError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(SELECT_LIST)
            .map_err(|error| query_error(&error))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| query_error(&error))?;

        let mut workouts = Vec::new();
        for row in rows {
            let json = row.map_err(|error| query_error(&error))?;
            // The row id lives inside the JSON, so a failure here cannot name a
            // specific id without decoding it.
            let detail = decode_detail(&json).map_err(|_| {
                CacheError::Query("a cached workout could not be decoded".to_owned())
            })?;
            workouts.push(detail.summary());
        }
        Ok(workouts)
    }

    fn details(
        &self,
        ids: &[i64],
    ) -> Result<std::collections::BTreeMap<i64, WorkoutDetail>, CacheError> {
        let mut found = std::collections::BTreeMap::new();
        if ids.is_empty() {
            return Ok(found);
        }

        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!("SELECT id, detail_json FROM workouts WHERE id IN ({placeholders});");
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| query_error(&error))?;
        let rows = statement
            .query_map(params_from_iter(ids.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| query_error(&error))?;

        for row in rows {
            let (id, json) = row.map_err(|error| query_error(&error))?;
            let detail = decode_detail(&json).map_err(|_| CacheError::Decode { id })?;
            found.insert(id, detail);
        }
        Ok(found)
    }

    fn save_details(&self, details: &[WorkoutDetail]) -> Result<usize, CacheError> {
        if details.is_empty() {
            return Ok(0);
        }

        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(|error| query_error(&error))?;
        {
            let mut statement = transaction
                .prepare(UPSERT_WORKOUT)
                .map_err(|error| query_error(&error))?;
            let updated_at = now_epoch_seconds();
            for detail in details {
                let workout = &detail.workout;
                let json = serde_json::to_string(detail)
                    .map_err(|_| CacheError::Encode { id: workout.id })?;
                statement
                    .execute(rusqlite::params![
                        workout.id,
                        workout.sport.as_str(),
                        workout.date,
                        workout.workout_type,
                        workout.distance,
                        workout.time,
                        workout.pace,
                        workout.stroke_rate,
                        workout.stroke_count,
                        workout.heart_rate_avg,
                        workout.calories_total,
                        workout.watt_minutes,
                        workout.drag_factor,
                        workout.comments,
                        workout.source,
                        workout.verified,
                        workout.has_stroke_data,
                        workout.is_interval,
                        json,
                        updated_at,
                    ])
                    .map_err(|error| query_error(&error))?;
            }
        }
        transaction.commit().map_err(|error| query_error(&error))?;
        Ok(details.len())
    }

    fn clear(&self) -> Result<(), CacheError> {
        let connection = self.lock()?;
        connection
            .execute("DELETE FROM workouts;", [])
            .map_err(|error| query_error(&error))?;
        Ok(())
    }
}

/// Decode a stored `WorkoutDetail`.
fn decode_detail(json: &str) -> Result<WorkoutDetail, serde_json::Error> {
    serde_json::from_str(json)
}

/// Map a `rusqlite` failure onto the cache's error type.
///
/// SQLite messages name tables and columns, never row payloads.
fn query_error(error: &rusqlite::Error) -> CacheError {
    CacheError::Query(error.to_string())
}

/// Seconds since the Unix epoch, for the `updated_at` column.
fn now_epoch_seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workout_cache::{InMemoryWorkoutCache, WorkoutCache};
    use rowplay_core::demo::demo_details;
    use rowplay_core::models::{Sport, Workout};

    fn cache() -> (tempfile::TempDir, SqliteWorkoutCache) {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = SqliteWorkoutCache::open(&dir.path().join("workouts.sqlite")).expect("open");
        (dir, cache)
    }

    #[test]
    fn migrates_an_empty_file_and_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("workouts.sqlite");

        // A zero-byte file, as a truncated or interrupted first run leaves.
        std::fs::File::create(&path).expect("empty file");
        let cache = SqliteWorkoutCache::open(&path).expect("open");
        cache.migrate().expect("first migration");
        cache.migrate().expect("second migration");
        assert!(cache.list_workouts().unwrap().is_empty());

        let connection = Connection::open(&path).expect("reopen");
        let version: i32 = connection
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'workouts';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
    }

    #[test]
    fn every_trait_method_round_trips_through_sqlite() {
        let (_dir, cache) = cache();
        cache.migrate().unwrap();
        assert!(cache.list_workouts().unwrap().is_empty());

        let details = demo_details();
        assert_eq!(cache.save_details(&details).unwrap(), details.len());

        let listed = cache.list_workouts().unwrap();
        assert_eq!(listed.len(), details.len());
        assert!(listed.windows(2).all(|pair| pair[0].date >= pair[1].date));
        // Full fidelity, including the fields Studio's summary columns drop.
        let mut expected: Vec<Workout> = details.iter().map(WorkoutDetail::summary).collect();
        expected.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.id.cmp(&a.id)));
        assert_eq!(listed, expected);

        // details() returns exactly what was stored, including strokes/splits.
        let ids: Vec<i64> = details.iter().map(WorkoutDetail::id).collect();
        let fetched = cache.details(&ids).unwrap();
        assert_eq!(fetched.len(), details.len());
        for detail in &details {
            assert_eq!(&fetched[&detail.id()], detail, "id {}", detail.id());
        }
        assert!(cache.details(&[424_242]).unwrap().is_empty());
        assert!(cache.details(&[]).unwrap().is_empty());

        cache.clear().unwrap();
        assert!(cache.list_workouts().unwrap().is_empty());
        assert!(cache.details(&ids).unwrap().is_empty());
    }

    #[test]
    fn saving_the_same_id_replaces_the_row() {
        let (_dir, cache) = cache();
        cache.migrate().unwrap();
        cache.save_details(&demo_details()).unwrap();

        let mut updated = cache.details(&[1001]).unwrap().remove(&1001).unwrap();
        updated.workout.comments = Some("edited".into());
        updated.workout.privacy = Some("private".into());
        assert_eq!(
            cache.save_details(std::slice::from_ref(&updated)).unwrap(),
            1
        );

        assert_eq!(cache.list_workouts().unwrap().len(), 17);
        let back = cache.details(&[1001]).unwrap().remove(&1001).unwrap();
        assert_eq!(back.workout.comments.as_deref(), Some("edited"));
        assert_eq!(back.workout.privacy.as_deref(), Some("private"));
    }

    #[test]
    fn summary_columns_agree_with_the_stored_json() {
        let (dir, cache) = cache();
        cache.migrate().unwrap();
        let details = demo_details();
        cache.save_details(&details).unwrap();
        drop(cache);

        // Read the mirrored Studio columns with an independent connection.
        let connection =
            Connection::open(dir.path().join("workouts.sqlite")).expect("independent open");
        for detail in &details {
            let workout = &detail.workout;
            let row = connection
                .query_row(
                    "SELECT sport, date, distance, time, pace, stroke_rate, stroke_count,
                            heart_rate_avg, calories_total, watt_minutes, drag_factor, comments,
                            source, verified, has_stroke_data, is_interval
                     FROM workouts WHERE id = ?1;",
                    [workout.id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, f64>(2)?,
                            row.get::<_, f64>(3)?,
                            row.get::<_, f64>(4)?,
                            row.get::<_, Option<f64>>(5)?,
                            row.get::<_, Option<f64>>(6)?,
                            row.get::<_, Option<f64>>(7)?,
                            row.get::<_, Option<f64>>(8)?,
                            row.get::<_, Option<f64>>(9)?,
                            row.get::<_, Option<f64>>(10)?,
                            row.get::<_, Option<String>>(11)?,
                            row.get::<_, Option<String>>(12)?,
                            row.get::<_, Option<bool>>(13)?,
                            row.get::<_, bool>(14)?,
                            row.get::<_, bool>(15)?,
                        ))
                    },
                )
                .unwrap_or_else(|error| panic!("row {}: {error}", workout.id));

            assert_eq!(row.0, workout.sport.as_str(), "sport {}", workout.id);
            assert_eq!(row.1, workout.date, "date {}", workout.id);
            assert_eq!(row.2, workout.distance);
            assert_eq!(row.3, workout.time);
            assert_eq!(row.4, workout.pace);
            assert_eq!(row.5, workout.stroke_rate);
            assert_eq!(row.6, workout.stroke_count);
            assert_eq!(row.7, workout.heart_rate_avg);
            assert_eq!(row.8, workout.calories_total);
            assert_eq!(row.9, workout.watt_minutes);
            assert_eq!(row.10, workout.drag_factor);
            assert_eq!(row.11, workout.comments);
            assert_eq!(row.12, workout.source);
            assert_eq!(row.13, workout.verified);
            assert_eq!(row.14, workout.has_stroke_data);
            assert_eq!(row.15, workout.is_interval);
        }
    }

    #[test]
    fn a_corrupt_row_is_typed_rather_than_panicking() {
        let (dir, cache) = cache();
        cache.migrate().unwrap();
        cache.save_details(&demo_details()).unwrap();
        drop(cache);

        let connection =
            Connection::open(dir.path().join("workouts.sqlite")).expect("independent open");
        connection
            .execute(
                "UPDATE workouts SET detail_json = '{not json' WHERE id = 1001;",
                [],
            )
            .unwrap();
        drop(connection);

        let cache = SqliteWorkoutCache::open(&dir.path().join("workouts.sqlite")).unwrap();
        assert!(cache.details(&[1001]).is_err());
        assert!(cache.list_workouts().is_err());
        // The healthy rows are still intact.
        assert!(cache.details(&[1004]).unwrap().contains_key(&1004));
    }

    #[test]
    fn an_unmigrated_database_reports_a_query_failure() {
        let (_dir, cache) = cache();
        assert!(cache.list_workouts().is_err());
        assert!(cache.details(&[1]).is_err());
        assert!(cache.save_details(&demo_details()[..1]).is_err());
        assert!(cache.clear().is_err());
        // migrate() then makes everything work.
        cache.migrate().unwrap();
        assert!(cache.list_workouts().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn the_database_file_is_owner_only_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, cache) = cache();
        cache.migrate().unwrap();
        let mode = std::fs::metadata(dir.path().join("workouts.sqlite"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "cache file mode");
    }

    #[test]
    fn the_sqlite_and_in_memory_caches_agree() {
        let (_dir, sqlite) = cache();
        sqlite.migrate().unwrap();
        let memory = InMemoryWorkoutCache::default();
        memory.migrate().unwrap();

        let details = demo_details();
        assert_eq!(
            sqlite.save_details(&details).unwrap(),
            memory.save_details(&details).unwrap()
        );
        assert_eq!(
            sqlite.list_workouts().unwrap(),
            memory.list_workouts().unwrap()
        );
        let ids = vec![1001, 1004, 999_999];
        assert_eq!(sqlite.details(&ids).unwrap(), memory.details(&ids).unwrap());
        sqlite.clear().unwrap();
        memory.clear().unwrap();
        assert_eq!(
            sqlite.list_workouts().unwrap(),
            memory.list_workouts().unwrap()
        );
    }

    #[test]
    fn sports_round_trip() {
        let (_dir, cache) = cache();
        cache.migrate().unwrap();
        let mut details = Vec::new();
        for (index, sport) in Sport::ALL.iter().enumerate() {
            details.push(WorkoutDetail {
                workout: Workout::new(
                    index as i64 + 1,
                    "2026-05-15 07:30:00",
                    *sport,
                    2000.0,
                    450.0,
                    112.5,
                ),
                strokes: Vec::new(),
                splits: Vec::new(),
            });
        }
        cache.save_details(&details).unwrap();
        let listed = cache.list_workouts().unwrap();
        let mut sports: Vec<Sport> = listed.iter().map(|workout| workout.sport).collect();
        sports.sort();
        assert_eq!(sports, vec![Sport::Rower, Sport::Skierg, Sport::Bike]);
    }
}
