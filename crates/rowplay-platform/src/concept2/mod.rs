// SPDX-License-Identifier: GPL-3.0-or-later
//! Concept2 Logbook API client boundary.
//!
//! [`http::Concept2HttpClient`] is the production implementation (Phase 3):
//! blocking `ureq` over rustls, bearer-token only (no OAuth flow), HTTPS-only,
//! same-origin redirects only, strict timeouts and a 25 MiB response cap.
//! [`MockConcept2Client`] stays for tests and demo mode.
//!
//! Payload decoding and mapping belong to `rowplay_core::concept2`; this module
//! only moves bytes and classifies failures.

pub mod http;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rowplay_core::models::{Sport, Stroke, Workout, WorkoutDetail};

pub use http::{Concept2HttpClient, DEFAULT_BASE_URL, HttpOptions, HttpUri, MAX_BODY_BYTES};

/// One page of the results list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResultsPage {
    /// Summaries on this page.
    pub workouts: Vec<Workout>,
    /// One-based page number.
    pub page: u32,
    /// Total number of pages, as the API reports it.
    pub total_pages: u32,
}

/// Client failures. Descriptions are privacy-safe: no tokens, headers or payloads.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum Concept2Error {
    /// The token was rejected (HTTP 401).
    #[error("Concept2 rejected the token")]
    Unauthorized,
    /// The token is valid but not allowed (HTTP 403).
    #[error("Concept2 rejected the request")]
    Forbidden,
    /// Too many requests (HTTP 429).
    #[error("Concept2 rate limit reached")]
    RateLimited {
        /// Seconds to wait before retrying, when the API said so.
        retry_after_secs: Option<u64>,
    },
    /// Any other non-2xx response.
    #[error("Concept2 returned HTTP {status}")]
    Http {
        /// The status code.
        status: u16,
    },
    /// Unknown result id.
    #[error("Concept2 result {0} was not found")]
    NotFound(i64),
    /// The base URL is not an absolute HTTP(S) URL.
    #[error("Concept2 base URL is invalid")]
    InvalidUrl,
    /// A plain `http` connection was attempted to a non-loopback host.
    #[error("Concept2 connections must use HTTPS")]
    InsecureConnection,
    /// A redirect would have sent the token to another host or over plain `http`.
    #[error("Concept2 redirect was blocked (cross-origin or insecure)")]
    InsecureRedirectBlocked,
    /// The request exceeded its timeout.
    #[error("Concept2 request timed out")]
    Timeout,
    /// The response body was larger than the client is willing to read.
    #[error("Concept2 response body exceeds the {limit}-byte limit")]
    BodyTooLarge {
        /// The configured cap.
        limit: u64,
    },
    /// Transport failure, reported as a coarse redacted class.
    #[error("Concept2 request failed: {0}")]
    Transport(String),
    /// Malformed response payload. The message carries no payload bytes.
    #[error("Concept2 response could not be decoded: {0}")]
    Decode(String),
}

impl Concept2Error {
    /// Whether a sync should stop rather than repeat the failing request.
    ///
    /// Authentication, authorization and rate limiting are the conditions the
    /// sync coordinator aborts on; anything else is counted and skipped.
    #[must_use]
    pub fn should_abort_sync(&self) -> bool {
        match self {
            Concept2Error::Unauthorized
            | Concept2Error::Forbidden
            | Concept2Error::RateLimited { .. } => true,
            Concept2Error::Http { status } => matches!(status, 401 | 403 | 429),
            _ => false,
        }
    }
}

/// Read-only access to the athlete's logbook. rowplay never writes back.
pub trait Concept2Client: Send + Sync {
    /// Fetch one page of result summaries (newest first).
    ///
    /// `per_page` is the Concept2 `number` parameter (the API maximum is 250).
    fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error>;
    /// Fetch the full detail (strokes and splits) of one result.
    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error>;
    /// Fetch just the per-stroke samples of one result.
    ///
    /// `sport` is needed because the BikeErg reports stroke pace per 1000 m.
    fn strokes(&self, id: i64, sport: Sport) -> Result<Vec<Stroke>, Concept2Error>;
}

