// The VS Code side: the "show history" command. It picks a repository and a branch, runs
// `sundgren graph` and shows the result in the history panel.

import { availableParallelism } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import { ContainmentPanel } from "./containment-panel.js";
import { parseContainment } from "./containment.js";
import { parseGraph } from "./graph.js";
import { branches, repositoryRoot, type Branch } from "./git.js";
import { HistoryPanel } from "./panel.js";
import { locateSundgren, runSundgren, type Binary, type RunHandle } from "./sundgren.js";

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
      /** Rows the panel drew, or undefined if it didn't confirm in time. */
      readonly rendered: number | undefined;
    }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "cancelled" };

/** Optional arguments of `sundgren.showContainment`; asked for if missing. */
export interface ContainmentArguments {
  readonly repository?: string;
  readonly base?: string;
  readonly target?: string;
  /** The branches to compare with the target; all others by default. */
  readonly branches?: readonly string[];
}

/** How `sundgren.showContainment` ended. */
export type ContainmentResult =
  | {
      readonly kind: "shown";
      readonly base: string;
      readonly target: string;
      readonly compared: number;
      readonly failed: number;
      /** Branches the panel drew with a result, or undefined if it didn't confirm in time. */
      readonly rendered: number | undefined;
    }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "cancelled" };

const CANCELLED = { kind: "cancelled" } as const;

function failed(message: string): { readonly kind: "failed"; readonly message: string } {
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

async function pickBranch(
  repository: string,
  placeholder: string,
  exclude: readonly string[] = [],
): Promise<string | undefined> {
  const all = await branches(repository);
  const found = all.filter((branch) => !exclude.includes(branch.name));
  if (found.length === 0) {
    return "HEAD";
  }
  const items: vscode.QuickPickItem[] = found.map((branch) => ({
    label: branch.name,
    description: branchDescription(branch),
  }));
  const picked = await vscode.window.showQuickPick(items, { placeHolder: placeholder });
  return picked?.label;
}

async function branchNames(repository: string): Promise<string[]> {
  const found = await branches(repository);
  return found.map((branch) => branch.name);
}

/** The repository to work in: given, or picked from the open folders. */
async function chooseRepository(
  given: string | undefined,
): Promise<string | typeof CANCELLED | { readonly kind: "failed"; readonly message: string }> {
  const repository = given ?? (await pickRepository());
  if (repository !== undefined) {
    return repository;
  }
  if ((vscode.workspace.workspaceFolders ?? []).length === 0) {
    return failed("open a folder in a git repository first.");
  }
  const repositories = await workspaceRepositories();
  return repositories.length === 0 ? failed("no git repository in the open folders.") : CANCELLED;
}

function findBinary(
  context: vscode.ExtensionContext,
  repository: string,
): Binary | { readonly kind: "failed"; readonly message: string } {
  const binary = locateSundgren({
    setting: vscode.workspace.getConfiguration("sundgren").get<string>("path", ""),
    bundled: context.asAbsolutePath(path.join("bundled", "sundgren")),
    repository,
    pathEnvironment: process.env["PATH"],
  });
  if (binary === undefined) {
    return failed("can't find the sundgren binary; set sundgren.path.");
  }
  return "error" in binary ? failed(binary.error) : binary;
}

async function showHistory(
  context: vscode.ExtensionContext,
  arguments_: ShowArguments | undefined,
): Promise<ShowResult> {
  const repository = await chooseRepository(arguments_?.repository);
  if (typeof repository !== "string") {
    return repository;
  }
  const rev = arguments_?.rev ?? (await pickBranch(repository, "Branch"));
  if (rev === undefined) {
    return CANCELLED;
  }
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const settings = vscode.workspace.getConfiguration("sundgren");
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
  const rendered = await HistoryPanel.show(context.extensionUri, graph);
  return {
    kind: "shown",
    rev,
    commits: graph.commits.length,
    truncated: graph.truncated,
    binary: binary.path,
    rendered,
  };
}

/** Run `work` on each item, at most `limit` at a time. */
async function eachLimited<T>(
  items: readonly T[],
  limit: number,
  work: (item: T) => Promise<void>,
): Promise<void> {
  let next = 0;
  const worker = async (): Promise<void> => {
    for (let item = items[next++]; item !== undefined; item = items[next++]) {
      await work(item);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
}

async function showContainment(
  context: vscode.ExtensionContext,
  arguments_: ContainmentArguments | undefined,
): Promise<ContainmentResult> {
  const repository = await chooseRepository(arguments_?.repository);
  if (typeof repository !== "string") {
    return repository;
  }
  const base =
    arguments_?.base ??
    (await pickBranch(repository, "Base: each branch's work is counted from where it left this"));
  if (base === undefined) {
    return CANCELLED;
  }
  const target =
    arguments_?.target ??
    (await pickBranch(repository, "Target: compare every branch with this one", [base]));
  if (target === undefined) {
    return CANCELLED;
  }
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const all = arguments_?.branches ?? (await branchNames(repository));
  const others = all.filter((name) => name !== base && name !== target);

  const panel = ContainmentPanel.open(context.extensionUri, `Containment: ${target}`);
  panel.post({ type: "start", target, base, branches: others });
  // Aborted when the panel closes or starts another comparison.
  const stopping = new AbortController();
  const isStopped = (): boolean => stopping.signal.aborted;
  const running = new Set<RunHandle>();
  const stop = panel.onStop(() => {
    stopping.abort();
    for (const handle of running) {
      handle.cancel();
    }
  });
  let failures = 0;
  const compare = async (branch: string): Promise<void> => {
    if (isStopped()) {
      return;
    }
    const handle = runSundgren(
      binary.path,
      ["contain", "--repo", repository, "--base", base, "--", branch, target],
      repository,
    );
    running.add(handle);
    const result = await handle.done;
    running.delete(handle);
    if (isStopped()) {
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(result.stdout);
    } catch {
      parsed = undefined;
    }
    const containment = result.code === 0 ? parseContainment(parsed) : undefined;
    if (containment === undefined) {
      failures += 1;
      const message = result.stderr.trim().replace(/^sundgren: /u, "");
      panel.post({ type: "failed", branch, message: message === "" ? "no result" : message });
    } else {
      panel.post({ type: "result", branch, result: containment });
    }
  };
  const limit = Math.max(1, Math.min(4, availableParallelism() - 1));
  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Window,
      title: `sundgren: comparing ${String(others.length)} branches with ${target}`,
    },
    () => eachLimited(others, limit, compare),
  );
  stop.dispose();
  if (isStopped()) {
    return CANCELLED;
  }
  const rendered = await panel.finish();
  return {
    kind: "shown",
    base,
    target,
    compared: others.length,
    failed: failures,
    rendered,
  };
}

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("sundgren.showHistory", (arguments_?: ShowArguments) =>
      showHistory(context, arguments_),
    ),
    vscode.commands.registerCommand(
      "sundgren.showContainment",
      (arguments_?: ContainmentArguments) => showContainment(context, arguments_),
    ),
  );
}

export function deactivate(): void {
  // Nothing to stop: running `sundgren` processes end on their own.
}
