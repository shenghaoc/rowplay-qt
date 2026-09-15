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

/** Collect the rig-local quantities the fixture pins, per sport. */
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

  const factories = {
    rower: makeRowerAvatar,
    skierg: makeSkierAvatar,
    bike: makeBikeAvatar,
  };
  const samples = [];
  for (const [sport, factory] of Object.entries(factories)) {
    const avatar = factory(AVATAR_ARGS.accent, AVATAR_ARGS.castShadow, AVATAR_ARGS.opacity, AVATAR_ARGS.bodySegments, AVATAR_ARGS.quality);
    // The avatar group stays at the origin and unrotated, so world
    // coordinates are rig-local coordinates (the fixture's frame).
    const scratch = new THREE.Vector3();
    for (const sweep of SWEEPS[sport]) {
      for (let i = 0; i < SAMPLES_PER_SWEEP; i++) {
        const pose = poseFor(sport, sweep, i);
        const meters = sport === "bike" ? i * 0.35 : 0;
        const cues = avatar.animate(pose.phase, false, pose, meters);
        avatar.resolveWorldContacts?.();
        const recorded = sample(avatar, sport, pose, meters, scratch);
        recorded.cues = cues;
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
  await writeFile(out, JSON.stringify(fixture, null, 1) + "\n");
  console.log(`wrote ${out} (${samples.length} samples, ${Object.keys(sourceFileSha256s).length} hashed sources)`);
  return 0;
}

process.exit(await main());
