// The outline webview panel: one at a time, beside the editor, showing how sundgren reads a file.

import * as vscode from "vscode";

import { parseFromPage } from "./messages.js";
import type { ToOutlinePage } from "./outline.js";
import { showPage } from "./panel.js";

const VIEW_TYPE = "sundgren.outline";
/** How long `show` waits for the page to draw. */
const RENDER_TIMEOUT_MS = 15_000;

export class OutlinePanel {
  private static current: OutlinePanel | undefined;

  /** The open panel, if there is one. */
  static get open(): OutlinePanel | undefined {
    return this.current;
  }

  /**
  Show `message` in the panel, opening it beside the editor if needed. Resolves to the items the
  page drew, or undefined if it didn't say within RENDER_TIMEOUT_MS.
  */
  static async show(
    extensionUri: vscode.Uri,
    message: ToOutlinePage,
    shouldFocus: boolean,
  ): Promise<number | undefined> {
    let current = this.current;
    if (current === undefined) {
      const panel = vscode.window.createWebviewPanel(
        VIEW_TYPE,
        "Outline",
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: !shouldFocus },
        {
          enableScripts: true,
          retainContextWhenHidden: true,
          localResourceRoots: [
            vscode.Uri.joinPath(extensionUri, "dist"),
            vscode.Uri.joinPath(extensionUri, "media"),
          ],
        },
      );
      current = new OutlinePanel(panel, extensionUri);
      this.current = current;
    } else if (shouldFocus) {
      current.panel.reveal(undefined, false);
    }
    current.message = message;
    current.panel.title = `Outline: ${message.path.split(/[/\\]/u).at(-1) ?? message.path}`;
    const rendered = current.waitForRender(message.path);
    current.post();
    return rendered;
  }

  private message: ToOutlinePage | undefined;

  private readonly renders = new vscode.EventEmitter<{ path: string; items: number }>();

  private constructor(
    private readonly panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
  ) {
    const webview = panel.webview;
    showPage(panel, extensionUri, {
      script: "outline.js",
      styles: ["history.css", "outline.css"],
      title: "Outline",
    });
    webview.onDidReceiveMessage((value: unknown) => {
      const message = parseFromPage(value);
      switch (message?.type) {
        case "ready": {
          this.post();
          break;
        }
        case "rendered": {
          this.renders.fire({ path: message.oid, items: message.rows });
          break;
        }
        case "reveal": {
          void this.reveal(message.line);
          break;
        }
        case undefined: {
          break;
        }
      }
    });
    panel.onDidDispose(() => {
      this.renders.dispose();
      if (OutlinePanel.current === this) {
        OutlinePanel.current = undefined;
      }
    });
  }

  /** Show `line` (1-based) of the file in an editor next to the panel. */
  private async reveal(line: number): Promise<void> {
    const path = this.path;
    if (path === undefined) {
      return;
    }
    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(path));
    const visible = vscode.window.visibleTextEditors.find((editor) => editor.document === document);
    const editor = await vscode.window.showTextDocument(document, {
      viewColumn: visible?.viewColumn ?? vscode.ViewColumn.One,
    });
    const position = new vscode.Position(Math.min(line, document.lineCount) - 1, 0);
    editor.selection = new vscode.Selection(position, position);
    editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.AtTop);
  }

  private async waitForRender(path: string): Promise<number | undefined> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        listener.dispose();
        resolve(undefined);
      }, RENDER_TIMEOUT_MS);
      const listener = this.renders.event((render) => {
        if (render.path !== path) {
          return;
        }
        clearTimeout(timer);
        listener.dispose();
        resolve(render.items);
      });
    });
  }

  private post(): void {
    if (this.message !== undefined) {
      void this.panel.webview.postMessage(this.message);
    }
  }

  /** The file shown, for following the editor. */
  get path(): string | undefined {
    return this.message?.path;
  }
}
