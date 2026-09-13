// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Bake the web app's procedural replay venues to vendored .glb assets.
 *
 * One command, from the repository root (ADR 0010; requires `pnpm install`
 * in reference/rowplay first):
 *
 *   node --experimental-transform-types \
 *     --import ./tools/bake-venues/register.mjs \
 *     tools/bake-venues/bake.mjs --all
 *
 * `--all` bakes each of the 12 variants (3 sports x 4 tiers) in its own child
 * process, one at a time: a three.js venue build retains a lot of geometry,
 * and reusing one process for the whole set accumulates every variant's
 * allocations. After the 12, one variant is re-baked into a temp directory
 * and compared byte for byte (R2.4). Use `--only <sport>:<tier>` to bake a
 * single variant in-process.
 *
 * Outputs, under build/venues-staging/:
 *   rowplay-venue-<sport>-<tier>.glb    archetype + static geometry, no textures
 *   rowplay-venue-<sport>-<tier>.json   the runtime contract (materials, colours,
 *                                       texture bindings, instance lists)
 *   procedural/<map>-<size>.png         the web builder's procedural maps
 *
 * Then run the Blender hygiene pass (cleanup.py) over the staged GLBs and
 * `--finalize`, which re-derives the contract inventory and bounds from the
 * cleaned artifacts.
 *
 * Determinism: Math.random is pinned (mulberry32, SEED) before the builder
 * module is imported, so its SimplexNoise permutation is fixed; all scatter
 * jitter in the web source is already index-hashed. Every process seeds
 * identically, so per-variant child processes agree.
 */
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { encodeRgbaPng } from "./png.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..", "..");
const REFERENCE = join(REPO, "reference", "rowplay");
const STAGING = join(REPO, "build", "venues-staging");
const BUILDER_URL = pathToFileURL(join(REFERENCE, "src/lib/replay/renderer3dEnvironment.ts")).href;
const KIT_URL = pathToFileURL(join(REFERENCE, "src/lib/replay/renderer3dVenueKit.ts")).href;

/** Pin for the noise permutation table; recorded in every contract. */
const SEED = 20260913;

const SPORTS = ["rower", "skierg", "bike"];
const TIERS = ["low", "medium", "high", "ultra"];

/**
 * The reference commit this baker was authored against. The contract records
 * the checkout's actual HEAD at bake time and this value; a mismatch is a
 * reviewer's cue to re-check the copied tables below (ADR 0010).
 */
const AUTHORED_AGAINST = "011e8303b66b4d2265a6f1ec8b3ed9d8ed497086";

// ---------------------------------------------------------------------------
// Tables copied from the pinned web source. They are module-private there, so
// the baker carries verified copies; each block cites its source lines.
// ---------------------------------------------------------------------------

/** rowplay renderer3d.ts:65-138 (`QUALITY`). */
const QUALITY = {
  low: { dprCap: 1, antialias: false, laneSegments: 48, groundSegments: 1, displacement: false, shadows: false, shadowMapSize: 0, wake: 0, buoys: true, buoysPerRing: 12, buoyRings: 2, spray: false, sprayParticles: 0, sprayPerCatch: 0, environmentDetail: 0, bodySegments: 10 },
  medium: { dprCap: 2, antialias: true, laneSegments: 80, groundSegments: 20, displacement: true, shadows: false, shadowMapSize: 0, wake: 20, buoys: true, buoysPerRing: 18, buoyRings: 2, spray: true, sprayParticles: 64, sprayPerCatch: 7, environmentDetail: 1, bodySegments: 14 },
  high: { dprCap: 2, antialias: true, laneSegments: 112, groundSegments: 32, displacement: true, shadows: true, shadowMapSize: 1024, wake: 32, buoys: true, buoysPerRing: 22, buoyRings: 2, spray: true, sprayParticles: 80, sprayPerCatch: 8, environmentDetail: 2, bodySegments: 18 },
  ultra: { dprCap: 3, antialias: true, laneSegments: 160, groundSegments: 64, displacement: true, shadows: true, shadowMapSize: 2048, wake: 52, buoys: true, buoysPerRing: 28, buoyRings: 2, spray: true, sprayParticles: 112, sprayPerCatch: 10, environmentDetail: 3, bodySegments: 24 },
};

