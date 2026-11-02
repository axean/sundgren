// The origin webview panel: one at a time. Its messages are replayed whenever the page (re)loads.

import * as vscode from "vscode";

import { parseFromPage, type Control } from "./messages.js";
import type { ToOriginPage } from "./origin.js";
import { showPage } from "./panel.js";

/** What the page asks for: another repository or other commits, or to inspect one commit's row. */
export interface OriginHandlers {
  readonly choose: (control: Control) => void;
  readonly inspect: (rev: string) => void;
}

const VIEW_TYPE = "sundgren.origin";
/** How long `show` waits for the page to draw. */
const RENDER_TIMEOUT_MS = 15_000;

export class OriginPanel {
  private static current: OriginPanel | undefined;

  /** Open the panel, or reuse the open one; `handlers` act on its clicks. */
  static open(extensionUri: vscode.Uri, title: string, handlers: OriginHandlers): OriginPanel {
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
      current = new OriginPanel(panel, extensionUri);
      this.current = current;
    } else {
      current.panel.reveal();
    }
    current.panel.title = title;
    current.messages = [];
    current.handlers = handlers;
    return current;
  }

  private messages: ToOriginPage[] = [];

  private handlers: OriginHandlers | undefined;

  private readonly renders = new vscode.EventEmitter<number>();

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "origin.js",
      styles: ["brand.css", "history.css", "containment.css"],
      title: "Origin",
    });
    webview.onDidReceiveMessage((value: unknown) => {
      const message = parseFromPage(value);
      switch (message?.type) {
        case "ready": {
          for (const replay of this.messages) {
            void webview.postMessage(replay);
          }
          break;
        }
        case "rendered": {
          if (message.oid === "origin") {
            this.renders.fire(message.rows);
          }
          break;
        }
        case "choose": {
          this.handlers?.choose(message.control);
          break;
        }
        case "inspect": {
          this.handlers?.inspect(message.branch);
          break;
        }
        default: {
          break;
        }
      }
    });
    panel.onDidDispose(() => {
      this.renders.dispose();
      if (OriginPanel.current === this) {
        OriginPanel.current = undefined;
      }
    });
  }

  /**
  Post `message`; for a result or a failure, resolve to the rows the page drew, or undefined if it
  didn't say in time.
  */
  async post(message: ToOriginPage): Promise<number | undefined> {
    this.messages.push(message);
    if (message.type === "start") {
      void this.panel.webview.postMessage(message);
      return undefined;
    }
    const rendered = new Promise<number | undefined>((resolve) => {
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
    void this.panel.webview.postMessage(message);
    return rendered;
  }
}
