// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Generate the V4 hand-continuity oracle for the dense guard (issue #40).
 *
 * One command, from the repository root (needs `pnpm install` in
 * reference/rowplay first, Node >= 23.6):
 *
 *   node --experimental-transform-types tools/gen-v4-hand-parity.mjs
 *
 * The dense guard (`rowplay-viewmodel` `pose.rs`,
 * `requested_twist_stays_continuous_and_engages_the_budgets`) used to bound
 * the rendered hand's per-step motion with a continuity budget, with the two
 * SkiErg wrist-refinement ±π windows carved out by cycle fraction. That is
 * the wrong shape of assert: the port is *faithful* inside those windows (the
 * web's own hand snaps 1.2991 rad at cyc 0.2635 and 1.0961 at cyc 0.7080), so
 * "stay under 0.35 rad" and "reproduce the web's snaps" cannot both hold.
 *
 * This generator records what the web actually RENDERS through the same
 * 2000-step sweep the guard runs, so the guard can compare against the oracle
 * per step instead of against a budget: the rendered hand's per-step position
 * and orientation deltas, the per-step delta of the contact point the V4
 * chain is driven onto (the sport's `gripEffectorOffsets` channel), and the
 * residual between that contact point and the target the sport hands it.
 * Where the web snaps, the port may snap; where the web is smooth, the port
 * must be smooth.
 *
 * Instrument shape (AGENTS.md: "a generator that builds the web's real object
 * and samples its animated output records independent observables"): the
 * generator installs the real `ReplayV4MotionController` on the real SkiErg
 * avatar (the same wiring `renderer3d.ts` performs in production — same
 * `gripEffectorOffsets`, same avatar args) and drives it exactly the way the
 * guard drives the port (`fallbackStrokePose("skierg", phase, 30)`,
 * `meters = step · 3`), reading the rendered bone transforms out of the scene
 * graph after each frame. It reproduces nothing about the port.
 */
import { register } from "node:module";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

register("./gen-rig-phase-resolve.mjs", import.meta.url);

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const REPLAY = join(REFERENCE, "src", "lib", "replay");
const OUT_DEFAULT = join(REPO, "tests", "fixtures", "replay-v4-hand-parity.json");

/** The reference commit this generator is pinned to (docs/source-map.md). */
const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";
const GENERATOR_VERSION = "gen-v4-hand-parity/1.0.0";

/** The web sources whose evaluation this fixture captures. */
const SOURCE_FILES = [
  "renderer3dSkiAvatar.ts",
  "renderer3dV4Motion.ts",
  "renderer3dV4Assets.ts",
  "renderer3dAvatarKit.ts",
  "figurePose.ts",
  "handGrip.ts",
  "skiGripReach.ts",
  "strokeModel.ts",
  "motionGraph.ts",
  "motion.ts",
  "skiEquipment.ts",
];

/** The guard's sampling density (`requested_twist_stays_continuous_...`). */
const STEPS = 2000;
/** The guard's stroke rate and distance advance. */
const RATE_SPM = 30;
const METERS_PER_STEP = 3.0;
/** The guard's drive: `fallbackStrokePose(sport, phase, 30)`. */
const AVATAR_ARGS = {
  accent: 0x3b82f6,
  castShadow: false,
  opacity: 1,
  bodySegments: 16,
  quality: "high",
};

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function gitHead() {
  return execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
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
    sourceFileSha256s[file] = sha256(await readFile(join(REPLAY, file)));
  }

  const url = (file) => pathToFileURL(join(REPLAY, file)).href;
  const THREE = await import("three");
  const { makeSkierAvatar } = await import(url("renderer3dSkiAvatar.ts"));
  const { installReplayV4MotionController } = await import(url("renderer3dV4Motion.ts"));
  const { fetchReplayV4Asset, tryCreateReplayV4AthleteInstance } = await import(
    url("renderer3dV4Assets.ts")
  );
  const { fallbackStrokePose } = await import(url("strokeModel.ts"));
  const { HAND_FIST_CENTRE } = await import(url("handGrip.ts"));

  // The production asset load (renderer3dLoader.ts → loadReplayV4Asset).
  const glb = await readFile(join(REFERENCE, "static", "replay-assets", "rowplay-athlete-v4.glb"));
  const template = await fetchReplayV4Asset(
    async () =>
      new Response(new Uint8Array(glb.buffer, glb.byteOffset, glb.byteLength), { status: 200 }),
  );

  // The production wiring (`renderer3d.ts`): avatar, parented rig (the skierg
  // pole solver early-returns without a course parent), the sport's grip
  // effector offsets and the V4 controller over the same targets.
  const avatar = makeSkierAvatar(
    AVATAR_ARGS.accent,
    AVATAR_ARGS.castShadow,
    AVATAR_ARGS.opacity,
    AVATAR_ARGS.bodySegments,
    AVATAR_ARGS.quality,
  );
  const scene = new THREE.Scene();
  scene.add(avatar.group);
  const instance = tryCreateReplayV4AthleteInstance(template);
  const motion = installReplayV4MotionController({
    sport: "skierg",
    parent: avatar.group,
    fallbackRoot: avatar.group,
    instance,
    targets: avatar.v4Targets,
    quality: AVATAR_ARGS.quality,
    castShadow: AVATAR_ARGS.castShadow,
    receiveShadow: false,
    effectorOffsets: {
      leftHand: { x: -HAND_FIST_CENTRE.x, y: HAND_FIST_CENTRE.y, z: HAND_FIST_CENTRE.z },
      rightHand: { x: HAND_FIST_CENTRE.x, y: HAND_FIST_CENTRE.y, z: HAND_FIST_CENTRE.z },
    },
  });
  if (!motion) throw new Error("no V4 controller");
  if (motion.root.parent !== avatar.group) throw new Error("V4 root is not on the avatar");

  const sides = [
    { name: "left", sign: -1, bone: "v4LeftHand", target: "leftHand", forearm: "v4LeftForearm" },
    { name: "right", sign: 1, bone: "v4RightHand", target: "rightHand", forearm: "v4RightForearm" },
  ];
  for (const side of sides) {
    if (!instance.bones[side.bone] || !instance.bones[side.forearm]) {
      throw new Error(`missing bone ${side.bone}`);
    }
  }

  const scratch = new THREE.Vector3();
  const handWorld = new THREE.Vector3();
  const contactWorld = new THREE.Vector3();
  const targetWorld = new THREE.Vector3();
  const quaternion = new THREE.Quaternion();
  const state = new Map(
    sides.map((side) => [
      side.name,
      { position: null, quaternion: null, contact: null, target: null, forearm: null },
    ]),
  );
  const samples = [];

  for (let step = 0; step < STEPS; step++) {
    const cycle = step / STEPS;
    const pose = fallbackStrokePose("skierg", cycle * Math.PI * 2, RATE_SPM);
    avatar.animate(pose.phase, false, pose, step * METERS_PER_STEP);
    scene.updateMatrixWorld(true);
    // `renderer3d.ts`'s drive order, verbatim: prepare → refineV4Targets →
    // resolveWorldContacts → constrain.
    if (!motion.prepare(pose)) {
      throw new Error(`prepare failed at ${step}: ${motion.root.userData.replayV4Failure}`);
    }
    avatar.refineV4Targets?.(motion);
    scene.updateMatrixWorld(true);
    avatar.resolveWorldContacts?.();
    if (!motion.constrain()) {
      throw new Error(`constrain failed at ${step}: ${motion.root.userData.replayV4Failure}`);
    }
    scene.updateMatrixWorld(true);

    const row = { step, cycle };
    for (const side of sides) {
      const bone = instance.bones[side.bone];
      bone.getWorldPosition(handWorld);
      bone.getWorldQuaternion(quaternion);
      // The contact point the chain is driven onto: the sport's grip effector
      // offset (`renderer3d.ts` `gripEffectorOffsets`) in the hand bone's
      // frame — the V4 arithmetic `solvePositionTowardTarget` performs.
      contactWorld.set(
        side.sign * HAND_FIST_CENTRE.x,
        HAND_FIST_CENTRE.y,
        HAND_FIST_CENTRE.z,
      );
      bone.localToWorld(contactWorld);
      avatar.v4Targets[side.target].getWorldPosition(targetWorld);
      const previous = state.get(side.name);
      const forearm = instance.bones[side.forearm].getWorldPosition(scratch);
      // Wrap-aware orientation delta: the quaternion double cover is
      // collapsed with |dot|, so the result is the shorter-arc angle in
      // [0, PI] — the port's guard metric after the issue-#40 fix.
      const orientation = previous.quaternion
        ? 2 *
          Math.acos(
            Math.min(
              1,
              Math.abs(
                quaternion.x * previous.quaternion.x +
                  quaternion.y * previous.quaternion.y +
                  quaternion.z * previous.quaternion.z +
                  quaternion.w * previous.quaternion.w,
              ),
            ),
          )
        : 0;
      row[`${side.name}HandDelta`] = previous.position
        ? handWorld.distanceTo(previous.position)
        : 0;
      row[`${side.name}ContactDelta`] = previous.contact
        ? contactWorld.distanceTo(previous.contact)
        : 0;
      row[`${side.name}OrientationDelta`] = orientation;
      row[`${side.name}ContactResidual`] = contactWorld.distanceTo(targetWorld);
      row[`${side.name}ForearmDelta`] = previous.forearm
        ? forearm.distanceTo(previous.forearm)
        : 0;
      state.set(side.name, {
        position: handWorld.clone(),
        quaternion: {
          x: quaternion.x,
          y: quaternion.y,
          z: quaternion.z,
          w: quaternion.w,
        },
        contact: contactWorld.clone(),
        target: targetWorld.clone(),
        forearm: forearm.clone(),
      });
    }
    samples.push(row);
  }

  // Self-check: the chain must actually close on the driven target. The
  // production V4 loop is a fixed point, so a residual here means the drive
  // order above is wrong and the fixture must not be written.
  const residual = samples.reduce(
    (worst, row) =>
      Math.max(worst, row.leftContactResidual, row.rightContactResidual),
    0,
  );
  if (!(residual < 1e-6)) {
    throw new Error(
      `V4 contact self-check failed: worst |contact − target| = ${residual} — the drive ` +
        "order does not close the chain; do not write the fixture",
    );
  }
  const handDelta = samples.reduce((worst, row) => Math.max(worst, row.leftHandDelta), 0);
  if (!(handDelta > 0.01)) {
    throw new Error(
      `V4 hand self-check failed: worst |Δhand| = ${handDelta} — the avatar did not move; ` +
        "do not write the fixture",
    );
  }

  const fixture = {
    schema: "rowplay-qt.replay-v4-hand-parity.v1",
    description:
      "Per-step V4 SkiErg hand continuity the web's own rig renders through the dense " +
      "guard's sweep (issue #40): the oracle the guard compares against instead of a " +
      "continuity budget. Regenerate with tools/gen-v4-hand-parity.mjs.",
    sourceCommit: PINNED_COMMIT,
    generatorVersion: GENERATOR_VERSION,
    sourceFileSha256s,
    avatarArgs: AVATAR_ARGS,
    sport: "skierg",
    steps: STEPS,
    rateSpm: RATE_SPM,
    metersPerStep: METERS_PER_STEP,
    effectorOffsets: {
      left: { x: -HAND_FIST_CENTRE.x, y: HAND_FIST_CENTRE.y, z: HAND_FIST_CENTRE.z },
      right: { x: HAND_FIST_CENTRE.x, y: HAND_FIST_CENTRE.y, z: HAND_FIST_CENTRE.z },
    },
    sampleCount: samples.length,
    samples,
  };
  await writeFile(out, stableStringify(fixture) + "\n");
  console.log(
    `wrote ${out} (${samples.length} steps, worst contact residual ${residual.toExponential(2)}, ` +
      `worst |Δhand| ${handDelta.toFixed(4)}, ${Object.keys(sourceFileSha256s).length} hashed sources)`,
  );
  return 0;
}

process.exit(await main());
