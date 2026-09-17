#!/usr/bin/env node

// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Generate the stroke-model web-pipeline parity fixture from the pinned web
 * commit (docs/parity-coverage.md, ranking 5).
 *
 * `stroke_model::{build_stroke_timeline, stroke_pose_at}` is the production
 * pose path (the backend feeds it to the renderers), but until now its only
 * fixture was `stroke-pose-parity.json`, which predates the web's 2026-07
 * rework (#171) and drives the Studio-semantics `compute_at_time` with three
 * range cases. The web pipeline's own inputs — varied rate, watts, hr,
 * duration, real vs synthetic timelines, degenerate rows — were pinned only
 * by unit tests written from the port.
 *
 * This generator imports the web's real `strokeModel.ts` (via `git show` at
 * the pinned commit, under Node's type stripping; its only runtime import is
 * `motion.ts` for `warpStrokePhase`), feeds it each case's stroke rows
 * verbatim, and records:
 *
 *   - the full `StrokeTimeline` it builds (entries + aggregates),
 *   - the `StrokePose` it returns at a sweep of query times per case
 *     (row starts, row midpoints, row ends — the `entryAt` boundary — plus
 *     before/after the timeline),
 *   - `fallbackStrokePose` samples over negative/large phases and rates.
 *
 * The Rust test feeds the recorded inputs to the port and compares the
 * recorded outputs field-by-field, except `warpedPhase`: the port's warp is
 * deliberately C1 where the web's is C0 (source-map divergence), so the test
 * pins only the band structure (same cycle, same drive/recovery half) there.
 *
 * Usage:
 *   node tools/gen-stroke-model-parity.mjs \
 *     --rowplay-repo reference/rowplay \
 *     --commit 011e8303b66b4d2265a6f1ec8b3ed9d8ed497086
 *
 * Writes tests/fixtures/replay-stroke-model-parity.json. Requires Node ≥ 23.6
 * (type stripping); no node_modules needed (motion.ts is dependency-free).
 */

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const PINNED_COMMIT = "011e8303b66b4d2265a6f1ec8b3ed9d8ed497086";
const STROKE_MODEL_PATH = "src/lib/replay/strokeModel.ts";
const MOTION_PATH = "src/lib/replay/motion.ts";

function argument(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index === -1) return fallback;
  const value = process.argv[index + 1];
  if (!value || value.startsWith("--")) throw new Error(`Missing value for ${name}`);
  return value;
}

