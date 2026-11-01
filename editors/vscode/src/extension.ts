// The VS Code side: the "show history", "show containment" and "show outline" commands. They
// run `sundgren` and show the result in a panel.

import { availableParallelism } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import { ContainmentPanel } from "./containment-panel.js";
import { InspectPanel } from "./inspect-panel.js";
import { parseContainment, type Comments, type Meta } from "./containment.js";
import { parseGraph, type Graph, type ShowMode } from "./graph.js";
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
import { MAX_BRANCHES, type Control, type View } from "./messages.js";
import { HistoryPanel, reloadPages } from "./panel.js";
import { REVISION_SCHEME, RevisionProvider } from "./revisions.js";
import {
  locateSundgren,
  runSundgren,
  type Binary,
  type RunHandle,
  type RunResult,
} from "./sundgren.js";

/**
Optional arguments of `sundgren.showHistory`, for scripts, tests and the Containment panel. The
base defaults as for Containment (asked for if there is none). `like` implies show `like`.
*/
export interface ShowArguments {
  readonly repository?: string;
  readonly base?: string;
  /** How the branches are chosen; local ones first by default. */
  readonly show?: ShowMode;
  readonly like?: string;
  /** How many branches besides the base, 0 to 30; 12 by default. */
  readonly branches?: number;
  /** Branches always shown. */
  readonly pins?: readonly string[];
}