/** rowplay renderer3d.ts:414-528 (`ENVIRONMENTS`), each value a [light, dark] pair. */
const ENVIRONMENTS = {
  rower: {
    skyZenith: [0x4b91bd, 0x0b2639], skyHorizon: [0xdceef1, 0x557c8c], skyNadir: [0x5f8f8e, 0x12333c],
    fog: [0xbfd4ca, 0x2e5059], fogNear: 62, fogFar: 178,
    hemisphereSky: [0xf2fbff, 0x7eabb9], hemisphereGround: [0x426c5b, 0x173535], hemisphereIntensity: 1.3,
    sun: [0xffedc1, 0xffce82], sunIntensity: 2.55, fill: [0xbfe9f1, 0x568b9a], fillIntensity: 0.7, exposure: 1.06,
    farSilhouette: [0x78947b, 0x23454a], midSilhouette: [0x3f6d50, 0x1c493d],
    venueStructure: [0xe8e4d8, 0x7f9093], venueAccent: [0xa65e3b, 0xd9a066],
    infield: [0x2f8198, 0x124f62], apron: [0x3c91a3, 0x175a69],
    envIntensity: 0, hemisphereIntensityIbl: 1.22,
  },
  skierg: {
    skyZenith: [0x1f3c58, 0x102b45], skyHorizon: [0xa2b4c4, 0x6f8d9f], skyNadir: [0x7e97a9, 0x385669],
    fog: [0x64798c, 0x718d9b], fogNear: 74, fogFar: 205,
    hemisphereSky: [0xb9cfe2, 0xa0bfd0], hemisphereGround: [0x5d7789, 0x405c6d], hemisphereIntensity: 0.9,
    sun: [0xffe8c0, 0xeaf6ff], sunIntensity: 2.7, fill: [0x7fa3c4, 0x719bb5], fillIntensity: 0.62, exposure: 1.02,
    farSilhouette: [0x9a8b96, 0x506f82], midSilhouette: [0x435d72, 0x315267],
    venueStructure: [0x25394a, 0x192f3d], venueAccent: [0xe04852, 0xff6670],
    infield: [0xcfdfe9, 0x9fb7c5], apron: [0xdfeaf1, 0xc7d7df],
    envIntensity: 0.62, hemisphereIntensityIbl: 0.36,
  },
  bike: {
    skyZenith: [0x2e3540, 0x1c2a38], skyHorizon: [0x4a4e54, 0x344252], skyNadir: [0x33383f, 0x202d36],
    fog: [0x3d4249, 0x2b3944], fogNear: 72, fogFar: 165,
    hemisphereSky: [0x8a8f96, 0x707f8e], hemisphereGround: [0x6b5844, 0x312d28], hemisphereIntensity: 1.15,
    sun: [0xffe2b0, 0xffd49a], sunIntensity: 3.5, fill: [0x7d94a8, 0x6d8ba0], fillIntensity: 0.62, exposure: 1.16,
    farSilhouette: [0x525c63, 0x263442], midSilhouette: [0x454f58, 0x314352],
    venueStructure: [0x5a646c, 0x344452], venueAccent: [0xd79a50, 0xf0b667],
    infield: [0x6d7a74, 0x405149], apron: [0x847d70, 0x3c3c3a],
    envIntensity: 0.72, hemisphereIntensityIbl: 0.34,
  },
};

/** The web loop radii the venue is placed around (renderer3d.ts:824-826, 2025-2026). */
const INNER_R = 22; // ghostRadius (26) - 4
const OUTER_R = 34; // loopRadius (30) + 4

const WORLDS = { rower: "addRowerRegattaWorld", skierg: "addSkiStadiumWorld", bike: "addBikeCircuitWorld" };

const TEXTURE_SLOTS = ["map", "roughnessMap", "normalMap"];
const UNEXPECTED_SLOTS = ["emissiveMap", "aoMap", "metalnessMap", "alphaMap", "lightMap", "specularMap"];

// ---------------------------------------------------------------------------
// Node shims (the web repo's build-replay-rig-v4.mjs precedent).
// ---------------------------------------------------------------------------

