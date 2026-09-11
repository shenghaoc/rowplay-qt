// SPDX-License-Identifier: GPL-3.0-or-later
//! Blocking HTTPS client for the Concept2 Logbook API.
//!
//! Port of rowplay-studio's `Concept2/URLSessionConcept2Client.swift`,
//! `HTTPTransport.swift`, `Concept2Endpoint.swift` and `Concept2Error.swift`,
//! built on `ureq` (rustls) instead of `URLSession`. The web app's
//! `src/lib/server/concept2.ts` stays canonical for the request shapes.
//!
//! Security rules, all enforced here:
//!
//! - HTTPS only; plain `http` is accepted solely for `localhost`, `127.0.0.1`
//!   and `::1` so tests can point the client at a local server;
//! - automatic redirects are off — at most [`MAX_REDIRECTS`] are followed by
//!   hand, and only same-origin (`https`) ones; cross-host redirects and
//!   HTTPS→HTTP downgrades fail with [`Concept2Error::InsecureRedirectBlocked`]
//!   so the bearer token can never reach another host;
//! - bounded timeouts ([`REQUEST_TIMEOUT`] per request, [`OVERALL_TIMEOUT`] for
//!   the redirect chain) and a bounded response body ([`MAX_BODY_BYTES`]);
//! - the token lives in a [`SecretToken`], is only ever written into the
//!   `Authorization` header (never the URL), and no error or log line can
//!   contain it.

use std::fmt;
use std::time::{Duration, Instant};

use rowplay_core::concept2::{
    DetailResponse, RawStroke, StrokesResponse, SummaryResponse, assemble_detail, map_strokes,
    map_workout,
};
use rowplay_core::models::{Sport, Stroke, WorkoutDetail};
use rowplay_core::privacy::PrivacySafeLogger;

use super::{Concept2Client, Concept2Error, ResultsPage};
use crate::logging::logger;
use crate::token_store::SecretToken;

/// Production Concept2 Logbook base URL.
pub const DEFAULT_BASE_URL: &str = "https://log.concept2.com";

/// Explicit API version pin, as the Concept2 documentation recommends.
pub const ACCEPT: &str = "application/vnd.c2logbook.v1+json";

/// Largest response body the client will read, in bytes (25 MiB).
pub const MAX_BODY_BYTES: u64 = 25 * 1024 * 1024;

/// Redirects followed by hand before the response is treated as an error.
pub const MAX_REDIRECTS: u32 = 3;

/// Timeout for a single request/response exchange.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Budget for one logical fetch including its redirect chain.
pub const OVERALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Largest response header block accepted (defence in depth; ureq's default is
/// 128 KiB).
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// Client tunables.
///
/// [`HttpOptions::default`] is the production contract; tests narrow the
/// budgets so they never wait 30 seconds or move 25 MiB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpOptions {
    /// Timeout for one request/response exchange.
    pub request_timeout: Duration,
    /// Budget for one logical fetch, redirect chain included.
    pub overall_timeout: Duration,
    /// Redirects followed before giving up.
    pub max_redirects: u32,
    /// Largest response body accepted.
    pub max_body_bytes: u64,
}

impl Default for HttpOptions {
    fn default() -> Self {
        HttpOptions {
            request_timeout: REQUEST_TIMEOUT,
            overall_timeout: OVERALL_TIMEOUT,
            max_redirects: MAX_REDIRECTS,
            max_body_bytes: MAX_BODY_BYTES,
        }
    }
}

/// A small absolute-or-joinable HTTP(S) URI.
///
/// The Concept2 client only ever talks to URLs it built itself from a base URL;
/// the one piece of remote input is a `Location` header, which [`HttpUri::resolve`]
/// joins onto the URL that produced it. That is deliberately less than a full
/// URL parser, and [`redirect_target`] fails closed on anything it cannot
/// affirmatively classify, so no `url`/ICU dependency enters the desktop build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpUri {
    scheme: String,
    host: String,
    port: u16,
    path_and_query: String,
}

impl HttpUri {
    /// Parse an absolute `http`/`https` URL with an optional port and path.
    ///
    /// Rejects empty hosts, userinfo (`user:pass@host`), unknown schemes and
    /// any URL containing whitespace or control characters.
    pub fn parse(raw: &str) -> Result<Self, Concept2Error> {
        let raw = raw.trim();
        if raw.is_empty()
            || raw.chars().any(char::is_whitespace)
            || raw.chars().any(char::is_control)
        {
            return Err(Concept2Error::InvalidUrl);
        }
        let (scheme, rest) = raw.split_once("://").ok_or(Concept2Error::InvalidUrl)?;
        let scheme = scheme.to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "https" => 443,
            "http" => 80,
            _ => return Err(Concept2Error::InvalidUrl),
        };

        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        if authority.is_empty() || authority.contains('@') {
            return Err(Concept2Error::InvalidUrl);
        }
        let (host, port) = split_authority(authority, default_port)?;

        let mut path_and_query = tail.to_owned();
        if let Some(hash) = path_and_query.find('#') {
            path_and_query.truncate(hash);
        }
        if path_and_query.is_empty() {
            path_and_query.push('/');
        } else if path_and_query.starts_with('?') {
            path_and_query.insert(0, '/');
        }