function git(repository, args) {
  return execFileSync("git", ["-C", repository, ...args], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

function sha256(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

/** Node's type stripping erases types but performs no module resolution. */
function withResolvedImports(source) {
  return source.replace(
    /(from\s+")(\.{1,2}\/[^"]+)(")/g,
    (match, prefix, specifier, suffix) =>
      /\.(ts|js|mjs|cjs|json)$/.test(specifier) ? match : `${prefix}${specifier}.ts${suffix}`,
  );
}

/**
 * Serialise `value` as JSON with every finite number at 17 significant digits
 * (`toPrecision(17)`, the IEEE-754 double round-trip length). V8's default
 * shortest-round-trip rendering has shifted across releases, which would make
 * the manifest's byte/sha pin unstable across Node versions; fixed precision
 * is byte-stable and still round-trips exactly. Same convention as
 * gen-rig-phase-parity.mjs.
 */
function stableStringify(value, { indent = 1 } = {}) {
  const PLACEHOLDER = "__STABLE_NUMBER__";
  const numbers = [];
  const replacer = (_key, v) => {
    if (typeof v === "number") {
      if (!Number.isFinite(v)) throw new Error(`non-finite number ${v}`);
      const literal =
        Number.isInteger(v) && Object.is(v, Math.trunc(v)) && Math.abs(v) < 1e17
          ? v.toString()
          : v.toPrecision(17);
      numbers.push(literal);
      return PLACEHOLDER;
    }
    return v;
  };
  const rendered = JSON.stringify(value, replacer, indent);
  let cursor = 0;
  return rendered.replace(new RegExp(`"${PLACEHOLDER}"`, "g"), () => numbers[cursor++]);
}

/** One raw stroke row; `hr` is omitted (not null) when the workout has none. */
function row(t, d, pace, spm, watts, hr) {
  const stroke = { t, d, pace, spm, watts };
  if (hr !== undefined) stroke.hr = hr;
  return stroke;
}

/**
 * The query-time sweep for one built timeline: before the start, the start,
 * every row's start / midpoint / end (the end is `entryAt`'s binary-search
 * boundary — `t < endT` keeps the row, `t == endT` advances), the timeline
 * duration and past it.
 */
function queryTimes(timeline) {
  const times = new Set([-1, 0]);
  for (const entry of timeline.entries) {
    times.add(entry.startT);
    times.add(entry.startT + (entry.endT - entry.startT) / 2);
    times.add(entry.endT);
  }
  times.add(timeline.duration);
  times.add(timeline.duration + 5);
  return [...times].sort((a, b) => a - b);
}

function assertFinitePose(pose, where) {
  for (const [key, value] of Object.entries(pose)) {
    if (typeof value === "number") {
      assert.ok(Number.isFinite(value), `${where}: pose.${key} is ${value}`);
    }
  }
}

// ---------------------------------------------------------------------------
// Cases. Each is fed to the web module verbatim; the fixture records exactly
// these inputs plus whatever the web returns for them.
// ---------------------------------------------------------------------------

function steadyRowerRows() {
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 8; i += 1) {
    t += 60 / 28;
    d += 11.2;
    rows.push(row(t, d, 120, 28, 190 + (i % 3) * 12, 140 + i * 4));
  }
  return rows;
}

function sprintRowerRows() {
  // Rate climbs 32 → 38 spm (rateNorm saturates near the 36 ceiling), watts
  // peak mid-piece, no heart-rate strap: hrFatigue stays 0. The last row's
  // spm 42 exceeds the elite ceiling and pins the intensity clamp at 1.
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 7; i += 1) {
    const spm = i === 6 ? 42 : 32 + i;
    t += 60 / spm;
    d += 12.5 - i * 0.4;
    rows.push(row(t, d, 104 + i, spm, Math.min(500, 320 + i * 30)));
  }
  return rows;
}

function skiergIntervalRows() {
  // Work/rest intervals: every third row is a rest stroke (0 watts, d does
  // not advance, pace 0 — so the zero-distance second pass cannot rescue it
  // and strokeMeters stays 0), one row repeats the running (t, d) exactly
  // (a non-advancing anchor the timeline must skip), and one rest row carries
  // hr 0 (falsy-but-present: the web's `hr ?? 0` and `h > 0` treat it as
  // missing, and the port's Option path must agree).
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 9; i += 1) {
    const rest = i % 3 === 2;
    const spm = i % 2 === 0 ? 24 : 36;
    t += 60 / spm;
    if (!rest) d += 8.4;
    const hr = rest ? (i === 5 ? 0 : undefined) : 152 + i * 3;
    rows.push(row(t, d, rest ? 0 : 138, spm, rest ? 0 : 210 + (i % 4) * 18, hr));
    if (i === 5) {
      // Interval boundary: Concept2 repeats the cumulative coordinate.
      rows.push(row(t, d, 0, spm, 0, undefined));
    }
  }
  return rows;
}

function bikeRows() {
  // BikeErg: rpm 70 → 110 (rate ceiling 120), distance-per-stroke ~5-7 m,
  // hr present. driveFraction short-circuits to 0.5 for the bike, so the
  // sweep pins that branch plus the rpm-based secondsFromRate clamp range.
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 8; i += 1) {
    const spm = 70 + i * 6;
    t += 60 / spm;
    d += 5.2 + (i % 3) * 0.7;
    rows.push(row(t, d, 98 + (i % 4) * 3, spm, 240 + i * 26, 130 + i * 5));
  }
  return rows;
}

