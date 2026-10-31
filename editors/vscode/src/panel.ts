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
    webview.html = pageHtml({
      cspSource: webview.cspSource,
      script: webview
        .asWebviewUri(vscode.Uri.joinPath(extensionUri, "dist", "webview.js"))
        .toString(),
      styles: [
        webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, "media", "history.css")).toString(),
      ],
      title: "History",
      nonce: randomBytes(16).toString("base64"),
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
