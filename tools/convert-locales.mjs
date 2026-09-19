// SPDX-License-Identifier: GPL-3.0-or-later
// Converts the rowplay web locales into ID-based Qt Linguist (.ts) files.
//
// The web dictionaries (`reference/rowplay/src/lib/locales/{en,zh,de,es,fr,ja}.ts`)
// are nested key objects with `{name}` placeholders and per-key English
// fallback. Each leaf becomes one ID-based Qt message: the id is the dotted
// web key (`dashboard.title`), <source> stays EMPTY (the canonical lupdate
// -idbased shape — lrelease only embeds the id lookup when the source is
// empty, verified against Qt 6.11.2), the English text goes to <oldsource>
// for translator context, and <translation> is the locale text — filled with
// the English value when the locale lacks the key, exactly like the web
// fallback. QML resolves them with
// `qsTrId(id)` through the `Tr.t(id, vars)` helper (no numerus forms: the web
// has no plural rules).
//
// Usage (Node >= 23.6, or >= 22.6 with --experimental-strip-types):
//   node tools/convert-locales.mjs                 # regenerate i18n/rowplay_*.ts
//   node tools/convert-locales.mjs --check         # fail unless committed files match
//   node tools/convert-locales.mjs --locales DIR   # non-default web checkout
//
// The generated .ts files are committed; CI has no `reference/` checkout, so
// --check only runs where the web repo is present (the parity test in
// crates/rowplay-viewmodel skips regeneration the same way).

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");

// The catalogues must be reproducible from the web commit pinned in
// docs/source-map.md. Same guard as tools/gen-ski-arm-hand-window.mjs:
// a stale reference/rowplay checkout silently regenerates reverted
// strings (observed: the web's restored fr dashboard.emptyTrend
// placeholder came back dropped).
const PINNED_COMMIT = "173c6facbcedef419ad39168c5e3e642abb7e57e";

const LANGUAGES = ["en", "zh", "de", "es", "fr", "ja"];