/** How `sundgren.showHistory` ended (its first map); returned by the command, for tests. */
export type ShowResult =
  | {
      readonly kind: "shown";
      readonly base: string;
      readonly show: ShowMode;
      readonly like?: string;
      readonly lanes: number;
      readonly commits: number;
      readonly binary: string;
      /** Lanes the panel drew, or undefined if it didn't confirm in time. */
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

/** The base: the `sundgren.base` setting, else the default branch, else picked. */
async function chooseBase(repository: string, placeholder: string): Promise<string | undefined> {
  const setting = vscode.workspace.getConfiguration("sundgren").get<string>("base", "").trim();
  return (
    (setting === "" ? undefined : setting) ??
    (await defaultBranch(repository)) ??
    (await pickBranch(repository, placeholder))
  );
}

/**
History's base: as `chooseBase`, but the local branch of a remote default when there is one
(`origin/main` → `main`): History is about your branches, Containment about the shared one.
*/
async function historyBase(repository: string): Promise<string | undefined> {
  const base = await chooseBase(repository, "Base: the reference line");
  const slash = base?.indexOf("/") ?? -1;
  if (base === undefined || slash < 0) {
    return base;
  }
  const local = base.slice(slash + 1);
  const found = await branches(repository);
  return found.some((branch) => branch.name === local) ? local : base;
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

/** Branches shown besides the base, by default. */
const DEFAULT_BRANCHES = 12;
/** Base commits read first, and added by each "Load older", by default. */
const DEFAULT_PAGE = 50;
/** The first view's outcome when the panel closed or started another map first. */
const STOPPED = Symbol("stopped");

/** `sundgren graph` arguments for `view`. */
function graphArguments(
  repository: string,
  base: string,
  pins: readonly string[],
  view: View,
): string[] {
  const patterns = vscode.workspace
    .getConfiguration("sundgren")
    .get<string[]>("protected", ["main", "master"]);
  return [
    "graph",
    "--repo",
    repository,
    "--base",
    base,
    ...(view.like === undefined ? ["--show", view.show] : ["--like", view.like]),
    "--branches",
    String(view.branches),
    "--commits",
    String(view.commits),
    ...patterns.flatMap((pattern) => ["--protected", pattern]),
    ...[...pins, ...view.pins].flatMap((pin) => ["--pin", pin]),
    ...view.hides.flatMap((name) => ["--hide", name]),
    ...(view.since === undefined ? [] : ["--since", String(view.since)]),
    ...(view.until === undefined ? [] : ["--until", String(view.until)]),
    "--fuzzy",
    String(FUZZY_THRESHOLD),
    "--format",
    "json",
  ];
}

/** The `sundgren.historyPage` setting: base commits per page. */
function historyPage(): number {
  const page = vscode.workspace.getConfiguration("sundgren").get<number>("historyPage", 0);
  return Number.isSafeInteger(page) && page >= 1 ? page : DEFAULT_PAGE;
}

/** What a run of `sundgren graph` gives the page: a graph, or why there is none. */
type Outcome =
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

function readGraph(result: RunResult): Outcome {
  let json: unknown;
  try {
    json = JSON.parse(result.stdout);
  } catch {
    json = undefined;
  }
  const graph = result.code === 0 ? parseGraph(json) : undefined;
  if (graph !== undefined) {
    return { type: "graph", graph };
  }
  const stderr = result.stderr.trim().replace(/^sundgren: /u, "");
  let message = stderr === "" ? `sundgren exited with ${String(result.code)}` : stderr;
  if (result.code === 0) {
    message = "sundgren printed something that isn't a sundgren.graph/3 document.";
  }
  return { type: "error", message };
}

/** The first view of `sundgren.showHistory`, from its arguments and the settings. */
function firstView(arguments_: ShowArguments | undefined): View {
  const like = arguments_?.like;
  const branches = arguments_?.branches ?? DEFAULT_BRANCHES;
  return {
    show: like === undefined ? (arguments_?.show ?? "local") : "like",
    ...(like !== undefined && { like }),
    branches: Math.min(MAX_BRANCHES, Math.max(0, branches)),
    commits: historyPage(),
    pins: [],
    hides: [],
  };
}

/**
The map of branches around `base` in the history panel. Every view the page asks for (another
show mode, branch count, more history) runs `sundgren graph` again, nothing is cached. Resolves
once the first map is drawn.
*/
async function showHistory(
  context: vscode.ExtensionContext,
  arguments_: ShowArguments | undefined,
): Promise<ShowResult> {
  const repository = await chooseRepository(arguments_?.repository);
  if (typeof repository !== "string") {
    return repository;
  }
  const base = arguments_?.base ?? (await historyBase(repository));
  if (base === undefined) {
    return CANCELLED;
  }
  const binary = findBinary(context, repository);
  if ("kind" in binary) {
    return binary;
  }
  const pins = arguments_?.pins ?? [];
  const first = firstView(arguments_);
  let current = first;

  // The first view's outcome goes to the command's result; later ones only to the page.
  const outcomes = new vscode.EventEmitter<Outcome | typeof STOPPED>();
  const firstOutcome = new Promise<Outcome | typeof STOPPED>((resolve) => {
    const listener = outcomes.event((outcome) => {
      listener.dispose();
      resolve(outcome);
    });
  });
  let isFirst = true;
  const didFinish = (outcome: Outcome | typeof STOPPED): boolean => {
    if (!isFirst) {
      return false;
    }
    isFirst = false;
    outcomes.fire(outcome);
    return true;
  };

  let running: RunHandle | undefined;
  const run = async (view: View): Promise<void> => {
    running?.cancel();
    current = view;
    const handle = runSundgren(
      binary.path,
      graphArguments(repository, base, pins, view),
      repository,
      (chunk) => {
        const line = chunk.trim().split("\n").at(-1) ?? "";
        if (running === handle && line !== "") {
          panel.post({ type: "running", view, progress: line.replace(/^sundgren: /u, "") });
        }
      },
    );
    running = handle;
    panel.post({ type: "running", view });
    const result = await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Window,
        title: `sundgren: mapping branches around ${base}`,
      },
      () => handle.done,
    );
    if (running !== handle || result.isCancelled) {
      return;
    }
    running = undefined;
    const outcome = readGraph(result);
    // The first is posted by the command, once it listens for the page drawing it.
    if (!didFinish(outcome)) {
      panel.post(outcome);
    }
  };
  const choose = async (control: Control): Promise<void> => {
    if (control === "base") {
      const picked = await pickBranch(repository, `Base: the reference line (now ${base})`);
      if (picked !== undefined) {
        await vscode.commands.executeCommand("sundgren.showHistory", {
          repository,
          base: picked,
          show: current.show,
          ...(current.like !== undefined && { like: current.like }),
          branches: current.branches,
        } satisfies ShowArguments);
      }
    } else if (control === "like") {
      const picked = await pickBranch(
        repository,
        "Like: show the branches whose work is most found in this one",
        [base],
      );
      if (picked !== undefined) {
        await run({ ...current, show: "like", like: picked });
      }
    }
  };
  const panel = HistoryPanel.open(context.extensionUri, `History: ${base}`, {
    view: (view) => {
      void run(view);
    },
    choose: (control) => {
      void choose(control);
    },
    select: () => {
      // Commits and lanes are clickable, but there are no actions yet.
    },
  });
  const stop = panel.onStop(() => {
    running?.cancel();
    didFinish(STOPPED);
    stop.dispose();
  });
  panel.post({
    type: "setup",
    base,
    view: first,
    step: historyPage(),
    states: await branchStates(repository),
    meta: await meta(context, binary),
  });
  void run(first);
  const outcome = await firstOutcome;
  outcomes.dispose();
  if (outcome === STOPPED) {
    return CANCELLED;
  }
  if (outcome.type === "error") {
    panel.post(outcome);
    return failed(outcome.message);
  }
  const rendered = panel.rendered();
  panel.post(outcome);
  const { show } = outcome.graph;
  return {
    kind: "shown",
    base,
    show: show.mode,
    ...(show.like !== undefined && { like: show.like.name }),
    lanes: outcome.graph.lanes.length,
    commits: outcome.graph.commits.length,
    binary: binary.path,
    rendered: await rendered,
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
    (await chooseBase(
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
    history: (branch) => {
      void vscode.commands.executeCommand("sundgren.showHistory", {
        repository,
        base,
        like: target,
        pins: branch === undefined ? [] : [branch],
      } satisfies ShowArguments);
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

/** How long a build has to settle before reloading: it writes several files. */
const RELOAD_DELAY_MS = 300;

/**
In development (an Extension Development Host), follow new builds: reload the open pages in place
when a page's script or styles change, and the whole window when the extension itself does.
*/
function reloadOnBuild(context: vscode.ExtensionContext): vscode.Disposable[] {
  if (context.extensionMode !== vscode.ExtensionMode.Development) {
    return [];
  }
  const extensionFile = vscode.Uri.joinPath(context.extensionUri, "dist", "extension.js");
  // Every build rewrites every file, so the extension counts as changed only if its bytes did.
  const running = vscode.workspace.fs.readFile(extensionFile);
  let timer: NodeJS.Timeout | undefined;
  const changed = (): void => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      void (async () => {
        const [before, now] = await Promise.all([
          running,
          vscode.workspace.fs.readFile(extensionFile),
        ]);
        if (Buffer.compare(before, now) === 0) {
          reloadPages();
        } else {
          await vscode.commands.executeCommand("workbench.action.reloadWindow");
        }
      })();
    }, RELOAD_DELAY_MS);
  };
  const watchers = [
    vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(vscode.Uri.joinPath(context.extensionUri, "dist"), "*.js"),
    ),
    vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(vscode.Uri.joinPath(context.extensionUri, "media"), "*.css"),
    ),
  ];
  return [
    ...watchers,
    ...watchers.flatMap((watcher) => [watcher.onDidChange(changed), watcher.onDidCreate(changed)]),
    {
      dispose: () => {
        clearTimeout(timer);
      },
    },
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
    ...reloadOnBuild(context),
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
