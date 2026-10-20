// Bundles the extension into dist/extension.js, as in Microsoft's "Bundling Extensions" guide.
// `--production` minifies and drops the source map (used by `vscode:prepublish`).
import * as esbuild from "esbuild";

const production = process.argv.includes("--production");

await esbuild.build({
  entryPoints: ["src/extension.ts"],
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node22",
  outfile: "dist/extension.js",
  external: ["vscode"],
  minify: production,
  sourcemap: !production,
  sourcesContent: false,
  logLevel: "warning",
});