function degenerateRowerRows() {
  // Everything the normalisers have to survive: spm 0 (JS `spm || base`
  // falls back to 28 wherever secondsFromRate runs), spm 65 (rateNorm
  // divides the raw value past the 36 ceiling, so the intensity clamp
  // saturates), pace 0 with d never advancing (metersFromPace cannot rescue
  // the zero-distance second pass; medianDps falls back to 11, wattsNorm to
  // the 0.35 constant), watts 0 everywhere and no hr at all. The third row
  // repeats the second's (t, d) exactly — a non-advancing anchor the
  // timeline must skip rather than fabricate a cycle for.
  return [
    row(60 / 28, 0, 0, 0, 0),
    row(60 / 28 + 1.5, 0, 0, 65, 0),
    row(60 / 28 + 1.5, 0, 0, 24, 0),
    row(60 / 28 + 1.5 + 2.5, 0, 0, 30, 0),
  ];
}

function singleRowerRow() {
  // One row: duration = endT, so progress = 1 at every query inside it and
  // the fatigue blend runs at its ceiling; median == the row's own watts.
  return [row(2.4, 12.1, 118, 25, 260, 175)];
}

function syntheticRowerRows() {
  // real=false integrates cycle spans from each row's duration against
  // secondsFromRate(spm): the rate ladder 24 → 40 makes later rows consume
  // less than one modelled cycle each, so `index` and `cycleFrac` decouple
  // from the row index — the phase-integration path strokePoseAt takes for
  // timelines without genuine per-stroke cycles.
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 6; i += 1) {
    const spm = 24 + i * 3.2;
    t += 60 / spm + 0.4;
    d += 10.5;
    rows.push(row(t, d, 121, spm, 200 + i * 20, 150 + i * 4));
  }
  return rows;
}

function syntheticBikeRows() {
  const rows = [];
  let t = 0;
  let d = 0;
  for (let i = 0; i < 5; i += 1) {
    const spm = 60 + i * 12;
    t += 60 / spm + 0.2;
    d += 4.8;
    rows.push(row(t, d, 105, spm, 180 + i * 40, 120 + i * 8));
  }
  return rows;
}

const CASES = [
  { name: "rower-steady-state", sport: "rower", real: true, strokes: steadyRowerRows() },
  { name: "rower-sprint-no-hr", sport: "rower", real: true, strokes: sprintRowerRows() },
  { name: "skierg-intervals-rest-anchor", sport: "skierg", real: true, strokes: skiergIntervalRows() },
  { name: "bike-rate-ladder", sport: "bike", real: true, strokes: bikeRows() },
  { name: "rower-degenerate-rows", sport: "rower", real: true, strokes: degenerateRowerRows() },
  { name: "rower-single-row", sport: "rower", real: true, strokes: singleRowerRow() },
  { name: "rower-synthetic-rate-ladder", sport: "rower", real: false, strokes: syntheticRowerRows() },
  { name: "bike-synthetic", sport: "bike", real: false, strokes: syntheticBikeRows() },
  // Empty timeline built with real=true: `real && entries.length > 0` forces
  // the timeline synthetic, and strokePoseAt takes the entry-less
  // syntheticPoseAt branch (default rate, cycle from t alone).
  { name: "skierg-empty-timeline", sport: "skierg", real: true, strokes: [] },
];

const TAU = Math.PI * 2;
const FALLBACKS = [
  { sport: "rower", phase: 0, rate: 0 },
  { sport: "rower", phase: -Math.PI, rate: 10 },
  { sport: "rower", phase: TAU * 2.5, rate: -5 },
  { sport: "rower", phase: TAU * 7.25, rate: 28 },
  { sport: "skierg", phase: TAU * 3.7, rate: 45 },
  { sport: "skierg", phase: TAU / 4, rate: 32 },
  { sport: "bike", phase: TAU / 3, rate: 130 },
  { sport: "bike", phase: TAU * 1.05, rate: 0 },
];

// ---------------------------------------------------------------------------

const repository = resolve(argument("--rowplay-repo", "reference/rowplay"));
const commit = argument("--commit", PINNED_COMMIT);

