#!/usr/bin/env node

// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Sample the pinned web SkiErg avatar's post-IK `arm.hand` over the
 * step-529 window (cyc [0.24, 0.29] at 2000 samples/cycle).
 *
 * This is NOT a re-derived formula: the generator builds the real
 * `makeSkierAvatar`, parents it so `placePoleArms` runs, drives
 * `fallbackStrokePose("skierg", phase, 30)` the same way the port's
 * dense guard does, and records `avatar.v4Targets.leftHand` —
 * which is `arm.hand` (renderer3dSkiAvatar.ts) — via
 * `getWorldPosition()` after `animate()` + `resolveWorldContacts()`.
 *
 * Usage (needs --experimental-transform-types; `three` in
 * reference/rowplay/node_modules):
 *
 *   node --experimental-transform-types tools/gen-ski-arm-hand-window.mjs
 *
 * Writes JSON to stdout. Does not touch tests/fixtures/.
 */
import { register } from "node:module";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

register("./gen-rig-phase-resolve.mjs", import.meta.url);

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const REPLAY = join(REFERENCE, "src", "lib", "replay");

const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";
const GENERATOR_VERSION = "gen-ski-arm-hand-window/1.0.0";

const SOURCE_FILES = [
  "renderer3dSkiAvatar.ts",
  "renderer3dAvatarKit.ts",
  "figurePose.ts",
  "strokeModel.ts",
  "sportKinematics.ts",
  "motionGraph.ts",
  "handGrip.ts",
  "motion.ts",
];

const AVATAR_ARGS = {
  accent: 0x3b82f6,
  castShadow: false,
  opacity: 1,
  bodySegments: 16,
  quality: "high",
};

const SAMPLES = 2000;
const STEP_LO = 480;
const STEP_HI = 580;
const WARMUP_ITERATIONS = 10;

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

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

const head = execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();
if (head !== PINNED_COMMIT) {
  console.error(
    `reference/rowplay is at ${head}, expected the pinned ${PINNED_COMMIT}.`,
  );
  process.exit(1);
}

const sourceFileSha256s = {};
for (const file of SOURCE_FILES) {
  sourceFileSha256s[file] = sha256(await readFile(join(REPLAY, file)));
}

const url = (name) => pathToFileURL(join(REPLAY, name)).href;
const { makeSkierAvatar } = await import(url("renderer3dSkiAvatar.ts"));
const { fallbackStrokePose } = await import(url("strokeModel.ts"));
const THREE = await import("three");

const avatar = makeSkierAvatar(
  AVATAR_ARGS.accent,
  AVATAR_ARGS.castShadow,
  AVATAR_ARGS.opacity,
  AVATAR_ARGS.bodySegments,
  AVATAR_ARGS.quality,
);
const scene = new THREE.Scene();
scene.add(avatar.group);

const handNode = avatar.v4Targets?.leftHand;
if (!handNode) {
  throw new Error("avatar.v4Targets.leftHand missing — that is arm.hand");
}
if (handNode.name !== "skierg-hand-left") {
  throw new Error(`leftHand name is ${handNode.name}, expected skierg-hand-left`);
}

const scratch = new THREE.Vector3();
const samples = [];
let prev = null;
let maxDelta = 0;
let maxDeltaStep = STEP_LO;

for (let step = STEP_LO; step <= STEP_HI; step++) {
  const cycle = step / SAMPLES;
  const pose = fallbackStrokePose("skierg", cycle * Math.PI * 2, 30);
  const meters = step * 3.0;
  for (let w = 0; w < WARMUP_ITERATIONS; w++) {
    avatar.animate(pose.phase, false, pose, meters);
  }
  avatar.resolveWorldContacts?.();
  avatar.group.updateMatrixWorld(true);
  const hp = handNode.getWorldPosition(scratch);
  const hand = [hp.x, hp.y, hp.z];
  const dHand = prev
    ? Math.hypot(hand[0] - prev[0], hand[1] - prev[1], hand[2] - prev[2])
    : 0;
  if (dHand > maxDelta) {
    maxDelta = dHand;
    maxDeltaStep = step;
  }
  samples.push({
    step,
    cyc: cycle,
    cycleFrac: pose.cycleFrac,
    hand,
    dHand,
  });
  prev = hand;
}

const out = {
  schema: "rowplay.replay.ski-arm-hand-window.v1",
  description:
    "Web SkiErg arm.hand (v4Targets.leftHand) world position after animate + resolveWorldContacts, sampled at the port guard's 2000-step resolution over cyc [0.24, 0.29].",
  sourceCommit: head,
  generatorVersion: GENERATOR_VERSION,
  sourceFileSha256s,
  avatarArgs: AVATAR_ARGS,
  samples,
  maxDelta,
  maxDeltaStep,
};

process.stdout.write(stableStringify(out) + "\n");
console.error(
  `max dHand=${maxDelta.toPrecision(17)} at step ${maxDeltaStep} (cyc ${(maxDeltaStep / SAMPLES).toPrecision(17)})`,
);
