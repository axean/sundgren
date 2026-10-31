// The VS Code side: the "show history", "show containment" and "show outline" commands. They
// run `sundgren` and show the result in a panel.

import { availableParallelism } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import { ContainmentPanel } from "./containment-panel.js";
import { InspectPanel } from "./inspect-panel.js";
import { parseContainment, type Comments, type Meta } from "./containment.js";
import { parseGraph } from "./graph.js";
import {
  branchStates,
  branches,
  defaultBranch,
  repositoryRoot,
  type Branch,
  type BranchState,
} from "./git.js";
import { flatten, parseOutline, type ToOutlinePage } from "./outline.js";
import { OutlinePanel } from "./outline-panel.js";
import type { Control } from "./messages.js";
import { HistoryPanel } from "./panel.js";
import { REVISION_SCHEME, RevisionProvider } from "./revisions.js";
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

/**
Optional arguments of `sundgren.showContainment`. The base defaults to the `sundgren.base`
setting, then the repository's default branch; the rest are asked for if missing.
*/
export interface ContainmentArguments {
  readonly repository?: string;
  readonly base?: string;
  /** Whether lines that are only a comment are work; ignored by default. */
  readonly comments?: Comments;
  readonly target?: string;
  /** The branches to compare with the target; all others by default. */
  readonly branches?: readonly string[];
}

/** Arguments of `sundgren.inspect`: a branch compared with a target, from a base. */
export interface InspectArguments {
  readonly repository: string;
  readonly base: string;
  readonly target: string;
  readonly branch: string;
  readonly comments?: Comments;
  /** Where to scroll to. */
  readonly path?: string;
  readonly line?: number;
}

/** How `sundgren.inspect` ended. */
export type InspectResult =
  | {
      readonly kind: "shown";
      /** The branch's lines, comments left out included. */
      readonly lines: number;
      /** Lines the panel drew, or undefined if it didn't confirm in time. */
      readonly rendered: number | undefined;
    }
  | { readonly kind: "failed"; readonly message: string };

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

/** Optional arguments of `sundgren.showOutline`; the active editor's file by default. */
export interface OutlineArguments {
  readonly file?: string;
}

/** How `sundgren.showOutline` ended. */
export type OutlineResult =
  | {
      readonly kind: "shown";
      readonly path: string;
      /** Left out if sundgren doesn't read the file's language. */
      readonly language?: string;
      /** All items, nested ones too. */
      readonly items: number;
      /** Items the panel drew, or undefined if it didn't confirm in time. */
      readonly rendered: number | undefined;
    }
  | { readonly kind: "failed"; readonly message: string };

const CANCELLED = { kind: "cancelled" } as const;

/** The commit the extension was built from: set by esbuild (esbuild.mjs); "dev" without it. */
declare const SUNDGREN_COMMIT: string | undefined;

/** How similar an edited line must be to count as found: `sundgren contain`'s default. */
const FUZZY_THRESHOLD = 0.9;

/** `sundgren contain` arguments for `branch` and `target`, from `base`, with `options`. */
function containArguments(
  repository: string,
  base: string,
  comments: Comments,
  branch: string,
  target: string,
  options: readonly string[] = [],
): string[] {
  return [
    "contain",
    "--repo",
    repository,
    "--base",
    base,
    "--fuzzy",
    String(FUZZY_THRESHOLD),
    "--comments",
    comments === "ignored" ? "ignore" : "count",
    ...options,
    "--",
    branch,
    target,
  ];
}

/** The extension's version and commit, and `sundgren --version` of `binary`. */
async function meta(context: vscode.ExtensionContext, binary: Binary): Promise<Meta> {
  const packageJson: unknown = context.extension.packageJSON;
  const version =
    typeof packageJson === "object" &&
    packageJson !== null &&
    "version" in packageJson &&
    typeof packageJson.version === "string"
      ? packageJson.version
      : "unknown";
  const result = await runSundgren(binary.path, ["--version"], path.dirname(binary.path)).done;
  const binaryVersion = result.code === 0 ? result.stdout.trim() : "sundgren (version unknown)";
  return {
    version,
    commit: typeof SUNDGREN_COMMIT === "string" ? SUNDGREN_COMMIT : "dev",
    binary: `${binaryVersion} (${binary.source})`,
  };
}

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

