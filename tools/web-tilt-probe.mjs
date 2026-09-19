// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Web wrist-refinement wrap oracle (Phase 7.5).
 *
 * Drives the pinned web SkiErg avatar exactly the way the port's dense guard
 * (`requested_twist_stays_continuous_and_engages_the_budgets`) drives its own
 * solve — `fallbackStrokePose("skierg", phase, 30)` at 2000 samples/cycle,
 * `meters = step · 3.0` — then re-derives, from the RENDERED nodes, the two
 * atan2 refinement angles inside `handGrip.ts`:
 *
 *   tilt = atan2((long × forearm)·palmNormal, long·forearm)  (off the palm normal)
 *   spin = atan2((long × forearm)·shaft,       long·forearm)  (off the pole shaft)
 *
 * and the rendered hand's per-step position/orientation deltas. Neither
 * refinement wrap-guards its angle, so where either angle passes through ±π
 * the applied correction snaps sign. The web itself snaps:
 *
 *   tilt wrap: cyc 0.2635 — qjump 1.299 rad, handjump 0.011 m
 *   spin wrap: cyc 0.7080 — qjump 1.096 rad, handjump 0.002 m
 *
 * (Node harness = the procedural path: `arm.hand.position` is set directly
 * from the pole solve, so its wraps stay orientation-only. The port, like the
 * web's V4 hero chain, solves the wrist as `target − R_hand·offset`, so the
 * same wrap couples into position — an architecture difference, not a law
 * difference; see the guard test's carve-out comment and docs/source-map.md.)
 *
 * Usage (needs --experimental-transform-types for the TS parameter
 * properties in renderer3dV4Motion.ts, which value imports in the graph
 * reach through renderer3d.ts):
 *
 *   node --experimental-transform-types tools/web-tilt-probe.mjs | \
 *     grep wrap_summary
 */
import { register } from "node:module";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

register("./gen-rig-phase-resolve.mjs", import.meta.url);

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const REPLAY = join(REFERENCE, "src", "lib", "replay");

const url = (name) => pathToFileURL(join(REPLAY, name)).href;

// The probe characterises the pinned web; its numbers are recorded in the
// wrap-window docs, so refuse to run against any other checkout (same guard
// idiom as tools/gen-rig-phase-parity.mjs).
const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";
const head = execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();
if (head !== PINNED_COMMIT) {
  console.error(
    `reference/rowplay is at ${head}, expected the pinned ${PINNED_COMMIT}.\n` +
      `Check out the pinned commit (docs/source-map.md) or update the pin in this script.`,
  );
  process.exit(1);
}

const { makeSkierAvatar } = await import(url("renderer3dSkiAvatar.ts"));
const { handLongAxis, handPalmNormalOut } = await import(url("handGrip.ts"));
const { fallbackStrokePose } = await import(url("strokeModel.ts"));
const THREE = await import("three");

// Same avatar construction as tools/gen-rig-phase-parity.mjs.
const AVATAR_ARGS = {
  accent: 0x3b82f6,
  castShadow: false,
  opacity: 1,
  bodySegments: 16,
  quality: "high",
};
const avatar = makeSkierAvatar(
  AVATAR_ARGS.accent,
  AVATAR_ARGS.castShadow,
  AVATAR_ARGS.opacity,
  AVATAR_ARGS.bodySegments,
  AVATAR_ARGS.quality,
);
// Parent the avatar so `placePoleArms` (which requires `group.parent`) runs.
const scene = new THREE.Scene();
scene.add(avatar.group);

const SAMPLES = 2000; // the guard test's density

// Scratch objects, allocated once (the web's own per-frame discipline).
const LONG = new THREE.Vector3();
const PALM = new THREE.Vector3();
const FORE = new THREE.Vector3();
const SHAFT = new THREE.Vector3();
const SPIN_LONG = new THREE.Vector3();
const SPIN_FORE = new THREE.Vector3();
const CROSS = new THREE.Vector3();
const Q = new THREE.Quaternion();
const V1 = new THREE.Vector3();
const V2 = new THREE.Vector3();
const V3 = new THREE.Vector3();

let lastHp = null;
let lastQ = null;
let lastSpin = null;
let lastTilt = null;
const wraps = [];

const findNode = (root, pattern, side) => {
  let found = null;
  root.traverse((o) => {
    if (found) return;
    if (
      pattern.test(o.name) &&
      (side < 0 ? /left/i.test(o.name) : /right/i.test(o.name))
    ) {
      found = o;
    }
  });
  return found;
};

for (let i = 0; i < SAMPLES; i++) {
  const cycle = i / SAMPLES;
  // Guard-test drive: fallbackStrokePose(skierg, phase, 30), meters = step·3.
  const pose = fallbackStrokePose("skierg", cycle * Math.PI * 2, 30);
  const meters = i * 3.0;
  // Ten warm-up calls converge the damped kinematics (same as the generator).
  for (let w = 0; w < 10; w++) avatar.animate(pose.phase, false, pose, meters);
  avatar.resolveWorldContacts?.();

  // The rendered left hand, elbow and pole basket (node names from
  // renderer3dSkiAvatar.ts: skiHandLeft / skiElbowLeft / skierg-pole-tip-left).
  const hand = findNode(avatar.group, /hand/i, -1);
  const elbow = findNode(avatar.group, /elbow/i, -1);
  const tip = findNode(avatar.group, /pole-tip/i, -1);
  if (!hand || !elbow || !tip) throw new Error("hand/elbow/pole-tip node not found");

  hand.getWorldQuaternion(Q);
  handLongAxis(-1, LONG).applyQuaternion(Q);
  handPalmNormalOut(-1, PALM).applyQuaternion(Q).normalize();
  const hp = hand.getWorldPosition(V1);
  const ep = elbow.getWorldPosition(V2);
  FORE.copy(hp).sub(ep).normalize();
  // Thumbward shaft: from the basket toward the hand.
  SHAFT.copy(hp).sub(tip.getWorldPosition(V3)).normalize();

  // Spin angle off the shaft, on unprojected long/forearm copies.
  let spin = null;
  SPIN_LONG.copy(LONG).addScaledVector(SHAFT, -LONG.dot(SHAFT));
  SPIN_FORE.copy(FORE).addScaledVector(SHAFT, -FORE.dot(SHAFT));
  if (SPIN_LONG.lengthSq() > 1e-8 && SPIN_FORE.lengthSq() > 1e-6) {
    SPIN_LONG.normalize();
    SPIN_FORE.normalize();
    spin = Math.atan2(
      CROSS.crossVectors(SPIN_LONG, SPIN_FORE).dot(SHAFT),
      SPIN_LONG.dot(SPIN_FORE),
    );
  }

  // Tilt angle off the palm normal.
  LONG.addScaledVector(PALM, -LONG.dot(PALM));
  FORE.addScaledVector(PALM, -FORE.dot(PALM));
  let tilt = null;
  if (LONG.lengthSq() > 1e-8 && FORE.lengthSq() > 1e-6) {
    LONG.normalize();
    FORE.normalize();
    tilt = Math.atan2(CROSS.crossVectors(LONG, FORE).dot(PALM), LONG.dot(FORE));
  }

  const q = [Q.x, Q.y, Q.z, Q.w];
  if (lastHp) {
    const dpos = hp.distanceTo(lastHp);
    const d = q[0] * lastQ[0] + q[1] * lastQ[1] + q[2] * lastQ[2] + q[3] * lastQ[3];
    const dang = 2 * Math.acos(Math.min(1, Math.abs(d)));
    if (lastSpin !== null && spin !== null && Math.abs(spin - lastSpin) > 2.0) {
      wraps.push({ cycle, kind: "spin", from: lastSpin, to: spin, dpos, dang });
    }
    if (tilt !== null && lastTilt !== null && Math.abs(tilt - lastTilt) > 2.0) {
      wraps.push({ cycle, kind: "tilt", from: lastTilt, to: tilt, dpos, dang });
    }
    console.log(
      `cycle ${cycle.toFixed(4)} tilt=${tilt === null ? "n/a" : tilt.toFixed(5)} ` +
        `spin=${spin === null ? "n/a" : spin.toFixed(5)} ` +
        `handjump=${dpos.toFixed(5)} qjump=${dang.toFixed(5)}`,
    );
  }
  lastHp = hp.clone();
  lastQ = q;
  lastSpin = spin;
  lastTilt = tilt;
}

console.error("--- wrap summary ---");
for (const w of wraps) {
  console.error(
    `wrap_summary kind=${w.kind} cyc=${w.cycle.toFixed(4)} ` +
      `${w.from.toFixed(4)} -> ${w.to.toFixed(4)} handjump=${w.dpos.toFixed(4)} qjump=${w.dang.toFixed(4)}`,
  );
}
if (wraps.length === 0) console.error("wrap_summary none");
