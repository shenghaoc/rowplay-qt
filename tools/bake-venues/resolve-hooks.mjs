// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Resolve hooks for the venue baker (ADR 0010).
 *
 * The web builder sources import each other extensionless (TypeScript
 * convention), which Node ESM cannot resolve, and the baker runs from this
 * repository where `three` is not installed. These hooks bridge both gaps:
 *
 *  - bare `three` / `three/...` specifiers fall back to the reference
 *    checkout's node_modules (installed there by `pnpm install`);
 *  - extensionless relative imports retry with `.ts` / `.js` / `/index.ts`
 *    appended, mirroring TypeScript's Node-ish resolution.
 */
import { pathToFileURL } from "node:url";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const REFERENCE_NODE_MODULES = join(here, "..", "..", "reference", "rowplay", "node_modules");

export async function resolve(specifier, context, nextResolve) {
  try {
    return await nextResolve(specifier, context);
  } catch (error) {
    if (specifier === "three") {
      return nextResolve(pathToFileURL(join(REFERENCE_NODE_MODULES, "three/build/three.module.js")).href, context);
    }
    if (specifier.startsWith("three/")) {
      return nextResolve(pathToFileURL(join(REFERENCE_NODE_MODULES, specifier)).href, context);
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