/**
The branches to compare with `target`: every branch but the base, the target, and a local branch
that only tracks the base (`main` when the base is `origin/main`); or those `names`, if given.
*/
async function branchesToCompare(
  repository: string,
  base: string,
  target: string,
  names: readonly string[] | undefined,
): Promise<BranchState[]> {
  const states = await branchStates(repository);
  const named = (name: string): BranchState =>
    states.find((state) => state.name === name) ?? { name, isRemote: name.includes("/") };
  return names === undefined
    ? states.filter(
        (state) => state.name !== base && state.name !== target && state.upstream !== base,
      )
    : names.map((name) => named(name));
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

/** `branch` against `target` line by line, in the inspect panel. */
async function inspect(
  context: vscode.ExtensionContext,
  arguments_: InspectArguments,
): Promise<InspectResult> {
  const { repository, base, target, branch, path: file, line } = arguments_;
  const comments = arguments_.comments ?? "ignored";
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const result = await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Window, title: `sundgren: inspecting ${branch}` },
    () =>
      runSundgren(
        binary.path,
        containArguments(repository, base, comments, branch, target, ["--lines"]),
        repository,
      ).done,
  );
  let parsed: unknown;
  try {
    parsed = JSON.parse(result.stdout);
  } catch {
    parsed = undefined;
  }
  const containment = result.code === 0 ? parseContainment(parsed) : undefined;
  if (containment === undefined) {
    const message = result.stderr.trim().replace(/^sundgren: /u, "");
    await InspectPanel.show(context.extensionUri, repository, {
      type: "error",
      branch,
      message: message === "" ? "no result" : message,
    });
    return failed(message === "" ? `can't inspect ${branch}` : message);
  }
  const rendered = await InspectPanel.show(context.extensionUri, repository, {
    type: "inspect",
    branch,
    target,
    base,
    comments,
    threshold: FUZZY_THRESHOLD,
    result: containment,
    ...(file !== undefined && { focus: { path: file, ...(line !== undefined && { line }) } }),
    meta: await meta(context, binary),
  });
  return {
    kind: "shown",
    lines: containment.a_in_b.lines_found?.length ?? 0,
    rendered,
  };
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
  const setting = vscode.workspace.getConfiguration("sundgren").get<string>("base", "").trim();
  const base =
    arguments_?.base ??
    (setting === "" ? undefined : setting) ??
    (await defaultBranch(repository)) ??
    (await pickBranch(
      repository,
      "Canonical branch: each branch's work is what it added since it left this one",
    ));
  if (base === undefined) {
    return CANCELLED;
  }
  const target =
    arguments_?.target ??
    (await pickBranch(
      repository,
      `Target: compare every branch with this one (work: what each added since it left ${base})`,
      [base],
    ));
  if (target === undefined) {
    return CANCELLED;
  }
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const states = await branchesToCompare(repository, base, target, arguments_?.branches);
  const others = states.map((state) => state.name);

  const comments = arguments_?.comments ?? "ignored";
  // A click on a control in the panel: pick another base or target, or count comments or not;
  // then compare again.
  const choose = async (control: Control): Promise<void> => {
    if (control === "comments") {
      await vscode.commands.executeCommand("sundgren.showContainment", {
        ...arguments_,
        repository,
        base,
        target,
        comments: comments === "ignored" ? "counted" : "ignored",
      } satisfies ContainmentArguments);
      return;
    }
    const picked =
      control === "base"
        ? await pickBranch(
            repository,
            `Canonical branch: each branch's work is what it added since it left this one (now ${base})`,
            [target],
          )
        : await pickBranch(
            repository,
            `Target: compare every branch with this one (now ${target})`,
            [base],
          );
    if (picked === undefined) {
      return;
    }
    await vscode.commands.executeCommand("sundgren.showContainment", {
      ...arguments_,
      repository,
      base: control === "base" ? picked : base,
      target: control === "target" ? picked : target,
    } satisfies ContainmentArguments);
  };
  const panel = ContainmentPanel.open(context.extensionUri, `Containment: ${target}`, {
    choose: (control) => {
      void choose(control);
    },
    inspect: (branch, focus) => {
      void vscode.commands.executeCommand("sundgren.inspect", {
        repository,
        base,
        target,
        branch,
        comments,
        ...focus,
      } satisfies InspectArguments);
    },
  });
  panel.post({
    type: "start",
    target,
    base,
    branches: others,
    states,
    comments,
    threshold: FUZZY_THRESHOLD,
    meta: await meta(context, binary),
  });
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
      containArguments(repository, base, comments, branch, target),
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

