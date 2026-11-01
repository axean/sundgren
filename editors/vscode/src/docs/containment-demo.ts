// Stands in for VS Code around the extension's containment page (containment.js) on the docs
// site: hands it the results the docs build made with sundgren (containment-data.js), and does
// what the extension does for the page's controls: pick another target, count comments or not,
// inspect a branch (on the inspect demo).

import type { Comments, ToContainmentPage } from "../containment.js";
import { parseFromPage, type Control } from "../messages.js";
import {
  demoContainment,
  followTheme,
  picker,
  post,
  standIn,
  type DemoContainment,
} from "./host.js";

function send(message: ToContainmentPage): void {
  post(message);
}

function start(demo: DemoContainment): void {
  const query = new URLSearchParams(location.search);
  const asked = query.get("target");
  const state: { target: string; comments: Comments } = {
    target: asked !== null && demo.targets.has(asked) ? asked : demo.target,
    comments: query.get("comments") === "counted" ? "counted" : "ignored",
  };

  // What the extension sends for a comparison: the branches, then each result.
  const show = (): void => {
    const modes = demo.targets.get(state.target);
    const results = (state.comments === "counted" ? modes?.counted : modes?.ignored) ?? [];
    send({
      type: "start",
      target: state.target,
      base: demo.base,
      branches: results.map((entry) => entry.branch),
      comments: state.comments,
      threshold: 0.9,
    });
    for (const entry of results) {
      send({ type: "result", branch: entry.branch, result: entry.result });
    }
    send({ type: "done" });
  };

  const choose = (control: Control): void => {
    if (control === "comments") {
      state.comments = state.comments === "ignored" ? "counted" : "ignored";
      show();
    } else if (control === "target") {
      picker("Target: compare every branch with this one", demo.names, state.target, (target) => {
        state.target = target;
        show();
      });
    } else {
      picker(`Base: this demo has only ${demo.base}`, [demo.base], demo.base, () => {
        // The only one.
      });
    }
  };

  // The inspect page, in place of this one; it links back with the same target and comments.
  const inspect = (branch: string, path: string | undefined, line: number | undefined): void => {
    const next = new URLSearchParams({ ...state, branch });
    if (path !== undefined) {
      next.set("path", path);
    }
    if (line !== undefined) {
      next.set("line", String(line));
    }
    location.assign(`inspect.html?${next.toString()}`);
  };

  standIn((value) => {
    const message = parseFromPage(value);
    switch (message?.type) {
      case "ready": {
        show();
        break;
      }
      case "choose": {
        choose(message.control);
        break;
      }
      case "inspect": {
        inspect(message.branch, message.path, message.line);
        break;
      }
      default:
      // Nothing else to do here.
    }
  });
}

followTheme();
const demo = demoContainment();
if (demo !== undefined) {
  start(demo);
}