        Ok(HttpUri {
            scheme,
            host,
            port,
            path_and_query,
        })
    }

    /// The URI as a request-URL string (brackets restored around IPv6 hosts).
    #[must_use]
    pub fn as_url(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        let default_port = if self.scheme == "https" { 443 } else { 80 };
        if self.port == default_port {
            format!("{}://{host}{}", self.scheme, self.path_and_query)
        } else {
            format!(
                "{}://{host}:{}{}",
                self.scheme, self.port, self.path_and_query
            )
        }
    }

    /// The lower-cased host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The effective port (defaults filled in).
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether this URI is HTTPS.
    #[must_use]
    pub fn is_secure(&self) -> bool {
        self.scheme == "https"
    }

    /// Whether the host is `localhost`, `127.0.0.1` or `::1`.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        matches!(self.host.as_str(), "localhost" | "127.0.0.1" | "::1")
    }

    /// Same host and effective port (scheme is checked separately by
    /// [`redirect_target`]).
    #[must_use]
    pub fn origin_matches(&self, other: &HttpUri) -> bool {
        self.host == other.host && self.port == other.port
    }

    /// Append a path (and query) to this URI's base path.
    #[must_use]
    pub fn join(&self, path_and_query: &str) -> HttpUri {
        let mut uri = self.clone();
        let base_path = self.path_and_query.split(['?', '#']).next().unwrap_or("/");
        let base_path = base_path.trim_end_matches('/');
        let suffix = if path_and_query.starts_with('/') {
            path_and_query.to_owned()
        } else {
            format!("/{path_and_query}")
        };
        uri.path_and_query = format!("{base_path}{suffix}");
        uri
    }

    /// Resolve a `Location` header value against this URI.
    ///
    /// Handles absolute URLs, protocol-relative (`//host/…`) URLs, absolute
    /// paths and relative paths.
    ///
    /// Fail closed on the shapes other URL parsers read differently, because a
    /// disagreement here is a token-leak vector:
    ///
    /// - whitespace and control characters are rejected;
    /// - a backslash is rejected — WHATWG parsers fold it to `/`, so
    ///   `https:\\evil.example` is `https://evil.example` to a browser;
    /// - an at-sign is rejected anywhere, since it can be read as userinfo;
    /// - a resolved path with a `..` segment or an interior `//` is rejected
    ///   rather than normalised, because a server or proxy may re-read it as an
    ///   authority.
    pub fn resolve(&self, location: &str) -> Result<HttpUri, Concept2Error> {
        let location = location.trim();
        if location.is_empty()
            || location.chars().any(char::is_whitespace)
            || location.chars().any(char::is_control)
            || location.contains(['\\', '@'])
        {
            return Err(Concept2Error::InvalidUrl);
        }

        let candidate = if let Some(rest) = location.strip_prefix("//") {
            HttpUri::parse(&format!("{}://{rest}", self.scheme))?
        } else if location.contains("://") {
            HttpUri::parse(location)?
        } else {
            let base_path = self.path_and_query.split(['?', '#']).next().unwrap_or("/");
            let mut path_and_query = if location.starts_with('/') {
                location.to_owned()
            } else {
                let dir = match base_path.rfind('/') {
                    Some(index) => &base_path[..=index],
                    None => "/",
                };
                format!("{dir}{location}")
            };
            if let Some(hash) = path_and_query.find('#') {
                path_and_query.truncate(hash);
            }

            let mut uri = self.clone();
            uri.path_and_query = path_and_query;
            uri
        };

        let path = candidate
            .path_and_query
            .split(['?', '#'])
            .next()
            .unwrap_or("/");
        if path.contains("//") || path.split('/').any(|segment| segment == "..") {
            return Err(Concept2Error::InvalidUrl);
        }

        Ok(candidate)
    }
}

/// Split `host[:port]`, handling bracketed IPv6 literals.
fn split_authority(authority: &str, default_port: u16) -> Result<(String, u16), Concept2Error> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or(Concept2Error::InvalidUrl)?;
        let port = match tail {
            "" => default_port,
            _ => tail
                .strip_prefix(':')
                .ok_or(Concept2Error::InvalidUrl)?
                .parse::<u16>()
                .map_err(|_| Concept2Error::InvalidUrl)?,
        };
        if host.is_empty() {
            return Err(Concept2Error::InvalidUrl);
        }
        return Ok((host.to_ascii_lowercase(), port));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>().map_err(|_| Concept2Error::InvalidUrl)?,
        ),
        None => (authority, default_port),
    };
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
    {
        return Err(Concept2Error::InvalidUrl);
    }
    Ok((host.to_ascii_lowercase(), port))
}

/// Decide whether a redirect from `base` to `location` may be followed.
///
/// Fail closed: the target must be resolvable, must not downgrade an HTTPS
/// origin to plain `http`, must be HTTPS unless it is a loopback host (the same
/// exemption the initial request uses) and must share the origin (host plus
/// effective port, compared case-insensitively). Anything else is
/// [`Concept2Error::InsecureRedirectBlocked`].
pub fn redirect_target(base: &HttpUri, location: &str) -> Result<HttpUri, Concept2Error> {
    let target = base
        .resolve(location)
        .map_err(|_| Concept2Error::InsecureRedirectBlocked)?;
    if base.is_secure() && !target.is_secure() {
        return Err(Concept2Error::InsecureRedirectBlocked);
    }
    if !target.is_secure() && !target.is_loopback() {
        return Err(Concept2Error::InsecureRedirectBlocked);
    }
    if !base.origin_matches(&target) {
        return Err(Concept2Error::InsecureRedirectBlocked);
    }
    Ok(target)
}

/// The `Authorization` header value, zeroed when it goes out of scope.
struct BearerHeader(Vec<u8>);