/** The page message for `file`: how sundgren reads it, or why it couldn't. */
async function readOutline(binary: string, file: string): Promise<ToOutlinePage> {
  const result = await runSundgren(
    binary,
    ["outline", "--tokens", "--format", "json", "--", file],
    path.dirname(file),
  ).done;
  if (result.code !== 0) {
    const message = result.stderr.trim().replace(/^sundgren: /u, "");
    return { type: "error", path: file, message: message === "" ? "no outline" : message };
  }
  let json: unknown;
  try {
    json = JSON.parse(result.stdout);
  } catch {
    json = undefined;
  }
  const outlined = parseOutline(json)?.files[0];
  if (outlined === undefined) {
    return {
      type: "error",
      path: file,
      message: "sundgren printed something that isn't an outline.",
    };
  }
  // The same bytes sundgren read, so the page can lay the tokens over them.
  const source = new TextDecoder().decode(
    await vscode.workspace.fs.readFile(vscode.Uri.file(file)),
  );
  return { type: "outline", path: file, source, file: outlined };
}

async function showOutline(
  context: vscode.ExtensionContext,
  file: string,
  shouldFocus: boolean,
): Promise<OutlineResult> {
  const repository = (await repositoryRoot(path.dirname(file))) ?? path.dirname(file);
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const message = await readOutline(binary.path, file);
  const rendered = await OutlinePanel.show(context.extensionUri, message, shouldFocus);
  if (message.type === "error") {
    return failed(message.message);
  }
  return {
    kind: "shown",
    path: file,
    ...(message.file.language !== undefined && { language: message.file.language }),
    items: flatten(message.file.items).length,
    rendered,
  };
}

/** Keep the open outline panel on the active editor's file, and up to date when it is saved. */
function followEditors(context: vscode.ExtensionContext): vscode.Disposable[] {
  const follow = (document: vscode.TextDocument | undefined, isSaved: boolean): void => {
    const panel = OutlinePanel.open;
    if (panel === undefined || document?.uri.scheme !== "file") {
      return;
    }
    const file = document.uri.fsPath;
    if (isSaved ? file === panel.path : file !== panel.path) {
      void showOutline(context, file, false);
    }
  };
  return [
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      follow(editor?.document, false);
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      follow(document, true);
    }),
  ];
}

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "sundgren.showOutline",
      async (arguments_?: OutlineArguments): Promise<OutlineResult> => {
        const document = vscode.window.activeTextEditor?.document;
        const file =
          arguments_?.file ?? (document?.uri.scheme === "file" ? document.uri.fsPath : undefined);
        return file === undefined ? failed("open a file first.") : showOutline(context, file, true);
      },
    ),
    vscode.commands.registerCommand(
      "sundgren.inspect",
      (arguments_: InspectArguments): Promise<InspectResult> => inspect(context, arguments_),
    ),
    ...followEditors(context),
    vscode.workspace.registerTextDocumentContentProvider(REVISION_SCHEME, new RevisionProvider()),
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
