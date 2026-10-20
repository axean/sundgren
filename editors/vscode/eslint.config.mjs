import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import tseslint from "typescript-eslint";
import unicorn from "eslint-plugin-unicorn";
import sonarjs from "eslint-plugin-sonarjs";
import security from "eslint-plugin-security";
import globals from "globals";

export default defineConfig(
  { ignores: ["dist/", "out/", "bundled/", ".vscode-test/", "node_modules/"] },
  js.configs.recommended,
  tseslint.configs.strictTypeChecked,
  tseslint.configs.stylisticTypeChecked,
  unicorn.configs.recommended,
  sonarjs.configs.recommended,
  security.configs.recommended,
  {
    languageOptions: {
      globals: globals.node,
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      eqeqeq: "error",
      // The extension and its tests are CommonJS (VS Code loads extensions with require()).
      "unicorn/prefer-module": "off",
      "unicorn/prefer-top-level-await": "off",
    },
  },
  {
    // Config and build scripts: plain JS, outside the TS project.
    files: ["**/*.mjs"],
    extends: [tseslint.configs.disableTypeChecked],
    rules: {
      // `export default defineConfig(...)` is how these files work.
      "unicorn/no-top-level-side-effects": "off",
    },
  },
);
