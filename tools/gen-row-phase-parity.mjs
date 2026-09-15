#!/usr/bin/env node

// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Generate the rower rig-phase parity fixture from the pinned web commit.
 *
 * The equipment corpus (replay-current-main-equipment.json) covers the web's
 * static rig contracts but nothing about the STROKE-PHASE calibration — which
 * channel drives the seat, the oar sweep and the oar roll, and in which
 * direction. That hole let rowplay-qt port Studio's rower calibration
 * op-for-op while Studio's phase was inverted against the web (web
 * renderer3dRowAvatar.ts: "At the catch the seat is closest to the feet";
 * Studio drove it farthest). This fixture pins the web's phase mapping:
 *
 *   rower.position.z    = SEAT_CATCH_Z  + pelvisTravel  · SEAT_TRAVEL
 *   oar.rotation.y      = side · (OAR_YAW_CATCH + handleTravel · OAR_YAW_SPAN)
 *   oar.rotation.z      = −side · (bladeWater · BLADE_DIP + handleTravel · HANDLE_RISE_ROLL)
 *
 * The channel values come from evaluating the web's motionGraph.ts at the
 * same deterministic poses as export_rowplay_native_parity.mjs (Studio's
 * exporter, cribbed here); the calibration constants are extracted from the
 * avatar source at the pinned commit and recorded with the file's SHA-256,
 * so a change to either surfaces as a fixture diff.
 *
 * Usage:
 *   node tools/gen-row-phase-parity.mjs \
 *     --rowplay-repo reference/rowplay \
 *     --commit 4d96480e7c6fb382f800555bd3aa463d9fe5b1a6
 *
 * Writes tests/fixtures/replay-row-phase-parity.json. Requires Node ≥ 23.6
 * (type stripping) and the RowPlay checkout's node_modules (motionGraph.ts
 * pulls in three.js).
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const PINNED_COMMIT = "4d96480e7c6fb382f800555bd3aa463d9fe5b1a6";
const SAMPLE_COUNT = 33;
const AVATAR_PATH = "src/lib/replay/renderer3dRowAvatar.ts";
const MOTION_PATH = "src/lib/replay/motionGraph.ts";

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

/** Deterministic pose scheme shared with export_rowplay_native_parity.mjs. */
function poseFor(phaseIndex) {
  const cycle = phaseIndex / SAMPLE_COUNT;
  const phase = cycle * Math.PI * 2;
  const strokeSeconds = 60 / 28;
  const driveFrac = 0.38;
  const intensity = ((phaseIndex * 37) % SAMPLE_COUNT) / (SAMPLE_COUNT - 1);
  return {
    index: 7,
    phase,
    warpedPhase: phase,
    cycleFrac: cycle,
    driveFrac,
    drive: cycle < driveFrac,
    driveProgress: cycle < driveFrac ? cycle / driveFrac : 1,
    recoveryProgress: cycle < driveFrac ? 0 : (cycle - driveFrac) / (1 - driveFrac),
    strokeSeconds,
    strokeMeters: 11,
    rate: 60 / strokeSeconds,
    watts: 200,
    intensity,
    amplitude: 1,
    fatigue: 0,
    real: true,
  };
}

/** Extract `const NAME = <number>;` from the avatar source at the pin. */
function extractConstant(source, name) {
  const match = source.match(new RegExp(`const ${name} = (-?[0-9.]+);`));
  if (!match) throw new Error(`Constant ${name} not found in ${AVATAR_PATH}`);
  return Number(match[1]);
}

const repository = resolve(argument("--rowplay-repo", "reference/rowplay"));
const commit = argument("--commit", PINNED_COMMIT);

git(repository, ["cat-file", "-e", `${commit}^{commit}`]);
const avatarSource = git(repository, ["show", `${commit}:${AVATAR_PATH}`]);
const motionSource = git(repository, ["show", `${commit}:${MOTION_PATH}`]);

const CONSTANTS = [
  "SEAT_TRAVEL",
  "SEAT_CATCH_Z",
  "OAR_YAW_CATCH",
  "OAR_DRAW_YAW",
  "BLADE_DIP",
  "HANDLE_RISE_ROLL",
];
const constants = {};
for (const name of CONSTANTS) {
  constants[name] = extractConstant(avatarSource, name);
}

const nodeModules = join(repository, "node_modules");
if (!existsSync(join(nodeModules, "three", "package.json"))) {
  throw new Error(
    `three.js not found under ${nodeModules}; run npm/pnpm install in the RowPlay checkout first`,
  );
}

const temporaryDirectory = mkdtempSync(join(tmpdir(), "rowplay-row-phase-"));
try {
  symlinkSync(nodeModules, join(temporaryDirectory, "node_modules"), "dir");
  writeFileSync(join(temporaryDirectory, "motionGraph.ts"), withResolvedImports(motionSource), "utf8");
  const load = async (name) =>
    import(`${pathToFileURL(join(temporaryDirectory, name)).href}?commit=${commit}`);
  const motion = await load("motionGraph.ts");

  const samples = [];
  for (let phaseIndex = 0; phaseIndex < SAMPLE_COUNT; phaseIndex += 1) {
    const pose = poseFor(phaseIndex);
    const graph = motion.sampleRowerMotionGraph(pose);
    const pelvisTravel = graph.body.pelvisTravel.value;
    const handleTravel = graph.body.handleTravel.value;
    const bladeWater = graph.contacts.bladeWater.value;
    samples.push({
      phaseIndex,
      pose,
      pelvisTravel,
      handleTravel,
      bladeWater,
      seatZ: constants.SEAT_CATCH_Z + pelvisTravel * constants.SEAT_TRAVEL,
      oarYaw: constants.OAR_YAW_CATCH + handleTravel * (constants.OAR_DRAW_YAW - constants.OAR_YAW_CATCH),
      oarRollZ: -(bladeWater * constants.BLADE_DIP + handleTravel * constants.HANDLE_RISE_ROLL),
    });
  }

  const fixture = {
    schema: "rowplay.replay.row-phase-parity.v1",
    sourceCommit: commit,
    generatorVersion: "gen-row-phase-parity/1.0.0",
    sourceFileSha256s: {
      [AVATAR_PATH]: sha256(avatarSource),
      [MOTION_PATH]: sha256(motionSource),
    },
    sampleCount: SAMPLE_COUNT,
    constants,
    mapping: {
      seatZ: "SEAT_CATCH_Z + pelvisTravel * SEAT_TRAVEL (rower group z; catch closest to the feet)",
      oarYaw: "OAR_YAW_CATCH + handleTravel * (OAR_DRAW_YAW - OAR_YAW_CATCH) (unsigned; apply per side)",
      oarRollZ: "-(bladeWater * BLADE_DIP + handleTravel * HANDLE_RISE_ROLL) (unsigned; apply per side)",
    },
    samples,
  };

  const target = resolve(argument("--out", "tests/fixtures/replay-row-phase-parity.json"));
  writeFileSync(target, `${JSON.stringify(fixture, null, 2)}\n`, "utf8");
  console.log(`wrote ${target}: ${SAMPLE_COUNT} samples from ${commit}`);
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
