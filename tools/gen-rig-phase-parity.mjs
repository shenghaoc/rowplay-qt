// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Generate the rig-phase parity fixture from the rowplay web avatars.
 *
 * One command, from the repository root (needs `pnpm install` in
 * reference/rowplay first, Node >= 23.6):
 *
 *   node --experimental-transform-types tools/gen-rig-phase-parity.mjs
 *
 * Stage 2 of the parity coverage audit (docs/parity-coverage.md, ranking
 * rows 1-3): `crates/rowplay-core/src/replay/rig_pose.rs` ports Studio's
 * `ReplayRigPose` calibration, which was never compared against the web —
 * the web's calibration lives inside the per-sport avatar factories
 * (`renderer3d{Row,Ski,Bike}Avatar.ts`), not in the rig modules the
 * source-map row cited. This generator builds the three web avatars
 * headless (three.js scene-graph maths need no renderer), drives them over
 * the full stroke cycle at two timing inputs per sport, and records the
 * resulting rig-local transforms and contact landmarks, so the Rust solver
 * can be compared against the web numbers instead of Studio's.
 *
 * Following `export_rowplay_native_parity.mjs` (the fixture pattern this
 * audit standardises on): the reference checkout's HEAD must equal the
 * pinned commit (we import the checkout, so this is stricter than the
 * export script's git-show), and every evaluated source file is recorded
 * by SHA-256 so a reference change surfaces as a fixture diff.
 *
 * Deterministic: no randomness in the avatar factories; every input pose is
 * constructed explicitly and echoed into the fixture.
 */
import { register } from "node:module";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

register("./gen-rig-phase-resolve.mjs", import.meta.url);

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const REPLAY = join(REFERENCE, "src", "lib", "replay");
const OUT_DEFAULT = join(REPO, "tests", "fixtures", "replay-rig-phase-parity.json");

/** The reference commit this generator is pinned to (docs/source-map.md). */
const PINNED_COMMIT = "011e8303b66b4d2265a6f1ec8b3ed9d8ed497086";
const GENERATOR_VERSION = "gen-rig-phase-parity/1.0.0";

/** The web sources whose evaluation this fixture captures. */
const SOURCE_FILES = [
  "renderer3dRowAvatar.ts",
  "renderer3dSkiAvatar.ts",
  "renderer3dBikeAvatar.ts",
  "renderer3dAvatarKit.ts",
  "renderer3dVenueKit.ts",
  "renderer3dAssets.ts",
  "strokeModel.ts",
  "motionGraph.ts",
  "motion.ts",
  "rowRig.ts",
  "skiEquipment.ts",
  "bikeRig.js",
  "bikeSaddle.js",
  "figurePose.ts",
  "handGrip.ts",
  "renderer.ts",
];

/** Avatar factory arguments, echoed for reproducibility. */
const AVATAR_ARGS = { accent: 0x3b82f6, castShadow: false, opacity: 1, bodySegments: 16, quality: "high" };

const SAMPLES_PER_SWEEP = 64;

/**
 * Sweeps per sport. `base` matches the motion corpus's pose scheme (fixed
 * timing, `warpedPhase = phase`); `timing` varies the drive fraction the way
 * a different stroke rate would, so the calibration is pinned at two points
 * of the timing input space (the corpus never varies it). The bike's rig
 * additionally consumes distance, so both sweeps advance it.
 */
const SWEEPS = {
  rower: [
    { id: "base", strokeSeconds: 60 / 28, driveFrac: 0.38, strokeMeters: 11.0 },
    { id: "timing", strokeSeconds: 60 / 22, driveFrac: 0.44, strokeMeters: 11.0 },
  ],
  skierg: [
    { id: "base", strokeSeconds: 1.875, driveFrac: 0.34, strokeMeters: 8.0 },
    { id: "timing", strokeSeconds: 60 / 36, driveFrac: 0.30, strokeMeters: 8.0 },
  ],
  bike: [
    { id: "base", strokeSeconds: 0.75, driveFrac: 0.5, strokeMeters: 5.0 },
    { id: "timing", strokeSeconds: 0.6, driveFrac: 0.5, strokeMeters: 5.0 },
  ],
};

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/**
 * Serialise `value` as JSON, formatting every finite number with 17
 * significant digits (via `Number.prototype.toPrecision(17)`, which is the
 * IEEE-754 double round-trip length). V8's default `JSON.stringify` uses
 * the shortest string that round-trips, and that "shortest" length has
 * shifted across V8 releases — so the same double serialises as
 * `0.1249269234996172` on one Node and `0.12492692349961732` on another,
 * differing by ULPs of text without differing in the double. Every parity
 * test tolerates >= 1e-6 so the drift is functionally invisible, but a CI
 * regeneration check compares the bytes and would fail. Fixed-precision
 * output makes the fixture byte-stable across Node/V8 versions (and stays
 * exactly round-trippable through the parsers on both sides).
 *
 * NaN and Infinity aren't valid JSON so they still throw (caller should
 * scrub or record them as strings before serialising if that ever comes
 * up).
 */
function stableStringify(value, { indent = 1 } = {}) {
  const PLACEHOLDER = "__STABLE_NUMBER__";
  const numbers = [];
  const replacer = (_key, v) => {
    if (typeof v === "number") {
      if (!Number.isFinite(v)) throw new Error(`non-finite number ${v}`);
      // Integers get exact decimal form (`toPrecision(17)` would print `0`
      // as "0.0000000000000000", which is stable but ugly for indices).
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

function gitHead() {
  return execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
}

/** The pose scheme: identical in shape to the export script's `poseFor`. */
function poseFor(sport, sweep, phaseIndex) {
  const cycle = phaseIndex / SAMPLES_PER_SWEEP;
  const phase = cycle * Math.PI * 2;
  const intensity = ((phaseIndex * 37) % SAMPLES_PER_SWEEP) / (SAMPLES_PER_SWEEP - 1);
  const drive = cycle < sweep.driveFrac;
  return {
    index: 7,
    phase,
    warpedPhase: phase,
    cycleFrac: cycle,
    driveFrac: sweep.driveFrac,
    drive,
    driveProgress: drive ? cycle / sweep.driveFrac : 1,
    recoveryProgress: drive ? 0 : (cycle - sweep.driveFrac) / (1 - sweep.driveFrac),
    strokeSeconds: sweep.strokeSeconds,
    strokeMeters: sweep.strokeMeters,
    rate: 60 / sweep.strokeSeconds,
    watts: 200,
    intensity,
    amplitude: 1,
    fatigue: 0,
    real: true,
  };
}

function worldPosition(object, scratch) {
  const v = object.getWorldPosition(scratch);
  return [v.x, v.y, v.z];
}

function euler(object) {
  return [object.rotation.x, object.rotation.y, object.rotation.z];
}

function named(root, name) {
  let found = null;
  root.traverse((o) => {
    if (o.name === name) found = o;
  });
  if (!found) throw new Error(`no node named ${name}`);
  return found;
}

/**
 * The RowErg oar solve's inputs and output, read from the animated scene.
 *
 * `solveRowerOarYaw` is a pure function of (shoulder→pin delta, signed inboard
 * lever, blade roll, requested reach, preferred yaw); the rendered yaw is its
 * return. Recording the real arguments lets a Rust test feed them back
 * verbatim and compare against what the web actually rendered — the
 * "record what the web renders" rule (AGENTS.md) applied where the quantity
 * is only observable as a function call. The generator then re-solves from
 * its own recording and asserts it reproduces the rendered yaw, so wrong
 * observables cannot be recorded silently.
 */
function sample(avatar, sport, pose, meters, scratch) {
  avatar.group.updateMatrixWorld(true);
  const v4 = avatar.v4Targets;
  const t = avatar.group;
  const out = {
    meters,
    cues: avatar.lastCues,
    targets: {
      pelvis: worldPosition(v4.pelvis, scratch),
      leftHand: worldPosition(v4.leftHand, scratch),
      rightHand: worldPosition(v4.rightHand, scratch),
      leftElbow: worldPosition(v4.leftElbow, scratch),
      rightElbow: worldPosition(v4.rightElbow, scratch),
      leftFoot: worldPosition(v4.leftFoot, scratch),
      rightFoot: worldPosition(v4.rightFoot, scratch),
      leftKnee: worldPosition(v4.leftKnee, scratch),
      rightKnee: worldPosition(v4.rightKnee, scratch),
    },
  };
  if (sport === "rower") {
    const athlete = named(t, "rower-athlete");
    out.seat = [athlete.position.x, athlete.position.y, athlete.position.z];
    out.torsoRotation = euler(named(t, "rower-torso"));
    out.hipsRotation = euler(named(athlete, "rower-hips"));
    out.handleContactLeft = worldPosition(named(t, "rower-hand-contact-left"), scratch);
    out.handleContactRight = worldPosition(named(t, "rower-hand-contact-right"), scratch);
    out.oarLeftRotation = euler(named(t, "rower-oar-left"));
    out.oarRightRotation = euler(named(t, "rower-oar-right"));
    out.bladeLeft = worldPosition(named(t, "rower-blade-left"), scratch);
    out.bladeRight = worldPosition(named(t, "rower-blade-right"), scratch);
  } else if (sport === "skierg") {
    const upper = named(t, "skierg-upper");
    out.upper = [upper.position.x, upper.position.y, upper.position.z];
    out.upperRotation = euler(upper);
    out.headRotation = euler(named(upper, "athlete:head"));
    out.handLeft = worldPosition(named(t, "skierg-hand-left"), scratch);
    out.handRight = worldPosition(named(t, "skierg-hand-right"), scratch);
    out.poleShaftLeftRotation = euler(named(t, "skierg-pole-shaft-left"));
    out.poleShaftRightRotation = euler(named(t, "skierg-pole-shaft-right"));
    out.poleTipLeft = worldPosition(named(t, "skierg-pole-tip-left"), scratch);
    out.poleTipRight = worldPosition(named(t, "skierg-pole-tip-right"), scratch);
  } else {
    const cranks = named(t, "bike-cranks");
    out.crankRotation = euler(cranks);
    out.crankPosition = [cranks.position.x, cranks.position.y, cranks.position.z];
    out.wheelFrontRotation = euler(named(t, "bike-wheel-front"));
    out.wheelRearRotation = euler(named(t, "bike-wheel-rear"));
    out.pedalLeft = worldPosition(named(t, "bike-pedal-left"), scratch);
    out.pedalRight = worldPosition(named(t, "bike-pedal-right"), scratch);
    out.torsoRotation = euler(named(t, "bike-torso"));
    out.pelvisRotation = euler(named(t, "bike-pelvis"));
  }
  return out;
}

function rowerOarSolve(avatar, armDraw, solve) {
  const root = avatar.group;
  const rower = named(root, "rower-athlete");
  const wrap = (value) => Math.atan2(Math.sin(value), Math.cos(value));
  const sides = [];
  for (const side of [-1, 1]) {
    const suffix = side < 0 ? "left" : "right";
    const oar = named(root, `rower-oar-${suffix}`);
    const shoulderNode = named(root, `rower-shoulder-${suffix}`);
    const inboard = named(root, `rower-hand-contact-${suffix}`).position.x;
    const roll = oar.rotation.z;
    // The web passes `oar.group.position - rower.position` as the pin and
    // `arm.shoulderPoint` (the shoulder node's local position, rower frame) as
    // the shoulder; only their difference reaches the solve. Each side uses
    // its own shoulder and pin — mirroring the right side's delta was the
    // error the self-check below caught.
    const pin = [
      oar.position.x - rower.position.x,
      oar.position.y - rower.position.y,
      oar.position.z - rower.position.z,
    ];
    const shoulder = [
      shoulderNode.position.x,
      shoulderNode.position.y,
      shoulderNode.position.z,
    ];
    const pinDelta = [pin[0] - shoulder[0], pin[1] - shoulder[1], pin[2] - shoulder[2]];
    const requestedReach = solve.requestedReach(armDraw);
    const preferredYaw = solve.preferredYaw(side, armDraw);
    const renderedYaw = oar.rotation.y;
    // Re-solve from the recording; a mismatch means an observable above is
    // not what the web passed, so the fixture must not be written.
    const reproduced = solve.fn(
      { x: 0, y: 0, z: 0 },
      pinDelta[0],
      pinDelta[1],
      pinDelta[2],
      inboard,
      roll,
      requestedReach,
      preferredYaw,
      true,
    );
    const delta = Math.abs(wrap(reproduced - renderedYaw));
    if (!(delta < 1e-9)) {
      throw new Error(
        `RowErg oar-solve self-check failed (side ${side}): recording reproduces ${reproduced}, ` +
          `rendered ${renderedYaw} (delta ${delta}) — an observable is wrong; do not write the fixture`,
      );
    }
    sides.push({ side, pinDelta, inboard, roll, requestedReach, preferredYaw, renderedYaw });
  }
  return { armDraw, sides };
}

async function main() {
  const out = process.argv[2] ? resolve(process.argv[2]) : OUT_DEFAULT;
  const head = gitHead();
  if (head !== PINNED_COMMIT) {
    console.error(
      `reference/rowplay is at ${head}, expected the pinned ${PINNED_COMMIT}.\n` +
        `Check out the pinned commit (docs/source-map.md) or update the pin in this script.`,
    );
    return 1;
  }
  const sourceFileSha256s = {};
  for (const file of SOURCE_FILES) {
    const { readFile } = await import("node:fs/promises");
    sourceFileSha256s[file] = sha256(await readFile(join(REPLAY, file)));
  }

  const url = (file) => pathToFileURL(join(REPLAY, file)).href;
  const { makeRowerAvatar } = await import(url("renderer3dRowAvatar.ts"));
  const { makeSkierAvatar } = await import(url("renderer3dSkiAvatar.ts"));
  const { makeBikeAvatar } = await import(url("renderer3dBikeAvatar.ts"));
  const THREE = await import("three");

  // The RowErg oar solve, evaluated from the web sources so the fixture pins
  // the real function's inputs and output (see `rowerOarSolve`).
  const { readFile } = await import("node:fs/promises");
  const readSource = async (file) => readFile(join(REPLAY, file), "utf8");
  const rowAvatarSource = await readSource("renderer3dRowAvatar.ts");
  const rowRigSource = await readSource("rowRig.ts");
  const constant = (source, name) => {
    const match = source.match(new RegExp(`const ${name} = (-?[0-9.]+);`));
    if (!match) throw new Error(`constant ${name} not found`);
    return Number(match[1]);
  };
  const clamp = (value, lo, hi) => Math.min(hi, Math.max(lo, value));
  const functionBody = (source, name) => {
    const start = source.indexOf(`export function ${name}(`);
    if (start === -1) throw new Error(`function ${name} not found`);
    const open = source.indexOf("{", start);
    let depth = 0;
    for (let i = open; i < source.length; i += 1) {
      if (source[i] === "{") depth += 1;
      else if (source[i] === "}") {
        depth -= 1;
        if (depth === 0) return `"use strict";${source.slice(open + 1, i)}\n`;
      }
    }
    throw new Error(`unbalanced braces in ${name}`);
  };
  const solveRowerOarYaw = new Function(
    "THREE",
    "shoulder",
    "pinX",
    "pinY",
    "pinZ",
    "signedInboard",
    "bladeRoll",
    "requestedReach",
    "preferredYaw",
    "forceReachBoundary",
    functionBody(rowRigSource, "solveRowerOarYaw"),
  ).bind(null, { MathUtils: { clamp } });
  const reachForFlexion = new Function(
    "THREE",
    "flexion",
    "upperArmLength",
    "forearmLength",
    functionBody(rowRigSource, "rowerReachForFlexion"),
  ).bind(null, { MathUtils: { clamp } });
  const oarYawCatch = constant(rowAvatarSource, "OAR_YAW_CATCH");
  const oarYawDraw = constant(rowAvatarSource, "OAR_DRAW_YAW");
  const upperArmLength = constant(rowAvatarSource, "UPPER_ARM_LENGTH");
  const forearmLength = constant(rowAvatarSource, "FOREARM_LENGTH");
  const baseArmReach = upperArmLength + forearmLength;
  const drawSoft = constant(rowRigSource, "ROWER_DRAW_SOFT_FLEXION");
  const drawFinish = constant(rowRigSource, "ROWER_DRAW_FINISH_FLEXION");
  const oarSolve = {
    fn: solveRowerOarYaw,
    // The web's `requestedRowerWristReach` at the base arm length (the
    // procedural avatar has no V4 refinement to override it).
    requestedReach: (draw) =>
      reachForFlexion(
        drawSoft + clamp(draw, 0, 1) * (drawFinish - drawSoft),
        baseArmReach * (upperArmLength / baseArmReach),
        baseArmReach - baseArmReach * (upperArmLength / baseArmReach),
      ) - 0.002,
    preferredYaw: (side, draw) => side * (oarYawCatch + clamp(draw, 0, 1) * (oarYawDraw - oarYawCatch)),
  };

  // The graph channel the arm-authority solve is scheduled from.
  const { sampleRowerMotionGraph } = await import(url("motionGraph.ts"));

  const factories = {
    rower: makeRowerAvatar,
    skierg: makeSkierAvatar,
    bike: makeBikeAvatar,
  };
  const samples = [];
  for (const [sport, factory] of Object.entries(factories)) {
    const avatar = factory(AVATAR_ARGS.accent, AVATAR_ARGS.castShadow, AVATAR_ARGS.opacity, AVATAR_ARGS.bodySegments, AVATAR_ARGS.quality);
    // Parent the avatar to a throwaway scene so `renderer3dSkiAvatar.placePoleArms`
    // (and any other `resolveWorldContacts` code that reads `group.parent`
    // before running) actually runs. Detached, `placePoleArms` early-returns on
    // `!group.parent` (renderer3dSkiAvatar.ts:828) and `arm.hand` and the
    // pole tips stay at the pelvis origin for every sample — the previous
    // fixture's `targets.leftHand` / `poleTipLeft` / `handLeft` all
    // collapsed to the pelvis, giving nothing to parity-check the port's
    // `preferred_hand_*` / `plant_basket_z` / `shoulder_*` against. The
    // scene stays at the origin so its world frame equals the avatar's
    // rig-local frame (what the fixture records).
    const scene = new THREE.Scene();
    scene.add(avatar.group);
    const scratch = new THREE.Vector3();
    for (const sweep of SWEEPS[sport]) {
      for (let i = 0; i < SAMPLES_PER_SWEEP; i++) {
        const pose = poseFor(sport, sweep, i);
        const meters = sport === "bike" ? i * 0.35 : 0;
        // Warm up the avatar's damped kinematics against this pose before
        // sampling. `solveSkierKinematics` (and the equivalents for the
        // rower and bike) evolves an internal state — `motion.cycle`,
        // `motion.poleContact`, `motion.rebound`, ... — through low-pass
        // filters, so a single `animate` call from an arbitrary prior
        // state does not represent the pose. `renderer3dSkiAvatar`'s
        // recovery Bezier gate reads `motion.cycle`; the port
        // (`solve_skierg`) reads `pose.cycle_frac`. Without a warm-up the
        // fixture's `motion.cycle` for the very first skierg sample is
        // 0.984 while `pose.cycle_frac` is 0, and the two branches
        // (Bezier vs polar arc) diverge by up to a metre on the hand
        // path — a scan artifact, not a port defect. Ten `animate` calls
        // per sample let the filters converge to within a fraction of a
        // percent (the smoothing time constants are well below ten
        // frames); the sample is recorded from the last call.
        let cues;
        const WARMUP_ITERATIONS = 10;
        for (let w = 0; w < WARMUP_ITERATIONS; w++) {
          cues = avatar.animate(pose.phase, false, pose, meters);
        }
        avatar.resolveWorldContacts?.();
        const recorded = sample(avatar, sport, pose, meters, scratch);
        recorded.cues = cues;
        if (sport === "rower") {
          const graph = sampleRowerMotionGraph(pose);
          recorded.oarSolve = rowerOarSolve(
            avatar,
            clamp(graph.body.armDraw.value, 0, 1),
            oarSolve,
          );
        }
        samples.push({ sport, sweep: sweep.id, phaseIndex: i, pose, rig: recorded });
      }
    }
  }

  const fixture = {
    schema: "rowplay.replay.rig-phase-parity.v1",
    description:
      "Web avatar rig calibration over the full stroke cycle: seat/oars/torso (rower), pole/torso/hand landmarks (skierg), crank/wheel/pedals (bike), plus the V4 contact target rig, sampled rig-local from renderer3d{Row,Ski,Bike}Avatar at the pinned rowplay commit.",
    sourceCommit: head,
    generatorVersion: GENERATOR_VERSION,
    sourceFileSha256s,
    avatarArgs: AVATAR_ARGS,
    samplesPerSweep: SAMPLES_PER_SWEEP,
    sweeps: SWEEPS,
    sampleCount: samples.length,
    samples,
  };
  await mkdir(dirname(out), { recursive: true });
  await writeFile(out, stableStringify(fixture) + "\n");
  console.log(`wrote ${out} (${samples.length} samples, ${Object.keys(sourceFileSha256s).length} hashed sources)`);
  return 0;
}

process.exit(await main());
