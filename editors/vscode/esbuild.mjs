// Bundles the extension into dist/extension.js (Node), and the history, containment, inspect and
// outline pages into dist/webview.js, containment.js, inspect.js and outline.js (browser), as in
// Microsoft's "Bundling Extensions" guide. Also the docs site's scripts (src/docs/) into
// dist/docs/, which the docs build copies. `--production` minifies and drops the source maps (used
// by `vscode:prepublish`).
import { execFileSync } from "node:child_process";
import { accessSync, constants, cpSync, rmSync } from "node:fs";
import path from "node:path";

import * as esbuild from "esbuild";

const production = process.argv.includes("--production");

/** Whether `file` is an executable. */
function isExecutable(file) {
  try {
    accessSync(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** The git on PATH, as an absolute path; undefined if there is none. */
function findGit() {
  const name = process.platform === "win32" ? "git.exe" : "git";
  return (process.env["PATH"] ?? "")
    .split(path.delimiter)
    .filter((directory) => path.isAbsolute(directory))
    .map((directory) => path.join(directory, name))
    .find((file) => isExecutable(file));
}

/** The short commit id of HEAD, with "+" if the work tree has changes; "unknown" without git. */
function commit() {
  const git = findGit();
  if (git === undefined) {
    return "unknown";
  }
  try {
    const run = (gitArguments) => execFileSync(git, gitArguments, { encoding: "utf8" }).trim();
    return run(["rev-parse", "--short", "HEAD"]) + (run(["status", "--porcelain"]) ? "+" : "");
  } catch {
    return "unknown";
  }
}

const COMMIT = JSON.stringify(commit());

// The pages' styles and fonts live in the repository's style/ folder, shared with the docs site;
// the extension ships a copy in media/.
const STYLE = path.join(import.meta.dirname, "..", "..", "style");
const MEDIA = path.join(import.meta.dirname, "media");
rmSync(MEDIA, { recursive: true, force: true });
// site.css and demo.css are the docs site's own; the README is about the folder.
const SITE_ONLY = new Set(["site.css", "demo.css", "README.md"]);
cpSync(STYLE, MEDIA, { recursive: true, filter: (file) => !SITE_ONLY.has(path.basename(file)) });
const common = {
  bundle: true,
  minify: production,
  sourcemap: !production,
  sourcesContent: false,
  logLevel: "warning",
};

// The docs site's scripts: the demos' stand-ins for VS Code, and the Development menu.
const DOCS = ["history-demo", "containment-demo", "inspect-demo", "environments"];

await Promise.all([
  ...DOCS.map((name) =>
    esbuild.build({
      ...common,
      // The site doesn't serve source maps.
      sourcemap: false,
      entryPoints: [`src/docs/${name}.ts`],
      format: "iife",
      platform: "browser",
      target: "es2022",
      outfile: `dist/docs/${name}.js`,
    }),
  ),
  esbuild.build({
    ...common,
    entryPoints: ["src/extension.ts"],
    format: "cjs",
    platform: "node",
    target: "node22",
    outfile: "dist/extension.js",
    external: ["vscode"],
    // The commit the extension is built from, for the panels' footer.
    define: { SUNDGREN_COMMIT: COMMIT },
  }),
  esbuild.build({
    ...common,
    entryPoints: ["src/webview/containment.ts"],
    format: "iife",
    platform: "browser",
    target: "es2022",
    outfile: "dist/containment.js",
  }),
  esbuild.build({
    ...common,
    entryPoints: ["src/webview/inspect.ts"],
    format: "iife",
    platform: "browser",
    target: "es2022",
    outfile: "dist/inspect.js",
  }),
  esbuild.build({
    ...common,
    entryPoints: ["src/webview/outline.ts"],
    format: "iife",
    platform: "browser",
    target: "es2022",
    outfile: "dist/outline.js",
  }),
  esbuild.build({
    ...common,
    entryPoints: ["src/webview/main.ts"],
    format: "iife",
    platform: "browser",
    target: "es2022",
    outfile: "dist/webview.js",
  }),
]);
