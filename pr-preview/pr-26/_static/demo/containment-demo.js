// Stands in for VS Code around the extension's containment page (containment.js): hands it the
// demo results (containment-data.js) and follows the docs site's light/dark theme (demo.js).
"use strict";

window.acquireVsCodeApi = () => ({
  postMessage(message) {
    if (message.type !== "ready") {
      return;
    }
    const demo = window.sundgrenDemoContainment;
    const post = (data) => window.postMessage(data, window.origin);
    post({
      type: "start",
      target: demo.target,
      base: demo.base,
      branches: demo.results.map((entry) => entry.branch),
    });
    for (const entry of demo.results) {
      post({ type: "result", branch: entry.branch, result: entry.result });
    }
    post({ type: "done" });
  },
});
