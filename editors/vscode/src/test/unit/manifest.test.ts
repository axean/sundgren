// Policy: everything sundgren adds to VS Code's own UI says "sundgren:", and each place has a
// `sundgren.show.*` setting that hides it.
import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

interface Manifest {
  readonly contributes: {
    readonly commands: readonly { readonly command: string; readonly title: string }[];
    readonly menus: Record<string, readonly { readonly command: string; readonly when?: string }[]>;
    readonly configuration: {
      readonly properties: Record<string, { readonly default?: unknown }>;
    };
  };
}

const MANIFEST = JSON.parse(
  readFileSync(path.join(__dirname, "../../../package.json"), "utf8"),
) as Manifest;

describe("package.json", () => {
  it("gives every menu entry a sundgren.show setting", () => {
    for (const [menu, entries] of Object.entries(MANIFEST.contributes.menus)) {
      if (menu === "commandPalette") {
        continue;
      }
      for (const entry of entries) {
        const setting = /config\.(sundgren\.show\.\w+)/u.exec(entry.when ?? "")?.[1];
        assert.ok(setting, `${menu}: ${entry.command} has no sundgren.show setting`);
        assert.ok(Object.hasOwn(MANIFEST.contributes.configuration.properties, setting), setting);
      }
    }
  });

  it("has every sundgren.show setting on by default", () => {
    const shows = Object.entries(MANIFEST.contributes.configuration.properties).filter(([name]) =>
      name.startsWith("sundgren.show."),
    );
    assert.ok(shows.length > 0);
    for (const [name, property] of shows) {
      assert.equal(property.default, true, name);
    }
  });

  it("titles every command sundgren:", () => {
    for (const { command, title } of MANIFEST.contributes.commands) {
      assert.ok(title.startsWith("sundgren: "), command);
    }
  });
});