// GLTFExporter's binary path uses Blob + FileReader; Node's Blob has the bytes.
globalThis.FileReader ??= class FileReader {
  readAsArrayBuffer(blob) {
    void blob.arrayBuffer().then((buffer) => {
      this.result = buffer;
      this.onloadend?.();
    });
  }
  readAsDataURL(blob) {
    void blob.arrayBuffer().then((buffer) => {
      this.result = `data:${blob.type || "application/octet-stream"};base64,${Buffer.from(buffer).toString("base64")}`;
      this.onloadend?.();
    });
  }
};

function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const round5 = (v) => Math.round(v * 1e5) / 1e5;
const hexColor = (v) => "#" + v.toString(16).padStart(6, "0");

// Pin before the builder module is evaluated: its module-scope SimplexNoise
// singleton seeds its permutation table from Math.random.
Math.random = mulberry32(SEED);
const THREE = await import("three");
const { EnvironmentBuilder } = await import(BUILDER_URL);
const { themed, FULL_CIRCLE } = await import(KIT_URL);
const { GLTFExporter } = await import("three/examples/jsm/exporters/GLTFExporter.js");

/** Resolve a table entry to a `ThemeColor`-shaped function (or pass scalars through). */
function makeEnvironment(sport) {
  const environment = {};
  for (const [key, value] of Object.entries(ENVIRONMENTS[sport])) {
    environment[key] = Array.isArray(value) ? themed(value[0], value[1]) : value;
  }
  return environment;
}

/** Record the environment style's resolved light-theme values for the runtime. */
function serialiseEnvironment(environment) {
  const out = {};
  for (const [key, value] of Object.entries(environment)) {
    out[key] = typeof value === "function" ? hexColor(value("light")) : value;
  }
  return out;
}

async function referenceCommit() {
  const { execFileSync } = await import("node:child_process");
  return execFileSync("git", ["-C", REFERENCE, "rev-parse", "HEAD"]).toString().trim();
}

// ---------------------------------------------------------------------------
// Bake one variant in this process.
// ---------------------------------------------------------------------------

