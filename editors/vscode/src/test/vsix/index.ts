// Runs bundled.test.ts inside VS Code (the entry point run.ts hands to @vscode/test-electron).
import path from "node:path";

import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "bdd", timeout: 120_000, color: true });
  mocha.addFile(path.join(__dirname, "bundled.test.js"));
  return new Promise((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${String(failures)} test(s) failed`));
      } else {
        resolve();
      }
    });
  });
}
