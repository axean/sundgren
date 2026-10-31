// The containment webview panel: one at a time. Results are sent as they arrive and replayed
// whenever the page (re)loads.

import * as vscode from "vscode";

import type { ToContainmentPage } from "./containment.js";
import { parseFromPage, type Control } from "./messages.js";
import { showPage } from "./panel.js";

/** What the page asks of the comparison: change a control, or inspect a branch. */
export interface ContainmentHandlers {
  readonly choose: (control: Control) => void;
  readonly inspect: (branch: string, focus?: { path: string; line?: number }) => void;
}

const VIEW_TYPE = "sundgren.containment";
/** How long `rendered` waits for the page after the last result. */
const RENDER_TIMEOUT_MS = 15_000;

export class ContainmentPanel {
  private static current: ContainmentPanel | undefined;

  /** Open the panel, or reuse the open one, for a new comparison; `handlers` act on its clicks. */
  static open(
    extensionUri: vscode.Uri,
    title: string,
    handlers: ContainmentHandlers,
  ): ContainmentPanel {
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
      current = new ContainmentPanel(panel, extensionUri);
      this.current = current;
    } else {
      current.panel.reveal();
      current.stopped.fire();
    }
    current.panel.title = title;
    current.messages = [];
    current.handlers = handlers;
    return current;
  }

  private messages: ToContainmentPage[] = [];

  private handlers: ContainmentHandlers | undefined;

  private readonly renders = new vscode.EventEmitter<number>();

  /** Fires when the panel closes or starts another comparison: stop this one. */
  private readonly stopped = new vscode.EventEmitter<void>();

  readonly onStop = this.stopped.event;

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "containment.js",
      styles: ["brand.css", "history.css", "containment.css"],
      title: "Containment",
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
          if (message.oid === "containment") {
            this.renders.fire(message.rows);
          }
          break;
        }
        case "choose": {
          this.handlers?.choose(message.control);
          break;
        }
        case "inspect": {
          const { branch, path, line } = message;
          this.handlers?.inspect(
            branch,
            path === undefined ? undefined : { path, ...(line !== undefined && { line }) },
          );
          break;
        }
        default: {
          break;
        }
      }
    });
    panel.onDidDispose(() => {
      this.stopped.fire();
      this.stopped.dispose();
      this.renders.dispose();
      if (ContainmentPanel.current === this) {
        ContainmentPanel.current = undefined;
      }
    });
  }

  post(message: ToContainmentPage): void {
    this.messages.push(message);
    void this.panel.webview.postMessage(message);
  }

  /** Send `done` and resolve to the rows the page drew, or undefined if it didn't say in time. */
  async finish(): Promise<number | undefined> {
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
    this.post({ type: "done" });
    return rendered;
  }
}