async function bakeVariant(sport, tier, outDir) {
  const cfg = QUALITY[tier];
  const environment = makeEnvironment(sport);

  // Registry captures mirror the renderer's helper closures
  // (renderer3d.ts:1247-1308) and additionally record the light/dark colour
  // pair each themed material was authored with, for the runtime contract.
  const themeColors = new Map();
  const materialOrder = [];

  const ctx = {
    sport,
    quality: tier,
    cfg,
    environment,
    textures: [],
    environmentThemeMats: [],
    mat: (m) => (materialOrder.includes(m) ? m : (materialOrder.push(m), m)),
    track: (g) => g,
    trackInstanced: (mesh) => mesh,
    environmentStandardMat: (name, color, opts = {}) => {
      const material = new THREE.MeshStandardMaterial({ ...opts, color: color("light") });
      material.name = name;
      materialOrder.push(material);
      themeColors.set(material, [color("light"), color("dark")]);
      return material;
    },
    environmentBasicMat: (name, color, opts = {}) => {
      const material = new THREE.MeshBasicMaterial({ ...opts, color: color("light") });
      material.name = name;
      materialOrder.push(material);
      themeColors.set(material, [color("light"), color("dark")]);
      return material;
    },
    // Production formula (renderer3d.ts:1722-1748).
    makeVerticalArc: (name, radius, height, y, sector, material) => {
      const segments = Math.max(6, Math.ceil((cfg.laneSegments * sector.span) / FULL_CIRCLE));
      const geometry = new THREE.CylinderGeometry(radius, radius, height, segments, 1, true, sector.start, sector.span);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.name = name;
      mesh.position.y = y;
      return mesh;
    },
  };

  const builder = new EnvironmentBuilder(ctx);
  const mid = new THREE.Group();
  mid.name = `environment:${sport}:midground`;
  const detail = new THREE.Group();
  detail.name = `environment:${sport}:detail`;

  // Horizon rings as the renderer adds them (renderer3d.ts:1759-1795):
  // BikeErg is indoors; the far ring only exists at detail >= 1.
  if (sport !== "bike") {
    const farHeight = sport === "skierg" ? 22 : 12.5;
    const farVariation = sport === "skierg" ? 9 : 5.2;
    const midHeight = sport === "skierg" ? 12 : 8.4;
    const midVariation = sport === "skierg" ? 6 : 3.6;
    if (cfg.environmentDetail >= 1) {
      mid.add(builder.makeHorizonRing(`environment:${sport}:horizon-far`, 116, -2.5, farHeight, farVariation, 72, environment.farSilhouette, 0.7));
    }
    mid.add(builder.makeHorizonRing(`environment:${sport}:horizon-mid`, 84, -1.4, midHeight, midVariation, 64, environment.midSilhouette, 2.1));
  }

  // Infield + apron (renderer3d.ts:1797-1856), procedural map slots included.
  const infieldMat = ctx.environmentStandardMat(`environment:${sport}:infield-material`, environment.infield, {
    roughness: sport === "rower" ? 0.2 : 0.9,
    metalness: sport === "rower" ? 0.06 : 0.01,
  });
  if (sport === "skierg" && cfg.environmentDetail >= 1) {
    infieldMat.map = builder.makeSnowSurfaceTexture(cfg.environmentDetail);
    infieldMat.needsUpdate = true;
  }
  if (sport === "rower" && cfg.environmentDetail >= 1) {
    infieldMat.map = builder.makeWaterSurfaceTexture(cfg.environmentDetail);
    infieldMat.needsUpdate = true;
  }
  const infield = new THREE.Mesh(new THREE.CircleGeometry(INNER_R - 0.8, cfg.laneSegments), infieldMat);
  infield.name = `environment:${sport}:infield`;
  infield.rotation.x = -Math.PI / 2;
  infield.position.y = sport === "bike" ? -0.04 : -0.015;
  // Row island and Ski bowl centre own their geometry; the generic underlay is
  // hidden for them (renderer3d.ts:1827-1829). Kept in the bake and listed in
  // `hidden` so the structural inventory stays tier-stable.
  if (sport === "rower" || sport === "skierg") infield.visible = false;

  const apronMat = ctx.environmentStandardMat(`environment:${sport}:apron-material`, environment.apron, {
    roughness: sport === "rower" ? 0.22 : 0.9,
    metalness: sport === "rower" ? 0.05 : 0.01,
  });
  if (sport === "skierg" && cfg.environmentDetail >= 1) {
    apronMat.map = builder.makeSnowSurfaceTexture(cfg.environmentDetail);
    apronMat.needsUpdate = true;
  }
  if (sport === "rower" && cfg.environmentDetail >= 1) {
    apronMat.map = builder.makeWaterSurfaceTexture(cfg.environmentDetail);
    apronMat.needsUpdate = true;
  }
  const apron = new THREE.Mesh(new THREE.RingGeometry(OUTER_R + 0.2, 55, cfg.laneSegments), apronMat);
  apron.name = `environment:${sport}:apron`;
  apron.rotation.x = -Math.PI / 2;
  apron.position.y = -0.005;

  // The world itself (renderer3d.ts:1858-1876).
  builder[WORLDS[sport]](mid, detail, OUTER_R);

  const root = new THREE.Group();
  root.name = `venue-${sport}-${tier}`;
  root.add(mid, detail, infield, apron);

  return harvestAndExport({ root, sport, tier, cfg, environment, outDir, themeColors, materialOrder });
}

// ---------------------------------------------------------------------------
// Harvest the runtime contract, strip textures, bake UV repeats, split
// instances out of the graph, export the GLB.
// ---------------------------------------------------------------------------

