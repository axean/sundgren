// The VS Code side: the "show history" command. It picks a repository and a branch, runs
// `sundgren graph` and shows the result in the history panel.

import path from "node:path";

import * as vscode from "vscode";

import { parseGraph } from "./graph.js";
import { branches, repositoryRoot, type Branch } from "./git.js";
import { HistoryPanel } from "./panel.js";
import { locateSundgren, runSundgren } from "./sundgren.js";

/** Optional arguments of `sundgren.showHistory`, for scripts and tests; asked for if missing. */
export interface ShowArguments {
  readonly repository?: string;
  readonly rev?: string;
}

/** How `sundgren.showHistory` ended; returned by the command, for tests and scripts. */
export type ShowResult =
  | {
      readonly kind: "shown";
      readonly rev: string;
      readonly commits: number;
      readonly truncated: boolean;
      readonly binary: string;
    }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "cancelled" };

const CANCELLED: ShowResult = { kind: "cancelled" };

function failed(message: string): ShowResult {
  void vscode.window.showErrorMessage(`sundgren: ${message}`);
  return { kind: "failed", message };
}

/** The repositories of the open folders. */
async function workspaceRepositories(): Promise<string[]> {
  const folders = vscode.workspace.workspaceFolders ?? [];
  const roots = await Promise.all(folders.map((folder) => repositoryRoot(folder.uri.fsPath)));
  return [...new Set(roots.filter((root) => root !== undefined))];
}

async function pickRepository(): Promise<string | undefined> {
  const repositories = await workspaceRepositories();
  if (repositories.length <= 1) {
    return repositories[0];
  }
  const picked = await vscode.window.showQuickPick(
    repositories.map((repository) => ({
      label: path.basename(repository),
      description: repository,
    })),
    { placeHolder: "Repository" },
  );
  return picked?.description;
}

function branchDescription(branch: Branch): string {
  if (branch.isCurrent) {
    return "current";
  }
  return branch.isRemote ? "remote" : "";
}

async function pickRev(repository: string): Promise<string | undefined> {
  const found = await branches(repository);
  if (found.length === 0) {
    return "HEAD";
  }
  const items: vscode.QuickPickItem[] = found.map((branch) => ({
    label: branch.name,
    description: branchDescription(branch),
  }));
  const picked = await vscode.window.showQuickPick(items, { placeHolder: "Branch" });
  return picked?.label;
}

async function showHistory(
  context: vscode.ExtensionContext,
  arguments_: ShowArguments | undefined,
): Promise<ShowResult> {
  const repository = arguments_?.repository ?? (await pickRepository());
  if (repository === undefined) {
    if ((vscode.workspace.workspaceFolders ?? []).length === 0) {
      return failed("open a folder in a git repository first.");
    }
    const repositories = await workspaceRepositories();
    return repositories.length === 0 ? failed("no git repository in the open folders.") : CANCELLED;
  }
  const rev = arguments_?.rev ?? (await pickRev(repository));
  if (rev === undefined) {
    return CANCELLED;
  }

  const settings = vscode.workspace.getConfiguration("sundgren");
  const binary = locateSundgren({
    setting: settings.get<string>("path", ""),
    bundled: context.asAbsolutePath(path.join("bundled", "sundgren")),
    repository,
    pathEnvironment: process.env["PATH"],
  });
  if (binary === undefined) {
    return failed("can't find the sundgren binary; set sundgren.path.");
  }
  if ("error" in binary) {
    return failed(binary.error);
  }
  const maxCommits = Math.max(1, settings.get<number>("maxCommits", 2000));

  const result = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Window,
      title: `sundgren: reading the history of ${rev}`,
    },
    () =>
      runSundgren(
        binary.path,
        ["graph", "--repo", repository, "--max-count", String(maxCommits), "--", rev],
        repository,
      ).done,
  );
  if (result.code !== 0) {
    const message = result.stderr.trim().replace(/^sundgren: /u, "");
    return failed(message === "" ? `sundgren exited with ${String(result.code)}` : message);
  }
  let json: unknown;
  try {
    json = JSON.parse(result.stdout);
  } catch {
    return failed("sundgren printed something that isn't JSON.");
  }
  const graph = parseGraph(json);
  if (graph === undefined) {
    return failed("sundgren printed JSON that isn't a sundgren.graph/1 document.");
  }
  HistoryPanel.show(context.extensionUri, graph);
  return {
    kind: "shown",
    rev,
    commits: graph.commits.length,
    truncated: graph.truncated,
    binary: binary.path,
  };
}

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("sundgren.showHistory", (arguments_?: ShowArguments) =>
      showHistory(context, arguments_),
    ),
  );
}

export function deactivate(): void {
  // Nothing to stop: a running `sundgren graph` ends on its own within moments.
}
