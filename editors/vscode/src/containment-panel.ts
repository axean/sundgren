// The containment webview panel: one at a time. Results are sent as they arrive and replayed
// whenever the page (re)loads.

import { randomBytes } from "node:crypto";

import * as vscode from "vscode";

import type { ToContainmentPage } from "./containment.js";
import { parseFromPage, type Control } from "./messages.js";
import { pageHtml } from "./panel.js";

const VIEW_TYPE = "sundgren.containment";
/** How long `rendered` waits for the page after the last result. */
const RENDER_TIMEOUT_MS = 15_000;

export class ContainmentPanel {
  private static current: ContainmentPanel | undefined;

  /**
  Open the panel, or reuse the open one, for a new comparison. `choose` is called when the user
  clicks a control in the page's toolbar.
  */
  static open(
    extensionUri: vscode.Uri,
    title: string,
    choose: (control: Control) => void,
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
    current.choose = choose;
    return current;
  }

  private messages: ToContainmentPage[] = [];

  private choose: ((control: Control) => void) | undefined;

  private readonly renders = new vscode.EventEmitter<number>();

  /** Fires when the panel closes or starts another comparison: stop this one. */
  private readonly stopped = new vscode.EventEmitter<void>();

  readonly onStop = this.stopped.event;

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    const media = (file: string): string =>
      webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, "media", file)).toString();
    webview.html = pageHtml({
      cspSource: webview.cspSource,
      script: webview
        .asWebviewUri(vscode.Uri.joinPath(extensionUri, "dist", "containment.js"))
        .toString(),
      styles: [media("history.css"), media("containment.css")],
      title: "Containment",
      nonce: randomBytes(16).toString("base64"),
    });
    webview.onDidReceiveMessage((value: unknown) => {
      const message = parseFromPage(value);
      if (message?.type === "ready") {
        for (const replay of this.messages) {
          void webview.postMessage(replay);
        }
      } else if (message?.type === "rendered" && message.oid === "containment") {
        this.renders.fire(message.rows);
      } else if (message?.type === "choose") {
        this.choose?.(message.control);
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