async function harvestAndExport({ root, sport, tier, cfg, environment, outDir, themeColors, materialOrder }) {
  const label = `${sport}:${tier}`;
  const proceduralPngs = new Map(); // png relative path -> Buffer
  const contractMaterials = new Map(); // name -> contract entry
  const instancing = {};
  const hidden = [];
  const repeatByGeometry = new Map();
  const geometryRepeatOwner = new Map(); // geometry -> mesh name (for conflict messages)

  // Pass 0: drop meshes the web marks invisible (the RowErg/SkiErg generic
  // infield underlay, whose centre geometry replaces it). They are not
  // rendered in the web either; Blender would drop them on export anyway, so
  // keeping them would desync the contract from the GLB. Record the names.
  const invisible = [];
  root.traverse((object) => {
    if (object.isMesh && !object.visible) invisible.push(object);
  });
  for (const object of invisible) {
    hidden.push(object.name);
    object.parent?.remove(object);
  }

  // The web builder names landmark groups but leaves some child meshes (the
  // pavilion bodies, roofs, glazing) unnamed. Unnamed glTF nodes cannot be
  // addressed by the runtime and Blender renames them to `Mesh_N` on
  // round-trip, so give each a deterministic name from its parent first.
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

  function materialEntry(material) {
    const name = material.name;
    if (!name) throw new Error(`${label}: unnamed material on a venue mesh`);
    if (contractMaterials.has(name)) return contractMaterials.get(name);
    const theme = themeColors.get(material);
    const entry = {
      type: material.isMeshBasicMaterial ? "basic" : material.isMeshPhysicalMaterial ? "physical" : "standard",
      color: {
        light: hexColor(theme ? theme[0] : material.color.getHex()),
        dark: hexColor(theme ? theme[1] : material.color.getHex()),
      },
      roughness: material.roughness ?? 1,
      metalness: material.metalness ?? 0,
      opacity: round5(material.opacity ?? 1),
      alphaMode: material.transparent ? "blend" : "opaque",
      doubleSided: material.side === THREE.DoubleSide,
      vertexColors: material.vertexColors === true,
      depthWrite: material.depthWrite !== false,
      textures: {},
    };
    if (material.normalScale) entry.normalScale = [round5(material.normalScale.x), round5(material.normalScale.y)];
    contractMaterials.set(name, entry);
    return entry;
  }

  function recordTextureBinding(entry, slot, texture, materialName) {
    const image = texture.image;
    if (image && image.data && image.width) {
      // Procedural DataTexture from the web builder: encode with our own
      // deterministic encoder (GLTFExporter's image path needs a canvas).
      const base = texture.name.replace(/^environment:texture:/, "").replace(/-(ultra|high)$/, "");
      const png = `procedural/${base}-${image.width}.png`;
      if (!proceduralPngs.has(png)) {
        proceduralPngs.set(png, encodeRgbaPng(image.width, image.height, image.data));
      }
      entry.textures[slot] = {
        kind: "procedural", name: base, size: image.width,
        repeat: [round5(texture.repeat.x), round5(texture.repeat.y)], png,
      };
    } else if (texture.userData.sourcePath) {
      entry.textures[slot] = {
        kind: "set", path: texture.userData.sourcePath,
        repeat: [round5(texture.repeat.x), round5(texture.repeat.y)],
      };
    } else {
      throw new Error(`${label}: texture in slot ${slot} of ${materialName} has neither pixel data nor a source path`);
    }
  }

  // Pass 1: materials, texture bindings, per-geometry UV repeat collection.
  root.traverse((object) => {
    if (!object.isMesh && !object.isInstancedMesh) return;
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    let objectRepeat = null;
    for (const material of materials) {
      const entry = materialEntry(material);
      for (const slot of TEXTURE_SLOTS) {
        if (material[slot]) recordTextureBinding(entry, slot, material[slot], material.name);
      }
      for (const slot of UNEXPECTED_SLOTS) {
        if (material[slot]) {
          throw new Error(`${label}: unexpected texture slot "${slot}" on ${material.name} — the web builder drifted; review the baker`);
        }
      }
      const repeats = [...new Set(Object.values(entry.textures).map((t) => t.repeat.join(",")))];
      if (repeats.length > 1) {
        throw new Error(`${label}: material ${material.name} binds slots with differing repeats (${repeats}); UV baking cannot represent that`);
      }
      if (repeats.length === 1) objectRepeat = repeats[0].split(",").map(Number);
    }
    if (objectRepeat) {
      const previous = repeatByGeometry.get(object.geometry);
      if (previous && (previous[0] !== objectRepeat[0] || previous[1] !== objectRepeat[1])) {
        throw new Error(`${label}: geometry shared by ${geometryRepeatOwner.get(object.geometry)} and ${object.name} has conflicting texture repeats`);
      }
      repeatByGeometry.set(object.geometry, objectRepeat);
      geometryRepeatOwner.set(object.geometry, object.name);
    }
  });

  // Bake repeats into UVs: Qt Quick 3D textures have no UV transform.
  for (const [geometry, [u, v]] of repeatByGeometry) {
    if (u === 1 && v === 1) continue;
    const uv = geometry.getAttribute("uv");
    if (!uv) throw new Error(`${label}: texture-bound geometry ${geometry.name || "<unnamed>"} has no UVs`);
    for (let i = 0; i < uv.count; i++) uv.setXY(i, uv.getX(i) * u, uv.getY(i) * v);
    uv.needsUpdate = true;
  }

  // Pass 2: split instances out of the graph. three's exporter would write
  // EXT_mesh_gpu_instancing, balsam drops it silently and Blender expands it,
  // so instances travel in the contract and the GLB keeps one archetype mesh
  // per group (ADR 0010). Collect first, convert after: mutating children
  // while three's traverse walks them skips and repeats nodes.
  const counts = { meshes: 0, instancedMeshes: 0, instances: 0 };
  const matrix = new THREE.Matrix4();
  const nodeMatrix = new THREE.Matrix4();
  const position = new THREE.Vector3();
  const quaternion = new THREE.Quaternion();
  const scale = new THREE.Vector3();
  const instancedMeshes = [];
  root.traverse((object) => {
    if (object.isInstancedMesh) instancedMeshes.push(object);
    else if (object.isMesh) counts.meshes++;
  });
  for (const object of instancedMeshes) {
    counts.instancedMeshes += 1;
    counts.instances += object.count;
    object.updateMatrix();
    nodeMatrix.copy(object.matrix);
    const list = [];
    for (let i = 0; i < object.count; i++) {
      object.getMatrixAt(i, matrix);
      matrix.premultiply(nodeMatrix).decompose(position, quaternion, scale);
      const record = {
        p: [round5(position.x), round5(position.y), round5(position.z)],
        q: [round5(quaternion.x), round5(quaternion.y), round5(quaternion.z), round5(quaternion.w)],
        s: [round5(scale.x), round5(scale.y), round5(scale.z)],
      };
      if (object.instanceColor) {
        record.c = [round5(object.instanceColor.getX(i)), round5(object.instanceColor.getY(i)), round5(object.instanceColor.getZ(i))];
      }
      list.push(record);
    }
    instancing[object.name] = list;
    const archetype = new THREE.Mesh(object.geometry, object.material);
    archetype.name = object.name;
    archetype.visible = object.visible;
    // Identity transform: each instance record was premultiplied by the
    // node's local matrix, so the offset already lives in the contract.
    object.parent.add(archetype);
    object.parent.remove(object);
  }

  // Pass 3: strip every texture binding before export.
  root.traverse((object) => {
    if (!object.isMesh) return;
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of materials) {
      for (const slot of TEXTURE_SLOTS) material[slot] = null;
      material.needsUpdate = true;
    }
  });

  // World bounds over the static + archetype meshes.
  const bounds = new THREE.Box3();
  root.traverse((object) => {
    if (!object.isMesh) return;
    object.updateWorldMatrix(true, false);
    bounds.union(new THREE.Box3().setFromObject(object));
  });

  let nodeCount = 0;
  root.traverse(() => nodeCount++);
  const commit = await referenceCommit();
  const contract = {
    format: 1,
    seed: SEED,
    source: {
      repo: "https://github.com/shenghaoc/rowplay",
      commit,
      authoredAgainst: AUTHORED_AGAINST,
      entry: "src/lib/replay/renderer3dEnvironment.ts",
      generator: "tools/bake-venues/bake.mjs",
    },
    sport,
    quality: tier,
    environmentDetail: cfg.environmentDetail,
    innerR: INNER_R,
    outerR: OUTER_R,
    environment: serialiseEnvironment(environment),
    materials: Object.fromEntries(contractMaterials),
    instancing,
    hidden,
    inventory: {
      nodes: nodeCount,
      meshes: counts.meshes + counts.instancedMeshes,
      instancedMeshes: counts.instancedMeshes,
      instances: counts.instances,
      materials: contractMaterials.size,
    },
    bounds: { min: [bounds.min.x, bounds.min.y, bounds.min.z].map(round5), max: [bounds.max.x, bounds.max.y, bounds.max.z].map(round5) },
  };
  root.userData = {
    venue: root.name,
    seed: SEED,
    sport,
    quality: tier,
    environmentDetail: cfg.environmentDetail,
    sourceCommit: commit,
  };

  const glb = await new GLTFExporter().parseAsync(root, { binary: true });
  if (!(glb instanceof ArrayBuffer)) throw new Error(`${label}: exporter did not produce a binary GLB`);

  // Self-check the exported graph before it leaves this process: no node may
  // be unnamed (the runtime addresses by name), no instancing extension may
  // survive (balsam drops it silently), and no image may be embedded.
  const doc = JSON.parse(readGlbJsonChunk(Buffer.from(glb)));
  const unnamed = (doc.nodes ?? []).filter((node) => !node.name);
  if (unnamed.length > 0) throw new Error(`${label}: ${unnamed.length} exported nodes are unnamed`);
  const instanced = (doc.nodes ?? []).filter((node) => node.extensions?.EXT_mesh_gpu_instancing);
  if (instanced.length > 0) throw new Error(`${label}: instancing extension survived on ${instanced.length} nodes`);
  if ((doc.images ?? []).length > 0) throw new Error(`${label}: GLB embeds ${doc.images.length} images`);
  for (const material of doc.materials ?? []) {
    if (!material.name) throw new Error(`${label}: unnamed exported material`);
  }

  const stem = `rowplay-venue-${sport}-${tier}`;
  await mkdir(join(outDir, "procedural"), { recursive: true });
  await writeFile(join(outDir, `${stem}.glb`), Buffer.from(glb));
  await writeFile(join(outDir, `${stem}.json`), JSON.stringify(contract, null, 2) + "\n");
  for (const [name, buffer] of proceduralPngs) await writeFile(join(outDir, name), buffer);

  console.log(
    `${stem}: ${glb.byteLength} B | ${contract.inventory.nodes} nodes, ${contract.inventory.meshes} meshes, ` +
      `${contract.inventory.instancedMeshes} instanced groups (${contract.inventory.instances} instances), ` +
      `${contract.inventory.materials} materials`,
  );
  return { stem, glbBytes: glb.byteLength, contract };
}

