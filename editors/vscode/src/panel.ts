// The history webview panel: one at a time, showing the last graph it was given.

import { randomBytes } from "node:crypto";

import * as vscode from "vscode";

import type { Graph } from "./graph.js";

const VIEW_TYPE = "sundgren.history";

/** The page: the bundled script and stylesheet, nothing else may load. */
export function pageHtml(options: {
  readonly cspSource: string;
  readonly script: string;
  readonly style: string;
  readonly nonce: string;
}): string {
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
<link rel="stylesheet" href="${options.style}">
<title>History</title>
</head>
<body>
<div id="root"></div>
<script nonce="${options.nonce}" src="${options.script}"></script>
</body>
</html>`;
}

export class HistoryPanel {
  private static current: HistoryPanel | undefined;

  /** Show `graph`, in the open panel if there is one. */
  static show(extensionUri: vscode.Uri, graph: Graph): void {
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
    current.post();
  }

  private graph: Graph | undefined;

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
      style: webview
        .asWebviewUri(vscode.Uri.joinPath(extensionUri, "media", "history.css"))
        .toString(),
      nonce: randomBytes(16).toString("base64"),
    });
    // The page asks for the graph whenever it (re)loads.
    webview.onDidReceiveMessage((message: unknown) => {
      if (
        typeof message === "object" &&
        message !== null &&
        "type" in message &&
        message.type === "ready"
      ) {
        this.post();
      }
    });
    panel.onDidDispose(() => {
      if (HistoryPanel.current === this) {
        HistoryPanel.current = undefined;
      }
    });
  }

  private post(): void {
    if (this.graph !== undefined) {
      void this.panel.webview.postMessage({ type: "graph", graph: this.graph });
    }
  }
}
