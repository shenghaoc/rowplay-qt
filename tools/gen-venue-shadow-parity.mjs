#!/usr/bin/env node
// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Record which venue meshes the web lets cast and receive the key light's
 * shadow, for the tiers that have one (High and Ultra).
 *
 * three.js meshes cast and receive nothing unless flagged, and the web flags
 * a few: the overhead spans' legs and decks, the edge posts, the RowErg
 * finish tower, the ground surfaces that take a shadow. A Qt Quick 3D Model
 * casts and receives by default, and the GLB bake carries no flags, so the
 * port shadowed its whole venue: the BikeErg roof shell put the track in
 * shadow and the snow shadowed itself into acne (#96). This fixture records
 * the flags the web renders, under the names the vendored GLBs use, so the
 * port's rule is checked against what the web renders rather than against
 * the porter's reading of its source.
 *
 * The web's production renderer builds every variant: `CourseRenderer3D`,
 * constructed in Node as the web's own renderer3d.test.ts constructs it (a
 * stand-in `WebGLRenderer`, see tools/gen-venue-shadow-resolve.mjs, and a
 * stub `document` and `window`). Every flag is read off the object the web
 * would draw, whichever file set it: the environment builder's objects and
 * the renderer's own vertical arcs, infield and apron alike, with anything
 * the constructor does to them afterwards. One rendered frame must leave
 * every flag as recorded, or the generator fails.
 *
 * The venue's four roots are then taken out of the scene and named as
 * tools/bake-venues/bake.mjs names the GLBs' nodes: the meshes the web hides
 * are dropped, and an unnamed mesh takes `<parent>-part-<n>`.
 *
 * Usage (from the repository root; needs `pnpm install` in reference/rowplay):
 *
 *   node --experimental-transform-types tools/gen-venue-shadow-parity.mjs
 *
 * Writes tests/fixtures/replay-venue-shadow-parity.json.
 */
import { register } from "node:module";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

register("./gen-rig-phase-resolve.mjs", import.meta.url);
register("./gen-venue-shadow-resolve.mjs", import.meta.url);

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const OUT = join(REPO, "tests", "fixtures", "replay-venue-shadow-parity.json");
const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";

const RENDERER_PATH = "src/lib/replay/renderer3d.ts";
const ENVIRONMENT_PATH = "src/lib/replay/renderer3dEnvironment.ts";
const KIT_PATH = "src/lib/replay/renderer3dVenueKit.ts";
const STROKE_MODEL_PATH = "src/lib/replay/strokeModel.ts";

const SPORTS = ["rower", "skierg", "bike"];
/** Only these tiers cast a shadow (renderer3d.ts `QUALITY`, `shadows: true`). */
const TIERS = ["high", "ultra"];
/** tools/bake-venues/bake.mjs: the bake's seed, pinned before the web's modules load. */
const SEED = 20260913;
/** The venue's roots in the renderer's scene (renderer3d.ts `buildEnvironment`). */
const ROOTS = ["midground", "detail", "infield", "apron"];

