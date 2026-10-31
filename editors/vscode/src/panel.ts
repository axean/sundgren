// The history webview panel: one at a time, showing the last graph it was given.

import { randomBytes } from "node:crypto";

import * as vscode from "vscode";

import type { Graph } from "./graph.js";
import { parseFromPage, type ToPage } from "./messages.js";

const VIEW_TYPE = "sundgren.history";
/** How long `show` waits for the page to draw the graph. */
const RENDER_TIMEOUT_MS = 15_000;

/** A page: the bundled script and stylesheets, nothing else may load. */
export function pageHtml(options: {
  readonly cspSource: string;
  readonly script: string;
  readonly styles: readonly string[];
  readonly title: string;
  readonly nonce: string;
}): string {
  const links = options.styles.map((style) => `<link rel="stylesheet" href="${style}">`).join("\n");
  const csp = [
    "default-src 'none'",
    `style-src ${options.cspSource}`,
    `font-src ${options.cspSource}`,
    `script-src 'nonce-${options.nonce}'`,
  ].join("; ");
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
${links}
<title>${options.title}</title>
</head>
<body>
<div id="root"></div>
<script nonce="${options.nonce}" src="${options.script}"></script>
</body>
</html>`;
}

/** The open pages' loaders, so a development build can reload them in place. */
const loaders = new Set<() => void>();

/**
Show a page in `panel`: `script` from dist/, `styles` from media/ (a copy of the repository's
style/). A version in each address makes a reload fetch the files again. Reloaded by
`reloadPages` until the panel closes.
*/
export function showPage(
  panel: vscode.WebviewPanel,
  extensionUri: vscode.Uri,
  page: { readonly script: string; readonly styles: readonly string[]; readonly title: string },
): void {
  const webview = panel.webview;
  const load = (): void => {
    const version = Date.now().toString(36);
    const address = (folder: string, file: string): string =>
      `${webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, folder, file)).toString()}?v=${version}`;
    webview.html = pageHtml({
      cspSource: webview.cspSource,
      script: address("dist", page.script),
      styles: page.styles.map((style) => address("media", style)),
      title: page.title,
      nonce: randomBytes(16).toString("base64"),
    });
  };
  load();
  loaders.add(load);
  panel.onDidDispose(() => {
    loaders.delete(load);
  });
}

/** Load every open page again: its script and styles fresh, and it asks for its data again. */
export function reloadPages(): void {
  for (const load of loaders) {
    load();
  }
}

export class HistoryPanel {
  private static current: HistoryPanel | undefined;

  /**
  Show `graph`, in the open panel if there is one. Resolves to the rows the page drew, or
  undefined if it didn't say within RENDER_TIMEOUT_MS.
  */
  static async show(extensionUri: vscode.Uri, graph: Graph): Promise<number | undefined> {
    let current = this.current;
    if (current === undefined) {
      const panel = vscode.window.createWebviewPanel(
        VIEW_TYPE,
        "History",
        vscode.ViewColumn.Active,
        {
          enableScripts: true,
          localResourceRoots: [
            vscode.Uri.joinPath(extensionUri, "dist"),
            vscode.Uri.joinPath(extensionUri, "media"),
          ],
        },
      );
      current = new HistoryPanel(panel, extensionUri);
      this.current = current;
    } else {
      current.panel.reveal();
    }
    current.graph = graph;
    current.panel.title = `History: ${graph.rev}`;
    const rendered = current.waitForRender(graph.oid);
    current.post();
    return rendered;
  }

  private graph: Graph | undefined;

  private readonly renders = new vscode.EventEmitter<{ oid: string; rows: number }>();

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "webview.js",
      styles: ["brand.css", "history.css"],
      title: "History",
    });
    // The page asks for the graph whenever it (re)loads.
    webview.onDidReceiveMessage((value: unknown) => {
      const message = parseFromPage(value);
      if (message?.type === "ready") {
        this.post();
      } else if (message?.type === "rendered") {
        this.renders.fire({ oid: message.oid, rows: message.rows });
      }
    });
    panel.onDidDispose(() => {
      this.renders.dispose();
      if (HistoryPanel.current === this) {
        HistoryPanel.current = undefined;
      }
    });
  }

  private async waitForRender(oid: string): Promise<number | undefined> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        listener.dispose();
        resolve(undefined);
      }, RENDER_TIMEOUT_MS);
      const listener = this.renders.event((render) => {
        if (render.oid !== oid) {
          return;
        }

        clearTimeout(timer);
        listener.dispose();
        resolve(render.rows);
      });
    });
  }

  private post(): void {
    if (this.graph === undefined) {
      return;
    }

    const message: ToPage = { type: "graph", graph: this.graph };
    void this.panel.webview.postMessage(message);
  }
}
