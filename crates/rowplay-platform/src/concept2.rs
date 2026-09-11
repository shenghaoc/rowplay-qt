// SPDX-License-Identifier: GPL-3.0-or-later
//! Concept2 Logbook API client boundary.
//!
//! Phase 3 adds the HTTPS implementation (bearer token, HTTPS-only, same-host
//! redirects only, strict timeouts, no on-disk cache) and the raw-payload
//! mapper validated against `tests/fixtures/Concept2/*.fixture.json`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use rowplay_core::models::{Workout, WorkoutDetail};

/// One page of the results list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResultsPage {
    /// Summaries on this page.
    pub workouts: Vec<Workout>,
    /// One-based page number.
    pub page: u32,
    /// Total number of pages.
    pub total_pages: u32,
}

/// Client failures. Descriptions are privacy-safe: no tokens, headers or payloads.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum Concept2Error {
    /// The token was rejected.
    #[error("Concept2 rejected the token")]
    Unauthorized,
    /// Too many requests.
    #[error("Concept2 rate limit reached")]
    RateLimited {
        /// Seconds to wait before retrying, when the API said so.
        retry_after_secs: Option<u64>,
    },
    /// Unknown result id.
    #[error("Concept2 result {0} was not found")]
    NotFound(i64),
    /// Network failure (message already redacted by the caller).
    #[error("Concept2 request failed: {0}")]
    Transport(String),
    /// Malformed response.
    #[error("Concept2 response could not be decoded: {0}")]
    Decode(String),
}

/// Read-only access to the athlete's logbook. rowplay never writes back.
pub trait Concept2Client: Send + Sync {
    /// Fetch one page of result summaries (newest first).
    fn list_results(&self, page: u32) -> Result<ResultsPage, Concept2Error>;
    /// Fetch the full detail (strokes and splits) of one result.
    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error>;
}

/// Deterministic client for tests: serves pages built from a detail list and
/// records every call.
#[derive(Debug, Default)]
pub struct MockConcept2Client {
    details: BTreeMap<i64, WorkoutDetail>,
    page_size: usize,
    failure: Option<Concept2Error>,
    calls: Mutex<Vec<String>>,
}

impl MockConcept2Client {
    /// A client serving `details` in pages of `page_size` (newest first).
    #[must_use]
    pub fn new(details: Vec<WorkoutDetail>, page_size: usize) -> Self {
        MockConcept2Client {
            details: details.into_iter().map(|d| (d.id(), d)).collect(),
            page_size: page_size.max(1),
            failure: None,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Make every call fail with `error`.
    #[must_use]
    pub fn failing_with(mut self, error: Concept2Error) -> Self {
        self.failure = Some(error);
        self
    }

    /// Calls made so far, as `list:<page>` / `detail:<id>` strings.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls lock").clone()
    }

    fn ordered(&self) -> Vec<&WorkoutDetail> {
        let mut list: Vec<&WorkoutDetail> = self.details.values().collect();
        list.sort_by(|a, b| {
            b.workout
                .date
                .cmp(&a.workout.date)
                .then_with(|| b.id().cmp(&a.id()))
        });
        list
    }
}

impl Concept2Client for MockConcept2Client {
    fn list_results(&self, page: u32) -> Result<ResultsPage, Concept2Error> {
        self.calls
            .lock()
            .expect("calls lock")
            .push(format!("list:{page}"));
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let ordered = self.ordered();
        let total_pages = ordered.len().div_ceil(self.page_size).max(1) as u32;
        let start = (page.max(1) as usize - 1) * self.page_size;
        let workouts = ordered
            .iter()
            .skip(start)
            .take(self.page_size)
            .map(|d| d.summary())
            .collect();
        Ok(ResultsPage {
            workouts,
            page: page.max(1),
            total_pages,
        })
    }

    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error> {
        self.calls
            .lock()
            .expect("calls lock")
            .push(format!("detail:{id}"));
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        self.details
            .get(&id)
            .cloned()
            .ok_or(Concept2Error::NotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowplay_core::demo::demo_details;

    #[test]
    fn mock_pages_through_details_newest_first() {
        let client = MockConcept2Client::new(demo_details(), 5);
        let first = client.list_results(1).unwrap();
        assert_eq!(first.page, 1);
        assert_eq!(first.total_pages, 4);
        assert_eq!(first.workouts.len(), 5);
        assert_eq!(first.workouts[0].id, 1001);
        let last = client.list_results(4).unwrap();
        assert_eq!(last.workouts.len(), 2);
        assert!(client.list_results(9).unwrap().workouts.is_empty());
        let detail = client.result_detail(1004).unwrap();
        assert!(!detail.strokes.is_empty());
        assert_eq!(client.result_detail(5), Err(Concept2Error::NotFound(5)));
        assert_eq!(
            client.calls(),
            vec!["list:1", "list:4", "list:9", "detail:1004", "detail:5"]
        );
    }

    #[test]
    fn failures_are_privacy_safe() {
        let client = MockConcept2Client::new(vec![], 10).failing_with(Concept2Error::Unauthorized);
        assert_eq!(client.list_results(1), Err(Concept2Error::Unauthorized));
        assert_eq!(
            Concept2Error::Unauthorized.to_string(),
            "Concept2 rejected the token"
        );
        assert_eq!(
            Concept2Error::RateLimited {
                retry_after_secs: Some(30)
            }
            .to_string(),
            "Concept2 rate limit reached"
        );
        let empty = MockConcept2Client::new(vec![], 10);
        let page = empty.list_results(1).unwrap();
        assert_eq!(page.total_pages, 1);
        assert!(page.workouts.is_empty());
    }
}
