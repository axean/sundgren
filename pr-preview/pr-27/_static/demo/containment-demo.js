// Stands in for VS Code around the extension's containment page (containment.js): hands it the
// demo results (containment-data.js, made by the docs build with sundgren) and does what the
// extension does for the page's controls: pick another target, count comments or not, inspect a
// branch. demo.js follows the docs site's light/dark theme.
"use strict";

(() => {
  const demo = window.sundgrenDemoContainment;
  const query = new URLSearchParams(window.location.search);
  const state = {
    target:
      demo.targets[query.get("target")] === undefined
        ? demo.target
        : query.get("target"),
    comments: query.get("comments") === "counted" ? "counted" : "ignored",
  };
  const post = (data) => window.postMessage(data, window.origin);

  // What the extension sends for a comparison: the branches, then each result.
  function show() {
    const results = demo.targets[state.target][state.comments];
    post({
      type: "start",
      target: state.target,
      base: demo.base,
      branches: results.map((entry) => entry.branch),
      comments: state.comments,
      threshold: 0.9,
    });
    for (const entry of results) {
      post({ type: "result", branch: entry.branch, result: entry.result });
    }
    post({ type: "done" });
  }

  // A list to pick from, like VS Code's quick pick; `pick` gets the choice.
  function picker(title, options, current, pick) {
    const backdrop = document.createElement("div");
    backdrop.className = "sg-picker-backdrop";
    const box = document.createElement("div");
    box.className = "sg-picker";
    const heading = document.createElement("p");
    heading.className = "sg-picker-title";
    heading.textContent = title;
    box.append(heading);
    const close = () => backdrop.remove();
    for (const option of options) {
      const button = document.createElement("button");
      button.type = "button";
      button.className =
        option === current ? "sg-picker-option current" : "sg-picker-option";
      button.textContent = option;
      button.addEventListener("click", () => {
        close();
        pick(option);
      });
      box.append(button);
    }
    backdrop.addEventListener("click", (event) => {
      if (event.target === backdrop) {
        close();
      }
    });
    backdrop.append(box);
    document.body.append(backdrop);
  }

  function choose(control) {
    if (control === "comments") {
      state.comments = state.comments === "ignored" ? "counted" : "ignored";
      show();
    } else if (control === "target") {
      picker(
        "Target: compare every branch with this one",
        Object.keys(demo.targets),
        state.target,
        (target) => {
          state.target = target;
          show();
        },
      );
    } else {
      picker(
        `Base: this demo has only ${demo.base}`,
        [demo.base],
        demo.base,
        () => {},
      );
    }
  }

  // The inspect page, in place of this one; it links back with the same target and comments.
  function inspect(message) {
    const query = new URLSearchParams({ ...state, branch: message.branch });
    if (message.path !== undefined) {
      query.set("path", message.path);
    }
    if (message.line !== undefined) {
      query.set("line", String(message.line));
    }
    window.location.href = `inspect.html?${query.toString()}`;
  }

  window.acquireVsCodeApi = () => ({
    postMessage(message) {
      if (message.type === "ready") {
        show();
      } else if (message.type === "choose") {
        choose(message.control);
      } else if (message.type === "inspect") {
        inspect(message);
      }
    },
  });
})();
