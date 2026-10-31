// Stands in for VS Code around the extension's inspect page (inspect.js): hands it one branch's
// result from the demo data (containment-data.js), as chosen on the containment demo, and links
// back to it. Files at a commit can't be opened here: there is no repository behind the demo.
"use strict";

(() => {
  const demo = window.sundgrenDemoContainment;
  const query = new URLSearchParams(window.location.search);
  const target =
    demo.targets[query.get("target")] === undefined
      ? demo.target
      : query.get("target");
  const comments = query.get("comments") === "counted" ? "counted" : "ignored";
  const results = demo.targets[target][comments];
  const entry =
    results.find((candidate) => candidate.branch === query.get("branch")) ??
    results[0];
  const path = query.get("path");
  const line = Number(query.get("line"));

  const back = document.querySelector(".sg-back");
  if (back !== null) {
    back.href = `containment.html?${new URLSearchParams({ target, comments }).toString()}`;
  }

  window.acquireVsCodeApi = () => ({
    postMessage(message) {
      if (message.type !== "ready") {
        return;
      }
      const focus =
        path === null
          ? undefined
          : { path, ...(Number.isInteger(line) && line > 0 && { line }) };
      window.postMessage(
        {
          type: "inspect",
          branch: entry.branch,
          target,
          base: demo.base,
          comments,
          threshold: 0.9,
          result: entry.result,
          ...(focus !== undefined && { focus }),
        },
        window.origin,
      );
    },
  });
})();