function sha256(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const head = execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"]).toString().trim();
if (head !== PINNED_COMMIT) {
  throw new Error(`reference/rowplay is at ${head}, not the pinned ${PINNED_COMMIT}`);
}
const sources = Object.fromEntries(
  [RENDERER_PATH, ENVIRONMENT_PATH, KIT_PATH].map((path) => [path, readFileSync(join(REFERENCE, path), "utf8")]),
);

// renderer3d.test.ts: a 2D context and canvas stub for the text sprites, a
// `document` that makes them, and a `window` for the reduced-motion query.
function make2dContext() {
  const noop = () => {};
  return {
    font: "",
    fillStyle: "",
    textAlign: "",
    textBaseline: "",
    fillRect: noop,
    fillText: noop,
    clearRect: noop,
    beginPath: noop,
    roundRect: noop,
    fill: noop,
    stroke: noop,
    measureText: () => ({ width: 60 }),
  };
}
function makeCanvas() {
  const context = make2dContext();
  return {
    style: {},
    width: 0,
    height: 0,
    getContext: (type) => (type === "2d" ? context : null),
    remove: () => {},
  };
}
globalThis.document = {
  createElement: (tag) => {
    if (tag === "canvas") return makeCanvas();
    if (tag === "div") return { style: {}, dataset: {}, remove: () => {} };
    return {};
  },
};
globalThis.window = { devicePixelRatio: 1, matchMedia: () => ({ matches: false }) };

Math.random = mulberry32(SEED);
const url = (path) => pathToFileURL(join(REFERENCE, path)).href;
const { CourseRenderer3D } = await import(url(RENDERER_PATH));
const { buildStrokeTimeline, strokePoseAt } = await import(url(STROKE_MODEL_PATH));
const THREE = await import("three");

/** Every mesh's flags under `root`, keyed by the object. */
function flagsOf(root) {
  const flags = new Map();
  root.traverse((object) => {
    if (object.isMesh) flags.set(object, [object.castShadow === true, object.receiveShadow === true]);
  });
  return flags;
}

/** renderer3d.test.ts `makeRenderState`: two strokes, the frame between them. */
function renderState(sport) {
  const timeline = buildStrokeTimeline(
    [
      { t: 2, d: 10, pace: 120, spm: 28, watts: 160 },
      { t: 4, d: 21, pace: 118, spm: 30, watts: 190 },
    ],
    sport,
    true,
  );
  return {
    frame: { t: 2.1, d: 100, pace: 120, spm: 28, watts: 100, hr: 0 },
    ghost: null,
    strokePose: strokePoseAt(timeline, 2.1),
    distFrac: 0.5,
    totalDistance: 2000,
    sport,
  };
}

function buildVariant(sport, tier) {
  const host = { appendChild: () => {}, children: [] };
  const renderer = new CourseRenderer3D(host, tier, sport);
  const scene = renderer.scene;
  const roots = ROOTS.map((suffix) => {
    const object = scene.getObjectByName(`environment:${sport}:${suffix}`);
    if (!object) throw new Error(`${sport}:${tier}: no environment:${sport}:${suffix} in the scene`);
    return object;
  });

  // The flags as constructed must be the flags a rendered frame leaves.
  const constructed = new Map(roots.flatMap((root) => [...flagsOf(root)]));
  renderer.render(renderState(sport), false, "light");
  for (const root of roots) {
    for (const [object, [cast, receive]] of flagsOf(root)) {
      const before = constructed.get(object);
      if (!before || before[0] !== cast || before[1] !== receive) {
        throw new Error(`${sport}:${tier}: a frame changed ${object.name || "an unnamed mesh"}'s flags`);
      }
    }
  }

  // bake.mjs `bakeVariant`: the four roots under one venue root.
  const root = new THREE.Group();
  root.name = `venue-${sport}-${tier}`;
  root.add(...roots);
  // bake.mjs pass 0: meshes the web hides are not in the GLB.
  const invisible = [];
  root.traverse((object) => {
    if (object.isMesh && !object.visible) invisible.push(object);
  });
  for (const object of invisible) object.parent?.remove(object);
  // bake.mjs: an unnamed mesh takes `<parent>-part-<n>`.
  root.traverse((object) => {
    if (!object.isMesh || object.name) return;
    const parentName = object.parent?.name || `venue-${sport}-${tier}`;
    let index = 0;
    for (const sibling of object.parent?.children ?? []) {
      if (sibling === object) break;
      if (sibling.isMesh) index += 1;
    }
    object.name = `${parentName}-part-${index + 1}`;
  });

  const meshes = {};
  root.traverse((object) => {
    if (!object.isMesh) return;
    if (object.name in meshes) throw new Error(`${sport}:${tier}: two meshes named ${object.name}`);
    const path = [];
    for (let node = object.parent; node && node !== root; node = node.parent) path.unshift(node.name);
    meshes[object.name] = {
      cast: object.castShadow === true,
      receive: object.receiveShadow === true,
      parent: path.join(" / "),
    };
  });
  return Object.fromEntries(Object.entries(meshes).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
}

const variants = {};
for (const sport of SPORTS) {
  for (const tier of TIERS) variants[`${sport}:${tier}`] = buildVariant(sport, tier);
}

const fixture = {
  description:
    "The key light's shadow flags of every venue mesh at the tiers that cast one, read off the objects the " +
    "web's production renderer (`CourseRenderer3D`, built headless as its own tests build it) puts in its " +
    "scene, under the names the vendored GLBs use (#96). `cast` / `receive` are three.js castShadow / " +
    "receiveShadow; `parent` is the mesh's ancestry below the venue root.",
  generator: "tools/gen-venue-shadow-parity.mjs",
  source: {
    repo: "https://github.com/shenghaoc/rowplay",
    commit: PINNED_COMMIT,
    sha256: Object.fromEntries(Object.entries(sources).map(([path, text]) => [path, sha256(text)])),
  },
  variants,
};
writeFileSync(OUT, `${JSON.stringify(fixture, null, 2)}\n`);
const counts = Object.entries(variants).map(([key, meshes]) => {
  const values = Object.values(meshes);
  return `${key}: ${values.length} meshes, ${values.filter((m) => m.cast).length} cast, ${values.filter((m) => m.receive).length} receive`;
});
console.log(`wrote ${OUT}\n  ${counts.join("\n  ")}`);