impl BearerHeader {
    fn new(token: &SecretToken) -> Self {
        let mut value = b"Bearer ".to_vec();
        value.extend_from_slice(token.expose().as_bytes());
        BearerHeader(value)
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for BearerHeader {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// Blocking Concept2 Logbook client.
pub struct Concept2HttpClient {
    base: HttpUri,
    token: SecretToken,
    options: HttpOptions,
    agent: ureq::Agent,
    logger: PrivacySafeLogger<'static>,
}

impl fmt::Debug for Concept2HttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Concept2HttpClient")
            .field("base", &self.base.as_url())
            .field("token", &self.token)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl Concept2HttpClient {
    /// A client for the production logbook with the default budgets.
    pub fn new(token: SecretToken) -> Result<Self, Concept2Error> {
        Self::with_base_url(DEFAULT_BASE_URL, token)
    }

    /// A client for `base_url` with the default budgets.
    pub fn with_base_url(base_url: &str, token: SecretToken) -> Result<Self, Concept2Error> {
        Self::with_options(base_url, token, HttpOptions::default())
    }

    /// A client for `base_url` with explicit budgets.
    pub fn with_options(
        base_url: &str,
        token: SecretToken,
        options: HttpOptions,
    ) -> Result<Self, Concept2Error> {
        let base = HttpUri::parse(base_url)?;
        if !base.is_secure() && !base.is_loopback() {
            return Err(Concept2Error::InsecureConnection);
        }

        let config = ureq::Agent::config_builder()
            // Statuses are classified here, not by the transport.
            .http_status_as_error(false)
            // Redirects are followed by hand so the origin rules are ours.
            .max_redirects(0)
            .timeout_per_call(Some(options.request_timeout))
            .timeout_global(Some(options.overall_timeout))
            .max_response_header_size(MAX_HEADER_BYTES)
            .user_agent(concat!("rowplay-qt/", env!("CARGO_PKG_VERSION")))
            .build();

        Ok(Concept2HttpClient {
            base,
            token,
            options,
            agent: config.into(),
            logger: logger("concept2-client"),
        })
    }

    /// The API base URL this client was built for.
    #[must_use]
    pub fn base_url(&self) -> String {
        self.base.as_url()
    }

    /// Fetch one path, following at most `max_redirects` same-origin redirects.
    fn send(&self, path_and_query: &str) -> Result<Vec<u8>, Concept2Error> {
        let started = Instant::now();
        let mut current = self.base.join(path_and_query);
        let mut followed = 0;
        let bearer = BearerHeader::new(&self.token);

        loop {
            if started.elapsed() >= self.options.overall_timeout {
                return Err(Concept2Error::Timeout);
            }

            let response = self
                .agent
                .get(&current.as_url())
                .header("Authorization", bearer.as_bytes())
                .header("Accept", ACCEPT)
                .call()
                .map_err(|error| map_transport_error(&error))?;

            let status = response.status().as_u16();

            if (300..400).contains(&status) {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok());
                let Some(location) = location else {
                    // A redirect without a target is just a failing response.
                    return Err(Concept2Error::Http { status });
                };
                if followed >= self.options.max_redirects {
                    return Err(Concept2Error::Http { status });
                }
                current = redirect_target(&current, location)?;
                followed += 1;
                continue;
            }

            if !(200..300).contains(&status) {
                return Err(map_status(status, response.headers().get("retry-after")));
            }

            return read_body(response, self.options.max_body_bytes);
        }
    }

    /// Fetch and decode one JSON path through the core payload parsers.
    fn send_json<T>(
        &self,
        path_and_query: &str,
        decode: impl FnOnce(&[u8]) -> Option<T>,
    ) -> Result<T, Concept2Error> {
        let bytes = self.send(path_and_query)?;
        decode(&bytes).ok_or_else(|| Concept2Error::Decode("malformed payload".to_owned()))
    }

    fn fetch_raw_strokes(&self, id: i64) -> Result<Vec<RawStroke>, Concept2Error> {
        let path = format!("/api/users/me/results/{id}/strokes");
        self.send_json(&path, |bytes| {
            StrokesResponse::from_slice(bytes)
                .ok()
                .map(|response| response.data)
        })
    }
}

impl Concept2Client for Concept2HttpClient {
    fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error> {
        let page = page.max(1);
        if per_page == 0 {
            return Ok(ResultsPage {
                workouts: Vec::new(),
                page,
                total_pages: page,
            });
        }
        let path = format!("/api/users/me/results?page={page}&number={per_page}");
        let response = self.send_json(&path, |bytes| SummaryResponse::from_slice(bytes).ok())?;
        // The web app defaults the page count to the page it just fetched, so a
        // response without `meta.pagination` ends the paging loop.
        let total_pages = response
            .meta
            .and_then(|meta| meta.pagination)
            .and_then(|pagination| pagination.total_pages)
            .unwrap_or(page);
        let workouts = response.data.iter().map(map_workout).collect();
        Ok(ResultsPage {
            workouts,
            page,
            total_pages,
        })
    }

    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error> {
        let path = format!("/api/users/me/results/{id}?include=metadata");
        let response = self.send_json(&path, |bytes| DetailResponse::from_slice(bytes).ok())?;

        // Studio fetches strokes only when the result advertises them, and a
        // stroke failure is not fatal: the timeline is synthesised from splits.
        let raw_strokes = if response.data.stroke_data.unwrap_or(false) {
            if let Ok(strokes) = self.fetch_raw_strokes(id) {
                Some(strokes)
            } else {
                self.logger.warn(
                    "Concept2 stroke fetch failed; falling back to synthesised strokes",
                    &[&id as &dyn fmt::Display],
                );
                None
            }
        } else {
            None
        };

        Ok(assemble_detail(&response, raw_strokes.as_deref()))
    }

    fn strokes(&self, id: i64, sport: Sport) -> Result<Vec<Stroke>, Concept2Error> {
        let raw = self.fetch_raw_strokes(id)?;
        Ok(map_strokes(&raw, sport))
    }
}

/// Read a bounded response body.
fn read_body(
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
) -> Result<Vec<u8>, Concept2Error> {
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| match error {
            ureq::Error::BodyExceedsLimit(_) => Concept2Error::BodyTooLarge { limit },
            other => map_transport_error(&other),
        })
}

/// Map an HTTP status onto a typed error.
fn map_status(status: u16, retry_after: Option<&ureq::http::HeaderValue>) -> Concept2Error {
    match status {
        401 => Concept2Error::Unauthorized,
        403 => Concept2Error::Forbidden,
        429 => Concept2Error::RateLimited {
            retry_after_secs: retry_after.and_then(parse_retry_after),
        },
        other => Concept2Error::Http { status: other },
    }
}

/// `Retry-After` is seconds in the Concept2 responses; HTTP-dates are ignored.
fn parse_retry_after(value: &ureq::http::HeaderValue) -> Option<u64> {
    value.to_str().ok()?.trim().parse::<u64>().ok()
}

/// Coarse, privacy-safe classification of a transport failure.
///
/// The underlying error is deliberately not rendered: it can carry URL or
/// request detail, and a hostile peer could try to make it echo a credential.
fn map_transport_error(error: &ureq::Error) -> Concept2Error {
    if matches!(error, ureq::Error::Timeout(_)) {
        return Concept2Error::Timeout;
    }
    Concept2Error::Transport(transport_class(error).to_owned())
}

/// A short static label for a transport failure class.
fn transport_class(error: &ureq::Error) -> &'static str {
    match error {
        ureq::Error::StatusCode(_) => "unexpected status",
        ureq::Error::Http(_) => "invalid request",
        ureq::Error::BadUri(_) => "invalid URI",
        ureq::Error::Protocol(_) => "protocol error",
        ureq::Error::Io(_) => "I/O error",
        ureq::Error::Timeout(_) => "timeout",
        ureq::Error::HostNotFound => "host not found",
        ureq::Error::RedirectFailed => "redirect failed",
        ureq::Error::InvalidProxyUrl => "invalid proxy URL",
        ureq::Error::ConnectionFailed => "connection failed",
        ureq::Error::BodyExceedsLimit(_) => "body exceeds limit",
        ureq::Error::TooManyRedirects => "too many redirects",
        _ => "transport error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::mpsc::{self, Receiver};
    use std::thread;

    /// A token distinctive enough that any leak is unmistakable.
    const TOKEN: &str = "test-token-abc123";
    const BEARER: &str = "Bearer test-token-abc123";

    const DETAIL_BODY: &str = r#"{"data":{"id":9001,"date":"2026-05-15 07:30:00","type":"rower","distance":2000,"time":4500,"stroke_data":true,"workout":{"splits":[{"distance":500,"time":1125,"stroke_rate":32}]}}}"#;

    fn summary_body(total_pages: u32) -> String {
        format!(
            r#"{{"data":[{{"id":9001,"date":"2026-05-15 07:30:00","type":"rower","distance":2000,"time":4500,"stroke_data":true}}],"meta":{{"pagination":{{"total_pages":{total_pages}}}}}}}"#
        )
    }

    // ---- a tiny loopback HTTP server -------------------------------------

    struct Canned {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
        delay: Duration,
    }

    impl Canned {
        fn ok(body: &str) -> Self {
            Canned {
                status: 200,
                headers: Vec::new(),
                body: body.to_owned(),
                delay: Duration::ZERO,
            }
        }

        fn status(status: u16) -> Self {
            Canned {
                status,
                headers: Vec::new(),
                body: String::new(),
                delay: Duration::ZERO,
            }
        }

        fn redirect(status: u16, location: &str) -> Self {
            Canned {
                status,
                headers: vec![("Location".to_owned(), location.to_owned())],
                body: String::new(),
                delay: Duration::ZERO,
            }
        }

        fn with_header(mut self, name: &str, value: &str) -> Self {
            self.headers.push((name.to_owned(), value.to_owned()));
            self
        }

        fn with_delay(mut self, delay: Duration) -> Self {
            self.delay = delay;
            self
        }
    }

    fn reason(status: u16) -> &'static str {
        match status {
            200 => "OK",
            302 => "Found",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "Status",
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match stream.read(&mut byte) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    buffer.push(byte[0]);
                    if buffer.ends_with(b"\r\n\r\n") || buffer.len() > 32 * 1024 {
                        break;
                    }
                }
            }
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }

    fn write_response(stream: &mut TcpStream, canned: &Canned) -> io::Result<()> {
        let mut head = String::new();
        let _ = write!(
            head,
            "HTTP/1.1 {} {}\r\n",
            canned.status,
            reason(canned.status)
        );
        for (name, value) in &canned.headers {
            let _ = write!(head, "{name}: {value}\r\n");
        }
        let _ = write!(head, "Content-Length: {}\r\n", canned.body.len());
        head.push_str("Connection: close\r\n\r\n");
        stream.write_all(head.as_bytes())?;
        stream.write_all(canned.body.as_bytes())?;
        stream.flush()
    }

    /// Serves `responses` in order, one per connection, then shuts down.
    ///
    /// `Connection: close` plus a shutdown after each response keeps `ureq` from
    /// reusing a socket, so one accepted connection is one client request. The
    /// accept loop is non-blocking with a deadline so a short-circuited client
    /// can never hang the test.
    struct TestServer {
        base_url: String,
        requests: Receiver<String>,
    }

    impl TestServer {
        fn start(responses: Vec<Canned>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            listener
                .set_nonblocking(true)
                .expect("non-blocking listener");
            let addr = listener.local_addr().expect("local addr");
            let (sender, requests) = mpsc::channel();
            // Detached on purpose: a client that short-circuits (a blocked
            // redirect, an exhausted budget) leaves responses unserved, and the
            // test must not wait for them. The idle deadline ends the thread.
            thread::spawn(move || {
                for canned in responses {
                    let deadline = Instant::now() + Duration::from_secs(2);
                    let stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break Some(stream),
                            Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                                if Instant::now() > deadline {
                                    break None;
                                }
                                thread::sleep(Duration::from_millis(2));
                            }
                            Err(_) => break None,
                        }
                    };
                    let Some(mut stream) = stream else { return };
                    // A non-blocking listener hands out non-blocking sockets on
                    // macOS; the request reader wants blocking reads.
                    let _ = stream.set_nonblocking(false);
                    let _ = sender.send(read_request(&mut stream));
                    if !canned.delay.is_zero() {
                        thread::sleep(canned.delay);
                    }
                    let _ = write_response(&mut stream, &canned);
                    let _ = stream.shutdown(Shutdown::Both);
                }
            });
            TestServer {
                base_url: format!("http://{addr}"),
                requests,
            }
        }

        fn requests(&self) -> Vec<String> {
            let mut out = Vec::new();
            while let Ok(request) = self.requests.try_recv() {
                out.push(request);
            }
            out
        }
    }

    fn test_client(base_url: &str) -> Concept2HttpClient {
        Concept2HttpClient::with_options(
            base_url,
            SecretToken::new(TOKEN).expect("token"),
            HttpOptions {
                request_timeout: Duration::from_millis(500),
                overall_timeout: Duration::from_secs(5),
                ..HttpOptions::default()
            },
        )
        .expect("client")
    }

    fn request_line(request: &str) -> &str {
        request.lines().next().unwrap_or_default()
    }

    fn header_value(request: &str, name: &str) -> Option<String> {
        request.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    }

    // ---- URI parsing and the redirect policy -----------------------------

    #[test]
    fn parses_absolute_uris() {
        let uri = HttpUri::parse("https://log.concept2.com/api/users/me/results?page=1").unwrap();
        assert_eq!(uri.host(), "log.concept2.com");
        assert_eq!(uri.port(), 443);
        assert!(uri.is_secure());
        assert!(!uri.is_loopback());
        assert_eq!(
            uri.as_url(),
            "https://log.concept2.com/api/users/me/results?page=1"
        );

        let local = HttpUri::parse("http://127.0.0.1:8080").unwrap();
        assert_eq!(local.port(), 8080);
        assert!(local.is_loopback());
        assert!(!local.is_secure());
        assert_eq!(local.as_url(), "http://127.0.0.1:8080/");

        let v6 = HttpUri::parse("http://[::1]:9000/api").unwrap();
        assert_eq!(v6.host(), "::1");
        assert_eq!(v6.port(), 9000);
        assert!(v6.is_loopback());
        assert_eq!(v6.as_url(), "http://[::1]:9000/api");

        let default_port = HttpUri::parse("https://example.com:443/a").unwrap();
        assert_eq!(default_port.as_url(), "https://example.com/a");

        let mixed_case = HttpUri::parse("HTTPS://LOG.Concept2.com/Api").unwrap();
        assert_eq!(mixed_case.host(), "log.concept2.com");
        assert_eq!(mixed_case.as_url(), "https://log.concept2.com/Api");
    }

    #[test]
    fn rejects_malformed_uris() {
        for raw in [
            "",
            "log.concept2.com/api",
            "ftp://log.concept2.com/api",
            "https://",
            "https://user:pass@log.concept2.com/api",
            "https://log.concept2.com/a b",
            "https://log.concept2.com:notaport/x",
            "https://log.concept2.com:99999/x",
        ] {
            assert_eq!(HttpUri::parse(raw), Err(Concept2Error::InvalidUrl), "{raw}");
        }
    }

    #[test]
    fn resolves_relative_locations() {
        let base = HttpUri::parse("https://log.concept2.com/api/users/me/results?page=1").unwrap();
        assert_eq!(
            base.resolve("/api/users/me/results?page=2")
                .unwrap()
                .as_url(),
            "https://log.concept2.com/api/users/me/results?page=2"
        );
        assert_eq!(
            base.resolve("results?page=3").unwrap().as_url(),
            "https://log.concept2.com/api/users/me/results?page=3"
        );
        // Fragments are dropped; a protocol-relative URL adopts our scheme but
        // keeps its own host (which the origin check then rejects).
        assert_eq!(
            base.resolve("https://log.concept2.com/other#frag")
                .unwrap()
                .as_url(),
            "https://log.concept2.com/other"
        );
        let sneaky = base.resolve("//evil.example/x").unwrap();
        assert_eq!(sneaky.host(), "evil.example");
        assert!(!base.origin_matches(&sneaky));
        for bad in ["", "/a b", "https://user@host/x", "//host\u{7f}/x"] {
            assert!(base.resolve(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn redirect_policy_is_fail_closed() {
        let secure = HttpUri::parse("https://log.concept2.com/api").unwrap();
        for location in [
            "https://log.concept2.com/next",
            "/next",
            "next",
            "https://LOG.CONCEPT2.COM/next",
        ] {
            assert!(
                redirect_target(&secure, location).is_ok(),
                "{location} should be allowed"
            );
        }
        for location in [
            "https://evil.example/next",
            "//evil.example/next",
            "http://log.concept2.com/next",
            "https://log.concept2.com:8443/next",
            "https://user@log.concept2.com/next",
            "not a url",
            "",
        ] {
            assert_eq!(
                redirect_target(&secure, location),
                Err(Concept2Error::InsecureRedirectBlocked),
                "{location} must be blocked"
            );
        }

        // The loopback exemption mirrors the initial-request rule, so a local
        // test server can exercise the allowed path without TLS.
        let local = HttpUri::parse("http://127.0.0.1:8080/api").unwrap();
        assert!(
            redirect_target(&local, "http://127.0.0.1:8080/next").is_ok(),
            "same-origin loopback redirect"
        );
        for location in [
            "http://localhost:8080/next",
            "http://127.0.0.1:9090/next",
            "https://evil.example/next",
        ] {
            assert_eq!(
                redirect_target(&local, location),
                Err(Concept2Error::InsecureRedirectBlocked),
                "{location} must be blocked"
            );
        }
    }

    #[test]
    fn insecure_base_urls_are_rejected() {
        let token = SecretToken::new(TOKEN).unwrap();
        assert_eq!(
            Concept2HttpClient::with_base_url("http://log.concept2.com", token.clone()).err(),
            Some(Concept2Error::InsecureConnection)
        );
        assert_eq!(
            Concept2HttpClient::with_base_url("ftp://log.concept2.com", token.clone()).err(),
            Some(Concept2Error::InvalidUrl)
        );
        for base_url in ["http://127.0.0.1:1", "http://localhost:1", "http://[::1]:1"] {
            assert!(
                Concept2HttpClient::with_base_url(base_url, token.clone()).is_ok(),
                "{base_url}"
            );
        }
        assert!(Concept2HttpClient::new(token).is_ok());
    }

    #[test]
    fn redirect_policy_rejects_locations_other_parsers_read_differently() {
        let secure = HttpUri::parse("https://log.concept2.com/api").unwrap();

        // Each of these stays on our host as far as `HttpUri` is concerned, so
        // only the "reject what we cannot classify" rule keeps them out. A
        // WHATWG parser folds `\` to `/` and reads `https:\\evil.example` and
        // `\/\/evil.example` as *that* host.
        for location in [
            // Backslash folding: a browser sees another host entirely.
            "https:\\\\evil.example",
            "https:\\/evil.example",
            "\\/\\/evil.example/next",
            "/next\\..\\..\\evil.example",
            // An at-sign anywhere is a userinfo question, so it is refused.
            "https://log.concept2.com#@evil.example",
            "/next#@evil.example",
            // Path confusion: `..` and an interior `//` are what a proxy may
            // re-read as an authority.
            "https://log.concept2.com/../..//evil.example",
            "/../..//evil.example",
            "//third-party.example/next",
        ] {
            assert_eq!(
                redirect_target(&secure, location),
                Err(Concept2Error::InsecureRedirectBlocked),
                "{location} must be blocked"
            );
        }

        // A trailing dot and a percent-encoded host are different names to the
        // origin comparison even before any character rule applies.
        for location in [
            "https://log.concept2.com./next",
            "https://log.concept2.com%2e.evil.com/next",
            "https://evil%2ecom/next",
            "https://log.concept2.com.:443/next",
        ] {
            assert_eq!(
                redirect_target(&secure, location),
                Err(Concept2Error::InsecureRedirectBlocked),
                "{location} must be blocked"
            );
        }

        // The plain forms keep working, including a protocol-relative URL that
        // names our own host and a path-only redirect.
        for location in [
            "/api/users/me/results?page=2",
            "results?page=2",
            "//log.concept2.com/next",
            "https://LOG.concept2.com/next",
        ] {
            assert!(
                redirect_target(&secure, location).is_ok(),
                "{location} should be allowed"
            );
        }
    }

    #[test]
    fn resolve_keeps_the_authority_it_started_with() {
        let base = HttpUri::parse("https://log.concept2.com/api/users/me/results?page=1").unwrap();
        // Whatever the shape of the Location, the host and port are ours.
        for location in [
            "/next",
            "next",
            "?page=2",
            "https://log.concept2.com/next",
            "//log.concept2.com/next",
        ] {
            let resolved = base.resolve(location).unwrap_or_else(|error| {
                panic!("{location}: {error}");
            });
            assert_eq!(resolved.host(), "log.concept2.com", "{location}");
            assert_eq!(resolved.port(), 443, "{location}");
            assert!(resolved.is_secure(), "{location}");
            // The resolved URL never carries the original host in a position a
            // peer could re-parse as an authority.
            assert!(!resolved.as_url().contains('@'), "{location}");
            assert!(!resolved.as_url().contains('\\'), "{location}");
        }
    }

    #[test]
    fn production_budgets_match_studio() {
        assert_eq!(DEFAULT_BASE_URL, "https://log.concept2.com");
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(30));
        assert_eq!(OVERALL_TIMEOUT, Duration::from_secs(300));
        assert_eq!(MAX_REDIRECTS, 3);
        assert_eq!(MAX_BODY_BYTES, 25 * 1024 * 1024);
        assert_eq!(ACCEPT, "application/vnd.c2logbook.v1+json");
        assert_eq!(
            HttpOptions::default(),
            HttpOptions {
                request_timeout: REQUEST_TIMEOUT,
                overall_timeout: OVERALL_TIMEOUT,
                max_redirects: MAX_REDIRECTS,
                max_body_bytes: MAX_BODY_BYTES,
            }
        );
    }

    // ---- request shape and status mapping --------------------------------

    #[test]
    fn list_results_requests_the_web_shape() {
        let server = TestServer::start(vec![Canned::ok(&summary_body(3))]);
        let client = test_client(&server.base_url);
        let page = client.list_results(2, 250).unwrap();

        assert_eq!(page.page, 2);
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.workouts.len(), 1);
        assert_eq!(page.workouts[0].id, 9001);
        assert_eq!(page.workouts[0].sport, Sport::Rower);
        assert_eq!(page.workouts[0].time, 450.0);
        assert_eq!(page.workouts[0].pace, 112.5);

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            request_line(&requests[0]),
            "GET /api/users/me/results?page=2&number=250 HTTP/1.1"
        );
        assert_eq!(
            header_value(&requests[0], "authorization").as_deref(),
            Some(BEARER)
        );
        assert_eq!(
            header_value(&requests[0], "accept").as_deref(),
            Some(ACCEPT)
        );
    }

    #[test]
    fn a_missing_pagination_envelope_ends_the_paging_loop() {
        let server = TestServer::start(vec![Canned::ok(r#"{"data":[]}"#)]);
        let client = test_client(&server.base_url);
        let page = client.list_results(1, 250).unwrap();
        assert_eq!(page.total_pages, 1);
        assert!(page.workouts.is_empty());
    }

    #[test]
    fn maps_every_status_code() {
        for (status, expected) in [
            (401, Concept2Error::Unauthorized),
            (403, Concept2Error::Forbidden),
            (404, Concept2Error::Http { status: 404 }),
            (500, Concept2Error::Http { status: 500 }),
        ] {
            let server = TestServer::start(vec![Canned::status(status)]);
            let client = test_client(&server.base_url);
            assert_eq!(
                client.list_results(1, 10).unwrap_err(),
                expected,
                "HTTP {status}"
            );
        }
    }

    #[test]
    fn rate_limiting_carries_retry_after() {
        let server =
            TestServer::start(vec![Canned::status(429).with_header("Retry-After", " 30 ")]);
        let client = test_client(&server.base_url);
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::RateLimited {
                retry_after_secs: Some(30)
            }
        );
    }

    #[test]
    fn a_rate_limit_without_a_numeric_retry_after_still_maps() {
        let server = TestServer::start(vec![
            Canned::status(429).with_header("Retry-After", "Wed, 21 Oct 2026 07:28:00 GMT"),
        ]);
        let client = test_client(&server.base_url);
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::RateLimited {
                retry_after_secs: None
            }
        );
    }

    #[test]
    fn malformed_payloads_are_reported_without_echoing_them() {
        let server = TestServer::start(vec![Canned::ok(r#"{"data":{"id":"#)]);
        let client = test_client(&server.base_url);
        let error = client.list_results(1, 10).unwrap_err();
        assert_eq!(error, Concept2Error::Decode("malformed payload".to_owned()));
        assert!(!format!("{error} {error:?}").contains("id"));
    }

    // ---- redirects, timeouts and the body cap ----------------------------

    #[test]
    fn follows_a_same_origin_redirect_and_keeps_the_token_there() {
        let server = TestServer::start(vec![
            Canned::redirect(302, "/api/users/me/results?page=2&number=10"),
            Canned::ok(&summary_body(3)),
        ]);
        let client = test_client(&server.base_url);
        let page = client.list_results(1, 10).unwrap();
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 3);

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(
                header_value(request, "authorization").as_deref(),
                Some(BEARER),
                "the token rides along on a same-origin redirect"
            );
        }
        assert_eq!(
            request_line(&requests[1]),
            "GET /api/users/me/results?page=2&number=10 HTTP/1.1"
        );
    }

    #[test]
    fn blocks_a_cross_host_redirect_without_sending_the_token() {
        let server = TestServer::start(vec![Canned::redirect(
            302,
            "https://evil.example/api/users/me/results",
        )]);
        let client = test_client(&server.base_url);
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::InsecureRedirectBlocked
        );
        // Only the original request happened: the token never left this host.
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn blocks_a_plain_http_downgrade_redirect() {
        let server = TestServer::start(vec![Canned::redirect(302, "http://example.com/next")]);
        let client = test_client(&server.base_url);
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::InsecureRedirectBlocked
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn stops_after_the_redirect_budget() {
        let server = TestServer::start(vec![
            Canned::redirect(302, "/a"),
            Canned::redirect(302, "/b"),
        ]);
        let client = Concept2HttpClient::with_options(
            &server.base_url,
            SecretToken::new(TOKEN).unwrap(),
            HttpOptions {
                max_redirects: 1,
                ..HttpOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::Http { status: 302 }
        );
        assert_eq!(server.requests().len(), 2);
    }

    #[test]
    fn a_redirect_without_a_target_is_a_plain_status_error() {
        let server = TestServer::start(vec![Canned::status(302)]);
        let client = test_client(&server.base_url);
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::Http { status: 302 }
        );
    }

    #[test]
    fn times_out_on_a_stalled_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                thread::sleep(Duration::from_secs(3));
                drop(stream);
            }
        });
        let client = Concept2HttpClient::with_options(
            &format!("http://{addr}"),
            SecretToken::new(TOKEN).unwrap(),
            HttpOptions {
                request_timeout: Duration::from_millis(200),
                overall_timeout: Duration::from_secs(30),
                ..HttpOptions::default()
            },
        )
        .unwrap();
        let started = Instant::now();
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::Timeout
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the 30 s production timeout must be configurable"
        );
    }

    #[test]
    fn enforces_the_overall_budget_across_a_redirect_chain() {
        // Each hop is comfortably inside the per-request timeout but the chain
        // as a whole must stop at the overall budget: ureq's global timeout is
        // per call, so only the client's own deadline bounds the chain.
        let hop = Duration::from_millis(120);
        let overall = Duration::from_millis(250);
        let server = TestServer::start(vec![
            Canned::redirect(302, "/a").with_delay(hop),
            Canned::redirect(302, "/b").with_delay(hop),
            Canned::redirect(302, "/c").with_delay(hop),
        ]);
        let client = Concept2HttpClient::with_options(
            &server.base_url,
            SecretToken::new(TOKEN).unwrap(),
            HttpOptions {
                request_timeout: Duration::from_secs(5),
                overall_timeout: overall,
                ..HttpOptions::default()
            },
        )
        .unwrap();

        let started = Instant::now();
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::Timeout
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed >= overall,
            "the overall budget must actually be reached: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "the chain must stop well before the per-request timeouts: {elapsed:?}"
        );
        assert!(server.requests().len() <= 3);
    }

    #[test]
    fn rejects_a_body_over_the_cap() {
        let server = TestServer::start(vec![Canned::ok(&"x".repeat(4096))]);
        let client = Concept2HttpClient::with_options(
            &server.base_url,
            SecretToken::new(TOKEN).unwrap(),
            HttpOptions {
                max_body_bytes: 1024,
                ..HttpOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            client.list_results(1, 10).unwrap_err(),
            Concept2Error::BodyTooLarge { limit: 1024 }
        );
    }

    // ---- detail assembly and token privacy ------------------------------

    #[test]
    fn result_detail_fetches_strokes_when_the_result_advertises_them() {
        let server = TestServer::start(vec![
            Canned::ok(DETAIL_BODY),
            Canned::ok(
                r#"{"data":[{"t":0,"d":0,"p":1080,"spm":32},{"t":12,"d":85,"p":1075,"spm":32}]}"#,
            ),
        ]);
        let client = test_client(&server.base_url);
        let detail = client.result_detail(9001).unwrap();

        assert_eq!(detail.id(), 9001);
        assert_eq!(detail.workout.sport, Sport::Rower);
        assert_eq!(detail.workout.time, 450.0);
        assert_eq!(detail.workout.pace, 112.5);
        assert!(detail.workout.has_stroke_data);
        assert_eq!(detail.strokes.len(), 2);
        assert_eq!(detail.strokes[1].t, 1.2);
        assert_eq!(detail.strokes[1].d, 8.5);
        assert_eq!(detail.strokes[1].pace, 107.5);
        assert_eq!(detail.splits.len(), 1);
        assert_eq!(detail.splits[0].pace, 112.5);

        let requests = server.requests();
        assert_eq!(
            request_line(&requests[0]),
            "GET /api/users/me/results/9001?include=metadata HTTP/1.1"
        );
        assert_eq!(
            request_line(&requests[1]),
            "GET /api/users/me/results/9001/strokes HTTP/1.1"
        );
    }

    #[test]
    fn a_failed_stroke_fetch_falls_back_to_synthesised_strokes() {
        let server = TestServer::start(vec![Canned::ok(DETAIL_BODY), Canned::status(500)]);
        let client = test_client(&server.base_url);
        let detail = client.result_detail(9001).unwrap();
        assert_eq!(detail.strokes.len(), 2, "origin plus one segment per split");
        assert!(!detail.workout.has_stroke_data);
    }

    #[test]
    fn a_result_without_stroke_data_never_fetches_strokes() {
        let server = TestServer::start(vec![Canned::ok(
            r#"{"data":{"id":9002,"date":"2026-05-16 08:00:00","type":"bike","distance":4000,"time":9600}}"#,
        )]);
        let client = test_client(&server.base_url);
        let detail = client.result_detail(9002).unwrap();
        assert!(!detail.workout.has_stroke_data);
        assert_eq!(detail.strokes.len(), 61, "60-step summary ramp");
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn the_strokes_endpoint_is_available_on_its_own() {
        let server = TestServer::start(vec![Canned::ok(
            r#"{"data":[{"t":15,"d":120,"p":2000,"spm":28}]}"#,
        )]);
        let client = test_client(&server.base_url);
        let strokes = client.strokes(9002, Sport::Bike).unwrap();
        assert_eq!(strokes.len(), 1);
        assert_eq!(strokes[0].t, 1.5);
        assert_eq!(strokes[0].d, 12.0);
        assert_eq!(strokes[0].pace, 100.0, "BikeErg per-1000m pace is halved");
        assert_eq!(
            request_line(&server.requests()[0]),
            "GET /api/users/me/results/9002/strokes HTTP/1.1"
        );
    }

    #[test]
    fn the_token_never_reaches_an_error_a_log_line_or_debug() {
        for error in [
            Concept2Error::Unauthorized,
            Concept2Error::Forbidden,
            Concept2Error::RateLimited {
                retry_after_secs: Some(12),
            },
            Concept2Error::Http { status: 500 },
            Concept2Error::NotFound(9001),
            Concept2Error::InvalidUrl,
            Concept2Error::InsecureConnection,
            Concept2Error::InsecureRedirectBlocked,
            Concept2Error::Timeout,
            Concept2Error::BodyTooLarge { limit: 1024 },
            Concept2Error::Transport("I/O error".to_owned()),
            Concept2Error::Decode("malformed payload".to_owned()),
        ] {
            let text = format!("{error} {error:?}");
            assert!(!text.contains(TOKEN), "{text}");
        }

        let client = test_client("http://127.0.0.1:1");
        let debug = format!("{client:?}");
        assert!(!debug.contains(TOKEN), "{debug}");
        assert!(debug.contains("REDACTED"), "{debug}");

        // A real failure raised while the client is holding the token.
        let transport = client.list_results(1, 10).unwrap_err();
        assert_eq!(transport, Concept2Error::Transport("I/O error".to_owned()));
        assert!(!format!("{transport} {transport:?}").contains(TOKEN));
    }
}