// ---------------------------------------------------------------------------
// Finalize: after the Blender pass, re-derive inventory and bounds from the
// cleaned GLBs (the artifacts are the source of truth) and refresh the
// contracts.
// ---------------------------------------------------------------------------

function readGlbJsonChunk(bytes) {
  if (bytes.readUInt32LE(0) !== 0x46546c67) throw new Error("not a GLB");
  const jsonLength = bytes.readUInt32LE(12);
  return bytes.toString("utf8", 20, 20 + jsonLength);
}

/** Bounds over POSITION accessors only (normal/tangent bounds are meaningless here). */
function computeBoundsFromAccessors(doc) {
  const positionAccessors = new Set();
  for (const mesh of doc.meshes ?? []) {
    for (const primitive of mesh.primitives ?? []) {
      if (typeof primitive.attributes?.POSITION === "number") positionAccessors.add(primitive.attributes.POSITION);
    }
  }
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (const index of positionAccessors) {
    const accessor = doc.accessors?.[index];
    if (!accessor?.min || !accessor?.max) continue;
    for (let i = 0; i < 3; i++) {
      min[i] = Math.min(min[i], accessor.min[i]);
      max[i] = Math.max(max[i], accessor.max[i]);
    }
  }
  if (!min.every(Number.isFinite)) return null;
  return { min: min.map(round5), max: max.map(round5) };
}