git(repository, ["cat-file", "-e", `${commit}^{commit}`]);
const head = git(repository, ["rev-parse", "HEAD"]).trim();
assert.equal(
  head,
  commit,
  `${repository} is at ${head}, not the pinned ${commit}; check out the pin before regenerating`,
);
const strokeModelSource = git(repository, ["show", `${commit}:${STROKE_MODEL_PATH}`]);
const motionSource = git(repository, ["show", `${commit}:${MOTION_PATH}`]);

const temporaryDirectory = mkdtempSync(join(tmpdir(), "rowplay-stroke-model-"));
try {
  writeFileSync(join(temporaryDirectory, "motion.ts"), withResolvedImports(motionSource), "utf8");
  writeFileSync(
    join(temporaryDirectory, "strokeModel.ts"),
    withResolvedImports(strokeModelSource),
    "utf8",
  );
  const web = await import(
    `${pathToFileURL(join(temporaryDirectory, "strokeModel.ts")).href}?commit=${commit}`
  );

  const cases = [];
  for (const definition of CASES) {
    const { name, sport, real, strokes } = definition;
    // The recorded inputs must survive the JSON round trip exactly (they are
    // what the port will be fed); a value that does not survive (NaN,
    // undefined-in-array) would silently change meaning.
    assert.deepStrictEqual(strokes, JSON.parse(JSON.stringify(strokes)), `${name}: inputs`);
    const timeline = web.buildStrokeTimeline(strokes, sport, real);
    assert.ok(timeline.entries.length <= strokes.length, `${name}: anchors skipped`);
    const queries = queryTimes(timeline).map((t) => {
      const pose = web.strokePoseAt(timeline, t);
      assertFinitePose(pose, `${name} @ ${t}`);
      return { t, pose };
    });
    cases.push({ name, sport, real, strokes, timeline, queries });
  }

  const fallbackPoses = FALLBACKS.map(({ sport, phase, rate }) => {
    const pose = web.fallbackStrokePose(sport, phase, rate);
    assertFinitePose(pose, `fallback ${sport} ${phase} ${rate}`);
    return { sport, phase, rate, pose };
  });

  const fixture = {
    schema: "rowplay.replay.stroke-model-parity.v1",
    sourceCommit: commit,
    generatorVersion: "gen-stroke-model-parity/1.0.0",
    sourceFileSha256s: {
      [STROKE_MODEL_PATH]: sha256(strokeModelSource),
      [MOTION_PATH]: sha256(motionSource),
    },
    mapping: {
      timeline: "buildStrokeTimeline(strokes, sport, real) — entries + aggregates, recorded verbatim",
      pose: "strokePoseAt(timeline, t) — recorded verbatim at every query time",
      fallback: "fallbackStrokePose(sport, phase, rate) — recorded verbatim",
      warpedPhase:
        "the web's C0 warpStrokePhase; the port is deliberately C1 (source-map divergence), so the Rust test pins only the band structure (same cycle, same drive/recovery half) for this field and compares every other field at 1e-10",
    },
    caseCount: cases.length,
    queryCount: cases.reduce((total, c) => total + c.queries.length, 0) + fallbackPoses.length,
    cases,
    fallbackPoses,
  };

  const serialised = `${stableStringify(fixture)}\n`;
  // The serialiser must not perturb any recorded number: parse it back and
  // demand deep equality with what plain JSON.stringify would have produced
  // (the plain parse is the reference for key presence — both serialisers
  // drop `hr: undefined`, and the numbers are the only thing under test).
  assert.deepStrictEqual(
    JSON.parse(serialised),
    JSON.parse(JSON.stringify(fixture)),
    "stableStringify round trip",
  );

  const target = resolve(argument("--out", "tests/fixtures/replay-stroke-model-parity.json"));
  writeFileSync(target, serialised, "utf8");
  console.log(
    `wrote ${target}: ${fixture.caseCount} cases, ${fixture.queryCount} pose samples from ${commit}`,
  );
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