/// Deterministic client for tests: serves pages built from a detail list and
/// records every call.
#[derive(Debug, Default)]
pub struct MockConcept2Client {
    details: BTreeMap<i64, WorkoutDetail>,
    failure: Option<Concept2Error>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl MockConcept2Client {
    /// A client serving `details` (newest first).
    #[must_use]
    pub fn new(details: Vec<WorkoutDetail>) -> Self {
        MockConcept2Client {
            details: details.into_iter().map(|d| (d.id(), d)).collect(),
            failure: None,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Make every call fail with `error`.
    #[must_use]
    pub fn failing_with(mut self, error: Concept2Error) -> Self {
        self.failure = Some(error);
        self
    }

    /// Calls made so far, as `list:<page>` / `detail:<id>` / `strokes:<id>` strings.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls lock").clone()
    }

    fn record(&self, call: String) -> Result<(), Concept2Error> {
        self.calls.lock().expect("calls lock").push(call);
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
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
    fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error> {
        let page = page.max(1);
        self.record(format!("list:{page}"))?;
        if per_page == 0 {
            return Ok(ResultsPage {
                workouts: Vec::new(),
                page,
                total_pages: page,
            });
        }
        let ordered = self.ordered();
        let total_pages = ordered.len().div_ceil(per_page as usize).max(1) as u32;
        let start = (page as usize - 1) * per_page as usize;
        let workouts = ordered
            .iter()
            .skip(start)
            .take(per_page as usize)
            .map(|d| d.summary())
            .collect();
        Ok(ResultsPage {
            workouts,
            page,
            total_pages,
        })
    }

    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error> {
        self.record(format!("detail:{id}"))?;
        self.details
            .get(&id)
            .cloned()
            .ok_or(Concept2Error::NotFound(id))
    }

    fn strokes(&self, id: i64, _sport: Sport) -> Result<Vec<Stroke>, Concept2Error> {
        self.record(format!("strokes:{id}"))?;
        self.details
            .get(&id)
            .map(|detail| detail.strokes.clone())
            .ok_or(Concept2Error::NotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowplay_core::demo::demo_details;

    #[test]
    fn mock_pages_through_details_newest_first() {
        let client = MockConcept2Client::new(demo_details());
        let first = client.list_results(1, 5).unwrap();
        assert_eq!(first.page, 1);
        assert_eq!(first.total_pages, 4);
        assert_eq!(first.workouts.len(), 5);
        assert_eq!(first.workouts[0].id, 1001);
        let last = client.list_results(4, 5).unwrap();
        assert_eq!(last.workouts.len(), 2);
        assert!(client.list_results(9, 5).unwrap().workouts.is_empty());
        let detail = client.result_detail(1004).unwrap();
        assert!(!detail.strokes.is_empty());
        assert_eq!(
            client.strokes(1004, Sport::Rower).unwrap().len(),
            detail.strokes.len()
        );
        assert_eq!(client.result_detail(5), Err(Concept2Error::NotFound(5)));
        assert_eq!(
            client.calls(),
            vec![
                "list:1",
                "list:4",
                "list:9",
                "detail:1004",
                "strokes:1004",
                "detail:5",
            ]
        );
    }

    #[test]
    fn mock_returns_an_empty_page_for_a_zero_page_size() {
        let client = MockConcept2Client::new(demo_details());
        let page = client.list_results(0, 0).unwrap();
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 1);
        assert!(page.workouts.is_empty());
    }

    #[test]
    fn failures_are_privacy_safe() {
        let client = MockConcept2Client::new(vec![]).failing_with(Concept2Error::Unauthorized);
        assert_eq!(client.list_results(1, 10), Err(Concept2Error::Unauthorized));
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
        let empty = MockConcept2Client::new(vec![]);
        let page = empty.list_results(1, 10).unwrap();
        assert_eq!(page.total_pages, 1);
        assert!(page.workouts.is_empty());
    }

    #[test]
    fn abort_rules_match_studio() {
        for error in [
            Concept2Error::Unauthorized,
            Concept2Error::Forbidden,
            Concept2Error::RateLimited {
                retry_after_secs: None,
            },
            Concept2Error::Http { status: 401 },
            Concept2Error::Http { status: 403 },
            Concept2Error::Http { status: 429 },
        ] {
            assert!(error.should_abort_sync(), "{error}");
        }
        for error in [
            Concept2Error::NotFound(7),
            Concept2Error::Http { status: 404 },
            Concept2Error::Http { status: 500 },
            Concept2Error::Decode("malformed payload".into()),
            Concept2Error::Timeout,
            Concept2Error::Transport("I/O".into()),
        ] {
            assert!(!error.should_abort_sync(), "{error}");
        }
    }
}