async function finalize(outDir) {
  for (const sport of SPORTS) {
    for (const tier of TIERS) {
      const stem = `rowplay-venue-${sport}-${tier}`;
      const glbPath = join(outDir, `${stem}.glb`);
      const contractPath = join(outDir, `${stem}.json`);
      const bytes = await readFile(glbPath);
      const doc = JSON.parse(readGlbJsonChunk(bytes));
      const contract = JSON.parse(await readFile(contractPath, "utf8"));

      const nodes = doc.nodes ?? [];
      const meshes = doc.meshes ?? [];
      const materials = doc.materials ?? [];
      const instancedNames = Object.keys(contract.instancing);

      for (const node of nodes) {
        const named = node.name;
        if (named && !named.startsWith(`environment:${sport}:`) && named !== `venue-${sport}-${tier}`) {
          throw new Error(`${stem}: node "${named}" lost its venue name prefix in the Blender pass`);
        }
      }
      const lost = instancedNames.filter((name) => !nodes.some((node) => node.name === name));
      if (lost.length > 0) throw new Error(`${stem}: the Blender pass dropped archetype nodes: ${lost.join(", ")}`);
      for (const material of materials) {
        if (!material.name) throw new Error(`${stem}: unnamed material survived the Blender pass`);
        if (!contract.materials[material.name]) throw new Error(`${stem}: material "${material.name}" is not in the contract`);
        if (material.pbrMetallicRoughness?.baseColorTexture || material.normalTexture || material.emissiveTexture) {
          throw new Error(`${stem}: material "${material.name}" embeds a texture image`);
        }
      }

      contract.inventory = {
        nodes: nodes.length,
        meshes: meshes.length,
        instancedMeshes: instancedNames.filter((name) => nodes.some((node) => node.name === name)).length,
        instances: Object.values(contract.instancing).reduce((sum, list) => sum + list.length, 0),
        materials: materials.length,
      };
      contract.bounds = computeBoundsFromAccessors(doc);
      await writeFile(contractPath, JSON.stringify(contract, null, 2) + "\n");
      console.log(`${stem}: finalized ${nodes.length} nodes, ${meshes.length} meshes, ${materials.length} materials`);
    }
  }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function runChild(...args) {
  const result = spawnSync(
    process.execPath,
    ["--experimental-transform-types", "--import", "./tools/bake-venues/register.mjs", "tools/bake-venues/bake.mjs", ...args],
    { stdio: "inherit", cwd: REPO },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`child bake failed (${args.join(" ")}): exit ${result.status}`);
}

const flagValue = (name) => {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] : null;
};
const hasFlag = (name) => process.argv.includes(`--${name}`);