function parseArgs(argv) {
  const args = { check: false, locales: null, out: join(REPO, "i18n") };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--check") {
      args.check = true;
    } else if (arg === "--locales") {
      args.locales = resolve(argv[(i += 1)]);
    } else if (arg === "--out") {
      args.out = resolve(argv[(i += 1)]);
    } else {
      console.error(`unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  args.locales ??= join(REPO, "reference", "rowplay", "src", "lib", "locales");
  return args;
}

/** Flattens a nested dictionary to dotted keys, preserving insertion order. */
function flatten(dict, prefix = "", out = new Map()) {
  for (const [key, value] of Object.entries(dict)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") {
      out.set(path, value);
    } else if (value && typeof value === "object") {
      flatten(value, path, out);
    } else {
      throw new Error(`non-string leaf at ${path}: ${typeof value}`);
    }
  }
  return out;
}

function escapeXml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/** Builds one Qt TS 2.1 document for `language` against the English keys. */
function buildTs(language, english, locale) {
  const lines = [
    '<?xml version="1.0" encoding="utf-8"?>',
    "<!DOCTYPE TS>",
    `<TS version="2.1" language="${language}">`,
    "<context>",
    "    <name></name>",
  ];
  let filled = 0;
  for (const [id, source] of english) {
    let translation = locale.get(id);
    if (translation === undefined) {
      // Web parity: missing keys fall back to the English value.
      translation = source;
      filled += 1;
    }
    lines.push(`    <message id="${escapeXml(id)}">`);
    lines.push(`        <source></source>`);
    lines.push(`        <oldsource>${escapeXml(source)}</oldsource>`);
    lines.push(`        <translation>${escapeXml(translation)}</translation>`);
    lines.push("    </message>");
  }
  lines.push("</context>", "</TS>", "");
  if (filled > 0) {
    console.warn(`${language}: filled ${filled} missing key(s) with English`);
  }
  return lines.join("\n");
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!existsSync(args.locales)) {
    console.error(
      `locale directory not found: ${args.locales}\n` +
        "check out the rowplay web app into reference/ (see AGENTS.md) or pass --locales",
    );
    process.exit(2);
  }

  // Refuse to read the default reference checkout at any commit but the
  // pin; an explicit --locales is a deliberate override that skips the
  // guard — loudly, so an override in a script or alias cannot pass
  // silently for a stale checkout.
  const reference = join(REPO, "reference", "rowplay");
  const defaultLocales = join(reference, "src", "lib", "locales");
  if (args.locales === defaultLocales) {
    let head;
    try {
      head = execFileSync("git", ["-C", reference, "rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim();
    } catch {
      head = "(not a git checkout)";
    }
    if (head !== PINNED_COMMIT) {
      console.error(
        `reference/rowplay is at ${head}, expected the pinned ${PINNED_COMMIT}.\n` +
          `Check out the pinned commit (docs/source-map.md) or update the pin in this script.`,
      );
      process.exit(1);
    }
  } else {
    console.error(
      `note: --locales ${args.locales} given; skipping the pinned-reference guard` +
        ` (expected ${defaultLocales} at ${PINNED_COMMIT} per docs/source-map.md).`,
    );
  }

  const dictionaries = new Map();
  for (const language of LANGUAGES) {
    const file = join(args.locales, `${language}.ts`);
    const module = await import(pathToFileURL(file).href);
    const dict = module[language];
    if (!dict) {
      console.error(`${file}: no named export \`${language}\``);
      process.exit(2);
    }
    dictionaries.set(language, flatten(dict));
  }

  // Desktop-only keys: the web has no locale key for these, so the desktop
  // port supplies the English value and every locale inherits it (the web's
  // fallback rule). Kept as few as possible — only keys with no web
  // equivalent that appear in committed QML.
  const DESKTOP_SUPPLEMENT = new Map([
    ["settings.reduceMotion", "Reduce motion"],
  ]);
  for (const [id, value] of DESKTOP_SUPPLEMENT) {
    for (const [, dict] of dictionaries) {
      dict.set(id, dict === dictionaries.get("en") ? value : value);
    }
  }

  const english = dictionaries.get("en");
  for (const [language, dict] of dictionaries) {
    if (language === "en") continue;
    for (const id of dict.keys()) {
      if (!english.has(id)) {
        console.error(`${language}: key not present in en: ${id}`);
        process.exit(1);
      }
    }
  }

  const generated = new Map();
  for (const language of LANGUAGES) {
    generated.set(
      language,
      buildTs(language, english, dictionaries.get(language)),
    );
  }

  if (args.check) {
    let failed = false;
    for (const [language, text] of generated) {
      const path = join(args.out, `rowplay_${language}.ts`);
      if (!existsSync(path)) {
        console.error(`missing committed file: ${path}`);
        failed = true;
        continue;
      }
      if (readFileSync(path, "utf8") !== text) {
        console.error(`stale committed file: ${path} (rerun without --check)`);
        failed = true;
      }
    }
    const committed = existsSync(args.out)
      ? readdirSync(args.out).filter((f) => /^rowplay_.*\.ts$/.test(f))
      : [];
    for (const file of committed) {
      const language = file.slice("rowplay_".length, -".ts".length);
      if (!generated.has(language)) {
        console.error(`unexpected committed file: ${join(args.out, file)}`);
        failed = true;
      }
    }
    if (failed) process.exit(1);
    console.log(`--check ok: ${generated.size} locales, ${english.size} ids`);
    return;
  }

  mkdirSync(args.out, { recursive: true });
  for (const [language, text] of generated) {
    writeFileSync(join(args.out, `rowplay_${language}.ts`), text, "utf8");
  }
  console.log(
    `wrote ${generated.size} .ts files (${english.size} ids each) to ${args.out}`,
  );
}

await main();
