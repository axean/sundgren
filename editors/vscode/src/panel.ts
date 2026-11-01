// The webview pages' HTML, and the history panel.

import { randomBytes } from "node:crypto";

import * as vscode from "vscode";

import {
  parseFromPage,
  type Control,
  type FromPage,
  type Selection,
  type ToPage,
  type View,
} from "./messages.js";

const VIEW_TYPE = "sundgren.history";
/** How long `rendered` waits for the page to draw a graph. */
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

/** What the history page asks for: another view, a control changed, something clicked. */
export interface HistoryHandlers {
  readonly view: (view: View) => void;
  readonly choose: (control: Control) => void;
  readonly select: (selection: Selection) => void;
}

/** The history panel: one at a time. The last setup and result are replayed when the page reloads. */
export class HistoryPanel {
  private static current: HistoryPanel | undefined;

  /** Open the panel, or reuse the open one, for a new map; `handlers` act on what the page asks. */
  static open(extensionUri: vscode.Uri, title: string, handlers: HistoryHandlers): HistoryPanel {
    let current = this.current;
    if (current === undefined) {
      const panel = vscode.window.createWebviewPanel(VIEW_TYPE, title, vscode.ViewColumn.Active, {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [
          vscode.Uri.joinPath(extensionUri, "dist"),
          vscode.Uri.joinPath(extensionUri, "media"),
        ],
      });
      current = new HistoryPanel(panel, extensionUri);
      this.current = current;
    } else {
      current.panel.reveal();
      current.stopped.fire();
    }
    current.panel.title = title;
    current.setup = undefined;
    current.last = undefined;
    current.handlers = handlers;
    return current;
  }

  private setup: Extract<ToPage, { type: "setup" }> | undefined;

  /** The last running, graph or error message. */
  private last: ToPage | undefined;

  private handlers: HistoryHandlers | undefined;

  private readonly renders = new vscode.EventEmitter<number>();

  /** Fires when the panel closes or starts another map: stop this one. */
  private readonly stopped = new vscode.EventEmitter<void>();

  readonly onStop = this.stopped.event;

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "webview.js",
      styles: ["brand.css", "history.css", "containment.css"],
      title: "History",
    });
    webview.onDidReceiveMessage((value: unknown) => {
      this.receive(parseFromPage(value));
    });
    panel.onDidDispose(() => {
      this.stopped.fire();
      this.stopped.dispose();
      this.renders.dispose();
      if (HistoryPanel.current === this) {
        HistoryPanel.current = undefined;
      }
    });
  }

  private receive(message: FromPage | undefined): void {
    switch (message?.type) {
      case "ready": {
        if (this.setup !== undefined) {
          void this.panel.webview.postMessage(this.setup);
        }
        if (this.last !== undefined) {
          void this.panel.webview.postMessage(this.last);
        }
        break;
      }
      case "rendered": {
        this.renders.fire(message.rows);
        break;
      }
      case "view": {
        const { offset, columns, branches } = message;
        const view = { offset, columns, branches };
        if (this.setup !== undefined) {
          this.setup = { ...this.setup, view };
        }
        this.handlers?.view(view);
        break;
      }
      case "choose": {
        this.handlers?.choose(message.control);
        break;
      }
      case "select": {
        this.handlers?.select(message.selection);
        break;
      }
      default: {
        break;
      }
    }
  }

  post(message: ToPage): void {
    if (message.type === "setup") {
      this.setup = message;
    } else {
      this.last = message;
    }
    void this.panel.webview.postMessage(message);
  }

  /** The lanes the page draws next, or undefined if it doesn't say in RENDER_TIMEOUT_MS. */
  async rendered(): Promise<number | undefined> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        listener.dispose();
        resolve(undefined);
      }, RENDER_TIMEOUT_MS);
      const listener = this.renders.event((rows) => {
        clearTimeout(timer);
        listener.dispose();
        resolve(rows);
      });
    });
  }
}
