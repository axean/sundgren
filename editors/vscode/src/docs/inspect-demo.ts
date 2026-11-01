// Stands in for VS Code around the extension's inspect page (inspect.js) on the docs site: hands
// it one branch's result from the demo data (containment-data.js), as chosen on the containment
// demo, and links back to it. Files at a commit can't be opened here: there is no repository.

import type { ToInspectPage } from "../inspect.js";
import { parseFromPage } from "../messages.js";
import { demoContainment, followTheme, post, standIn, type DemoContainment } from "./host.js";

function send(message: ToInspectPage): void {
  post(message);
}

function start(demo: DemoContainment): void {
  const query = new URLSearchParams(location.search);
  const asked = query.get("target");
  const target = asked !== null && demo.targets.has(asked) ? asked : demo.target;
  const comments = query.get("comments") === "counted" ? "counted" : "ignored";
  const modes = demo.targets.get(target);
  const results = (comments === "counted" ? modes?.counted : modes?.ignored) ?? [];
  const entry = results.find((candidate) => candidate.branch === query.get("branch")) ?? results[0];
  const path = query.get("path");
  const line = Number(query.get("line"));

  const back = document.querySelector<HTMLAnchorElement>(".sg-back");
  if (back !== null) {
    back.href = `containment.html?${new URLSearchParams({ target, comments }).toString()}`;
  }

  standIn((value) => {
    if (entry === undefined || parseFromPage(value)?.type !== "ready") {
      return;
    }
    const focus =
      path === null ? undefined : { path, ...(Number.isSafeInteger(line) && line > 0 && { line }) };
    send({
      type: "inspect",
      branch: entry.branch,
      target,
      base: demo.base,
      comments,
      threshold: 0.9,
      result: entry.result,
      ...(focus !== undefined && { focus }),
    });
  });
}

followTheme();
const demo = demoContainment();
if (demo !== undefined) {
  start(demo);
}