const outDir = resolve(flagValue("out") ?? STAGING);
const only = flagValue("only");
const all = hasFlag("all");
const verify = hasFlag("verify") || all;

if (hasFlag("finalize")) {
  await finalize(outDir);
} else if (all && !only) {
  // One child per variant: a three.js venue build retains a lot of geometry,
  // so each variant gets a clean process and the memory is reclaimed.
  await mkdir(join(outDir, "procedural"), { recursive: true });
  for (const sport of SPORTS) for (const tier of TIERS) runChild("--only", `${sport}:${tier}`, "--out", outDir);
  if (verify) {
    const scratch = await mkdtemp(join(tmpdir(), "rowplay-venue-determinism-"));
    try {
      const [sport, tier] = [SPORTS[0], TIERS[0]];
      runChild("--only", `${sport}:${tier}`, "--out", scratch);
      const stem = `rowplay-venue-${sport}-${tier}`;
      for (const suffix of [".glb", ".json"]) {
        const a = await readFile(join(outDir, stem + suffix));
        const b = await readFile(join(scratch, stem + suffix));
        if (!a.equals(b)) throw new Error(`determinism check failed for ${stem}${suffix}`);
      }
      console.log(`determinism: ${stem} re-bakes byte-identical`);
    } finally {
      await rm(scratch, { recursive: true, force: true });
    }
  }
} else if (only) {
  const [sport, tier] = only.split(":");
  if (!SPORTS.includes(sport) || !TIERS.includes(tier)) throw new Error(`--only expects <sport>:<tier>, got "${only}"`);
  await mkdir(join(outDir, "procedural"), { recursive: true });
  await bakeVariant(sport, tier, outDir);
} else {
  console.error("usage: bake.mjs --all | --only <sport>:<tier> | --finalize [--out DIR]");
  process.exit(2);
}
