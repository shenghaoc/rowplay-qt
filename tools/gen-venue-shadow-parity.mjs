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
 * the flags the web's own objects carry, under the names the vendored GLBs
 * use, so the port's rule is checked against what the web renders rather
 * than against the porter's reading of its source.
 *
 * The scene is assembled as tools/bake-venues/bake.mjs assembles it (the
 * horizon rings, infield, apron and the world, then the bake's two naming
 * passes), with one difference: the three pieces the web's renderer builds
 * itself rather than the environment builder (`makeVerticalArc`, the
 * infield and the apron, renderer3d.ts) take the renderer's flags. The
 * generator reads those assignments from renderer3d.ts at the pin and fails
 * if the source no longer carries them, so a change there re-records.
 *
 * Usage (from the repository root; needs `pnpm install` in reference/rowplay):
 *
 *   node --experimental-transform-types \
 *     --import ./tools/bake-venues/register.mjs \
 *     tools/gen-venue-shadow-parity.mjs
 *
 * Writes tests/fixtures/replay-venue-shadow-parity.json.
 */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const OUT = join(REPO, "tests", "fixtures", "replay-venue-shadow-parity.json");
const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";

const ENVIRONMENT_PATH = "src/lib/replay/renderer3dEnvironment.ts";
const KIT_PATH = "src/lib/replay/renderer3dVenueKit.ts";
const RENDERER_PATH = "src/lib/replay/renderer3d.ts";

const SPORTS = ["rower", "skierg", "bike"];
/** Only these tiers cast a shadow (renderer3d.ts `QUALITY`, `shadows: true`). */
const TIERS = ["high", "ultra"];
/** tools/bake-venues/bake.mjs: the bake's seed, so the scatter matches. */
const SEED = 20260913;
const INNER_R = 22;
const OUTER_R = 34;
const WORLDS = { rower: "addRowerRegattaWorld", skierg: "addSkiStadiumWorld", bike: "addBikeCircuitWorld" };
/** The QUALITY fields the builder reads, as bake.mjs copies them. */
const QUALITY = {
  high: { laneSegments: 112, groundSegments: 32, displacement: true, shadows: true, environmentDetail: 2 },
  ultra: { laneSegments: 160, groundSegments: 64, displacement: true, shadows: true, environmentDetail: 3 },
};

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
  [ENVIRONMENT_PATH, KIT_PATH, RENDERER_PATH].map((path) => [path, readFileSync(join(REFERENCE, path), "utf8")]),
);

/**
 * The renderer's own receivers: the line that sets the flag, found inside
 * the named block of renderer3d.ts. Throws when the source moved on.
 */
function rendererFlag(anchor, assignment) {
  const source = sources[RENDERER_PATH];
  const start = source.indexOf(anchor);
  if (start === -1) throw new Error(`${RENDERER_PATH}: "${anchor}" not found`);
  const at = source.indexOf(assignment, start);
  const next = source.indexOf("\n  private ", start + anchor.length);
  if (at === -1 || (next !== -1 && at > next)) {
    throw new Error(`${RENDERER_PATH}: "${assignment}" not found in the block of "${anchor}"`);
  }
  return `${RENDERER_PATH}:${source.slice(0, at).split("\n").length}`;
}
const VERTICAL_ARC = rendererFlag("private makeVerticalArc(", "mesh.receiveShadow = this.cfg.shadows;");
const INFIELD = rendererFlag("infield.name = `environment:${this.sport}:infield`;", "infield.receiveShadow = this.cfg.shadows;");
const APRON = rendererFlag("apron.name = `environment:${this.sport}:apron`;", "apron.receiveShadow = this.cfg.shadows;");

Math.random = mulberry32(SEED);
const THREE = await import("three");
const { EnvironmentBuilder } = await import(pathToFileURL(join(REFERENCE, ENVIRONMENT_PATH)).href);
const { themed, FULL_CIRCLE } = await import(pathToFileURL(join(REFERENCE, KIT_PATH)).href);

