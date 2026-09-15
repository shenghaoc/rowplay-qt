// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Resolve hooks for tools/gen-rig-phase-parity.mjs.
 *
 * Same bridges as the venue baker's hooks (ADR 0010) — `three` from the
 * reference checkout's node_modules, extensionless relative imports retried
 * with TypeScript suffixes — plus a stub for SvelteKit's `$app/paths` virtual
 * module, which `renderer3dAssets.ts` imports for its asset base path. The
 * web app is deployed at the root, so `base` is the empty string.
 *
 * The retries must be `return await`: a bare `return` of the promise leaves
 * the `try` block before the rejection, so a failed `.ts` retry would escape
 * the `catch` instead of falling through to `.js` (the bike rig modules are
 * the only `.js`-suffixed imports in the graph).
 */
import { pathToFileURL, fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const REFERENCE_NODE_MODULES = join(here, "..", "reference", "rowplay", "node_modules");

const APP_STUBS = {
  "$app/paths":
    "export const base = '';\nexport const assets = '';\nexport const prerendering = false;\n",
};

export async function resolve(specifier, context, nextResolve) {
  if (APP_STUBS[specifier]) {
    return {
      url: `data:text/javascript;base64,${Buffer.from(APP_STUBS[specifier]).toString("base64")}`,
      shortCircuit: true,
      format: "module",
    };
  }
  try {
    return await nextResolve(specifier, context);
  } catch (error) {
    if (specifier === "three") {
      return await nextResolve(
        pathToFileURL(join(REFERENCE_NODE_MODULES, "three/build/three.module.js")).href,
        context,
      );
    }
    if (specifier.startsWith("three/")) {
      return await nextResolve(
        pathToFileURL(join(REFERENCE_NODE_MODULES, specifier)).href,
        context,
      );
    }
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const suffix of [".ts", ".js", "/index.ts", "/index.js"]) {
        try {
          return await nextResolve(specifier + suffix, context);
        } catch {}
      }
    }
    throw error;
  }
}
