// Stands in for VS Code around the extension's history page (webview.js) on the docs site. It
// can't run sundgren, so it answers with views made in advance by the docs build (graph.js): each
// Show mode's pages, its runs of days, and the default mode's ticks. When it has none for exactly
// what was asked, it shows the closest and says so. Base and Like open a stand-in picker with the
// branches it has views of.

import { parseGraph } from "../graph.js";
import { parseFromPage, type Control, type ToPage, type View } from "../messages.js";
import { demoGraphs, followTheme, picker, post, standIn, type DemoGraphs } from "./host.js";
import { closest, modeOf } from "./history-views.js";

const BASE = "main";
/** How long sundgren "runs": long enough for the page to show it is reading. */
const RUN_MS = 150;

function send(message: ToPage): void {
  post(message);
}

/** A note when the demo shows the closest view it has, not the one asked for. */
function note(isExact: boolean): void {
  const shown = document.querySelector(".sg-demo-note");
  if (isExact) {
    shown?.remove();
    return;
  }
  if (shown !== null) {
    return;
  }
  const node = document.createElement("div");
  node.className = "sg-demo-note";
  node.textContent =
    "The demo has views made in advance, and none for exactly this: it shows the closest. " +
    "In VS Code, sundgren draws what you ask for.";
  document.body.append(node);
}

function start(demo: DemoGraphs): void {
  let current: View = {
    show: "local",
    branches: 12,
    commits: demo.page,
    pins: [],
    hides: [],
  };

  /** The graph for `view`, posted as sundgren's output would be; and the note if it isn't exact. */
  const answer = (view: View): void => {
    const found = closest(demo.graphs, demo.days, view);
    const graph = parseGraph(found.view?.graph);
    if (graph === undefined) {
      send({ type: "error", message: "The demo has no view for this." });
      return;
    }
    send({ type: "graph", graph });
    note(found.isExact);
  };

  /** Run `view`, as the extension does: say it is running, then post its graph. */
  const run = (view: View): void => {
    current = view;
    send({ type: "running", view });
    setTimeout(() => {
      answer(view);
    }, RUN_MS);
  };

  const likes = [
    ...new Set(
      demo.graphs.filter((made) => modeOf(made) === "like").map((made) => made.show[1] ?? ""),
    ),
  ];
  const choose = (control: Control): void => {
    if (control === "base") {
      picker(
        `Base: the reference line (now ${BASE})`,
        [BASE],
        BASE,
        () => {
          run(current);
        },
        "The demo has views of one base; in VS Code, any branch.",
      );
    } else if (control === "like") {
      picker(
        "Like: show the branches whose work is most found in this one",
        likes,
        current.like,
        (like) => {
          run({ ...current, show: "like", like });
        },
        "The demo ranks by these; in VS Code, any branch.",
      );
    }
  };

  standIn((value) => {
    const message = parseFromPage(value);
    switch (message?.type) {
      case "ready": {
        send({ type: "setup", base: BASE, view: current, step: demo.page, states: [] });
        answer(current);
        break;
      }
      case "view": {
        const { show, like, branches, commits, pins, hides, since, until } = message;
        run({
          show,
          ...(like !== undefined && { like }),
          branches,
          commits,
          pins,
          hides,
          ...(since !== undefined && { since }),
          ...(until !== undefined && { until }),
        });
        break;
      }
      case "choose": {
        choose(message.control);
        break;
      }
      default:
      // Clicks and "rendered": nothing to do.
    }
  });
}

followTheme();
const demo = demoGraphs();
if (demo !== undefined) {
  start(demo);
}