/** Any colour will do: the flags do not depend on the palette. */
const grey = themed(0x808080, 0x404040);
function environmentStyle() {
  return new Proxy(
    { fogNear: 60, fogFar: 180, hemisphereIntensity: 1, sunIntensity: 2, fillIntensity: 0.6, exposure: 1, envIntensity: 0.5, hemisphereIntensityIbl: 0.5 },
    { get: (target, key) => (key in target ? target[key] : grey) },
  );
}

function buildVariant(sport, tier) {
  const cfg = QUALITY[tier];
  const environment = environmentStyle();
  const owners = new Map();
  const ctx = {
    sport,
    quality: tier,
    cfg,
    environment,
    textures: [],
    environmentThemeMats: [],
    mat: (m) => m,
    track: (g) => g,
    trackInstanced: (mesh) => mesh,
    environmentStandardMat: (name, color, opts = {}) =>
      Object.assign(new THREE.MeshStandardMaterial({ ...opts, color: color("light") }), { name }),
    environmentBasicMat: (name, color, opts = {}) =>
      Object.assign(new THREE.MeshBasicMaterial({ ...opts, color: color("light") }), { name }),
    // renderer3d.ts `makeVerticalArc`, flags included (see VERTICAL_ARC).
    makeVerticalArc: (name, radius, height, y, sector, material) => {
      const segments = Math.max(6, Math.ceil((cfg.laneSegments * sector.span) / FULL_CIRCLE));
      const geometry = new THREE.CylinderGeometry(radius, radius, height, segments, 1, true, sector.start, sector.span);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.name = name;
      mesh.position.y = y;
      mesh.receiveShadow = cfg.shadows;
      owners.set(mesh, VERTICAL_ARC);
      return mesh;
    },
  };

  const builder = new EnvironmentBuilder(ctx);
  const mid = new THREE.Group();
  mid.name = `environment:${sport}:midground`;
  const detail = new THREE.Group();
  detail.name = `environment:${sport}:detail`;
  if (sport !== "bike") {
    const farHeight = sport === "skierg" ? 22 : 12.5;
    const farVariation = sport === "skierg" ? 9 : 5.2;
    const midHeight = sport === "skierg" ? 12 : 8.4;
    const midVariation = sport === "skierg" ? 6 : 3.6;
    mid.add(builder.makeHorizonRing(`environment:${sport}:horizon-far`, 116, -2.5, farHeight, farVariation, 72, grey, 0.7));
    mid.add(builder.makeHorizonRing(`environment:${sport}:horizon-mid`, 84, -1.4, midHeight, midVariation, 64, grey, 2.1));
  }
  const surface = () => new THREE.MeshStandardMaterial({ color: 0x808080 });
  const infield = new THREE.Mesh(new THREE.CircleGeometry(INNER_R - 0.8, cfg.laneSegments), surface());
  infield.name = `environment:${sport}:infield`;
  infield.receiveShadow = cfg.shadows;
  owners.set(infield, INFIELD);
  if (sport === "rower" || sport === "skierg") infield.visible = false;
  const apron = new THREE.Mesh(new THREE.RingGeometry(OUTER_R + 0.2, 55, cfg.laneSegments), surface());
  apron.name = `environment:${sport}:apron`;
  apron.receiveShadow = cfg.shadows;
  owners.set(apron, APRON);
  builder[WORLDS[sport]](mid, detail, OUTER_R);

  const root = new THREE.Group();
  root.name = `venue-${sport}-${tier}`;
  root.add(mid, detail, infield, apron);

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
    meshes[object.name] = {
      cast: object.castShadow === true,
      receive: object.receiveShadow === true,
      source: owners.get(object) ?? ENVIRONMENT_PATH,
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
    "The key light's shadow flags of every venue mesh at the tiers that cast one, recorded from the web's own " +
    "objects under the names the vendored GLBs use (#96). `cast` / `receive` are three.js castShadow / " +
    "receiveShadow; `source` is the file (and, for the renderer's own pieces, the line) that set them.",
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
