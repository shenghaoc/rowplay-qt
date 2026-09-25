// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Resolve hook for tools/gen-venue-shadow-parity.mjs: the web's modules get
 * a `three` whose `WebGLRenderer` is a stand-in, so the production
 * `CourseRenderer3D` builds its scene in Node without a GL context.
 *
 * The stand-in is the one the web's own renderer3d.test.ts mocks in ("Mock
 * only WebGLRenderer — everything else in Three.js works headlessly in
 * Node"): the same members, doing nothing. Every other export is the real
 * `three` the reference checkout installed, the one instance the whole
 * import graph shares, so the scene's objects are real three.js objects.
 *
 * Only imports from the web's own sources are redirected; `three`'s addons
 * and this repository's tools keep the real module. Registered after
 * tools/gen-rig-phase-resolve.mjs, so it runs first and hands every other
 * specifier on to that hook.
 */
import { realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
// Real path: Node resolves modules through symlinks, and a worktree's
// `reference` is usually a link to the primary checkout's.
const REFERENCE_SOURCES =
  pathToFileURL(realpathSync(join(here, "..", "reference", "rowplay", "src"))).href + "/";

/** renderer3d.test.ts `FakeWebGLRenderer`, member for member. */
const STAND_IN = `
export class WebGLRenderer {
  outputColorSpace = "";
  shadowMap = { enabled: false, type: 0 };
  setPixelRatio() {}
  setSize() {}
  render() {}
  getContext() {
    return { getExtension: () => ({ loseContext() {} }) };
  }
  dispose() {}
}
`;

export async function resolve(specifier, context, nextResolve) {
  if (specifier !== "three" || !context.parentURL?.startsWith(REFERENCE_SOURCES)) {
    return nextResolve(specifier, context);
  }
  const real = await nextResolve(specifier, context);
  const source = `export * from ${JSON.stringify(real.url)};\n${STAND_IN}`;
  return {
    url: `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`,
    format: "module",
    shortCircuit: true,
  };
}
