// The inspect webview panel: one at a time, beside the containment panel.

import path from "node:path";

import * as vscode from "vscode";

import type { ToInspectPage } from "./inspect.js";
import { parseFromPage, type Revision } from "./messages.js";
import { WORK_TREE } from "./origin.js";
import { showPage } from "./panel.js";
import { revisionUri } from "./revisions.js";

const VIEW_TYPE = "sundgren.inspect";
/** How long `show` waits for the page to draw. */
const RENDER_TIMEOUT_MS = 15_000;

export class InspectPanel {
  private static current: InspectPanel | undefined;

  /**
  Show `message` for `repository`, opening the panel beside the active one if needed; `others`
  are the repositories of commits that aren't in it, by commit id. Resolves to the lines the page
  drew, or undefined if it didn't say within RENDER_TIMEOUT_MS.
  */
  static async show(
    extensionUri: vscode.Uri,
    repository: string,
    message: ToInspectPage,
    others: ReadonlyMap<string, string> = new Map(),
  ): Promise<number | undefined> {
    let current = this.current;
    if (current === undefined) {
      const panel = vscode.window.createWebviewPanel(
        VIEW_TYPE,
        "sundgren: Inspect",
        vscode.ViewColumn.Beside,
        {
          enableScripts: true,
          retainContextWhenHidden: true,
          localResourceRoots: [
            vscode.Uri.joinPath(extensionUri, "dist"),
            vscode.Uri.joinPath(extensionUri, "media"),
          ],
        },
      );
      current = new InspectPanel(panel, extensionUri);
      this.current = current;
    } else {
      current.panel.reveal(undefined, false);
    }
    current.repository = repository;
    current.others = others;
    current.message = message;
    current.panel.title = `sundgren: Inspect (${message.branch})`;
    const rendered = current.waitForRender(message.branch);
    current.post();
    return rendered;
  }

  private message: ToInspectPage | undefined;

  private repository = "";

  private others: ReadonlyMap<string, string> = new Map();

  private readonly renders = new vscode.EventEmitter<{ branch: string; lines: number }>();

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "inspect.js",
      styles: ["brand.css", "history.css", "containment.css", "inspect.css"],
      title: "Inspect",
    });
    webview.onDidReceiveMessage((value: unknown) => {
      const message = parseFromPage(value);
      switch (message?.type) {
        case "ready": {
          this.post();
          break;
        }
        case "rendered": {
          this.renders.fire({ branch: message.oid, lines: message.rows });
          break;
        }
        case "open": {
          void this.open(message.revision, message.line);
          break;
        }
        default: {
          break;
        }
      }
    });
    panel.onDidDispose(() => {
      this.renders.dispose();
      if (InspectPanel.current === this) {
        InspectPanel.current = undefined;
      }
    });
  }

  /**
  A file at a commit, read-only, in the editor group next to the panel, at `line`; a file of the
  work tree (WORK_TREE) as it is on disk.
  */
  private async open(revision: Revision, line = 1): Promise<void> {
    const uri =
      revision.oid === WORK_TREE
        ? vscode.Uri.file(path.join(this.repository, revision.path))
        : revisionUri(this.others.get(revision.oid) ?? this.repository, revision);
    const document = await vscode.workspace.openTextDocument(uri);
    const at = new vscode.Position(Math.max(0, line - 1), 0);
    await vscode.window.showTextDocument(document, {
      viewColumn: vscode.ViewColumn.Beside,
      preview: true,
      selection: new vscode.Range(at, at),
    });
  }

  private async waitForRender(branch: string): Promise<number | undefined> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        listener.dispose();
        resolve(undefined);
      }, RENDER_TIMEOUT_MS);
      const listener = this.renders.event((render) => {
        if (render.branch !== branch) {
          return;
        }
        clearTimeout(timer);
        listener.dispose();
        resolve(render.lines);
      });
    });
  }

  private post(): void {
    if (this.message !== undefined) {
      void this.panel.webview.postMessage(this.message);
    }
  }
}
