// The containment webview panel: one at a time. Results are sent as they arrive and replayed
// whenever the page (re)loads.

import { randomBytes } from "node:crypto";

import * as vscode from "vscode";

import type { ToContainmentPage } from "./containment.js";
import { parseFromPage, type Change, type Control, type Revision } from "./messages.js";
import { pageHtml } from "./panel.js";
import { revisionUri } from "./revisions.js";

const VIEW_TYPE = "sundgren.containment";
/** How long `rendered` waits for the page after the last result. */
const RENDER_TIMEOUT_MS = 15_000;

export class ContainmentPanel {
  private static current: ContainmentPanel | undefined;

  /**
  Open the panel, or reuse the open one, for a new comparison in `repository`. `choose` is called
  when the user clicks a control in the page's toolbar.
  */
  static open(
    extensionUri: vscode.Uri,
    title: string,
    repository: string,
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
    current.repository = repository;
    return current;
  }

  private messages: ToContainmentPage[] = [];

  private choose: ((control: Control) => void) | undefined;

  private repository = "";

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
          this.choose?.(message.control);
          break;
        }
        case "diff": {
          void this.diff(message.left, message.right, message.title, message.line);
          break;
        }
        case "changes": {
          void this.changes(message.title, message.changes);
          break;
        }
        case "reveal":
        case undefined: {
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

  /** Two revisions side by side, beside the panel, at `line` of the right one. */
  private async diff(left: Revision, right: Revision, title: string, line?: number): Promise<void> {
    const at = line === undefined ? undefined : new vscode.Position(line - 1, 0);
    await vscode.commands.executeCommand(
      "vscode.diff",
      revisionUri(this.repository, left),
      revisionUri(this.repository, right),
      title,
      {
        viewColumn: vscode.ViewColumn.Beside,
        preview: true,
        ...(at !== undefined && { selection: new vscode.Range(at, at) }),
      } satisfies vscode.TextDocumentShowOptions,
    );
  }

  /** A branch's changed files in one multi-file diff. */
  private async changes(title: string, changes: readonly Change[]): Promise<void> {
    const revision = (oid: string, path: string): vscode.Uri =>
      revisionUri(this.repository, { oid, path });
    await vscode.commands.executeCommand(
      "vscode.changes",
      title,
      changes.map((change) => [
        revision(change.to, change.path),
        revision(change.from, change.path),
        revision(change.to, change.path),
      ]),
    );
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
