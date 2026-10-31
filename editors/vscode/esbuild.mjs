// Bundles the extension into dist/extension.js (Node), and the history, containment and outline
// pages into dist/webview.js, dist/containment.js and dist/outline.js (browser), as in Microsoft's "Bundling Extensions" guide. `--production` minifies and drops the
// source maps (used by `vscode:prepublish`).
import * as esbuild from "esbuild";

const production = process.argv.includes("--production");
const common = {
  bundle: true,
  minify: production,
  sourcemap: !production,
  sourcesContent: false,
  logLevel: "warning",
};

await Promise.all([
  esbuild.build({
    ...common,
    entryPoints: ["src/extension.ts"],
    format: "cjs",
    platform: "node",
    target: "node22",
    outfile: "dist/extension.js",
    external: ["vscode"],
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
