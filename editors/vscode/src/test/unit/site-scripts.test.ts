import * as assert from "node:assert/strict";

import { label, parseEnvironments } from "../../docs/environments.js";
import type { DemoGraph } from "../../docs/host.js";
import { alike, closest, daysKey } from "../../docs/history-views.js";
import type { View } from "../../messages.js";

const DAY = 86_400;
const DAYS = [10 * DAY, 11 * DAY, 12 * DAY];

function made(fields: Partial<DemoGraph>): DemoGraph {
  return { show: ["--show", "local"], commits: 10, hides: [], graph: fields, ...fields };
}

const VIEW: View = { show: "local", branches: 12, commits: 10, pins: [], hides: [] };

describe("history demo views", () => {
  it("compares hidden branches", () => {
    assert.equal(alike([], []), 1);
    assert.equal(alike(["a"], ["a"]), 1);
    assert.equal(alike(["a", "b"], ["a"]), 0.5);
    assert.equal(alike(["a"], ["b"]), 0);
  });

  it("keys dates by the days with commits they cover", () => {
    assert.equal(daysKey(DAYS, undefined, undefined), "any");
    assert.equal(daysKey(DAYS, 11 * DAY + 5, 11 * DAY + 10), String(11 * DAY));
    assert.equal(
      daysKey(DAYS, 10 * DAY, 12 * DAY),
      `${String(10 * DAY)},${String(11 * DAY)},${String(12 * DAY)}`,
    );
    assert.equal(daysKey(DAYS, 0, 5 * DAY), "before");
    assert.equal(daysKey(DAYS, 20 * DAY, 25 * DAY), "after");
  });

  it("finds the exact view, else the closest", () => {
    const pages = [made({ commits: 10 }), made({ commits: 20 })];
    const hidden = made({ hides: ["a"] });
    const none = made({ hides: ["a", "b"] });
    const empty = made({ dates: [20 * DAY, 21 * DAY] });
    const graphs = [...pages, hidden, none, empty];

    assert.deepEqual(closest(graphs, DAYS, { ...VIEW, commits: 20 }), {
      view: pages[1],
      isExact: true,
    });
    assert.deepEqual(closest(graphs, DAYS, { ...VIEW, hides: ["b", "a"] }), {
      view: none,
      isExact: true,
    });
    // Any empty interval after the history is that view.
    assert.deepEqual(closest(graphs, DAYS, { ...VIEW, since: 30 * DAY, until: 31 * DAY }), {
      view: empty,
      isExact: true,
    });
    // One branch hidden, but on page 2: the nearest, not exact.
    const found = closest(graphs, DAYS, { ...VIEW, commits: 20, hides: ["a"] });
    assert.equal(found.view, hidden);
    assert.equal(found.isExact, false);
  });
});

describe("environments", () => {
  it("reads what it can of environments.json", () => {
    const environments = parseEnvironments({
      environments: [
        { kind: "release", path: "", title: "v1.1.1" },
        {
          kind: "pr",
          path: "pr-preview/pr-31/",
          number: 31,
          title: "History",
          ref: "hv",
          sha: "956377b1234",
        },
        { kind: "tag", path: "x/" },
        "nonsense",
      ],
    });
    assert.equal(environments.length, 2);
    assert.deepEqual(
      environments.map((environment) => label(environment)),
      [
        { title: "Live site", summary: "Release v1.1.1" },
        { title: "#31 History", summary: "Pull request preview: hv 956377b" },
      ],
    );
    assert.deepEqual(parseEnvironments({ environments: "none" }), []);
    assert.deepEqual(parseEnvironments(undefined), []);
  });
});
