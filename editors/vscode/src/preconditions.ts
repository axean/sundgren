// Everything checked before a pluck runs, in order; the first failure stops it. No VS Code: the
// editor facts come in as plain values. sundgren's `run --check` checks the rest (the patch
// applies, the preset resolves, its tools are installed).
/* eslint-disable security/detect-non-literal-fs-filename -- paths come from the open workspace */

import { existsSync } from "node:fs";
import path from "node:path";

import {
  changedTrackedFiles,
  headSha,
  headText,
  isTracked,
  operationInProgress,
  repositoryPath,
  repositoryRoot,
} from "./git.js";
import { findTests, type FoundTest, type Language } from "./tests.js";

export interface PluckRequest {
  readonly file: string;
  readonly language: Language;
  /** The test as the code lens saw it. */
  readonly test: FoundTest;
  /** The editor's current text of the file. */
  readonly documentText: string;
  readonly isDocumentDirty: boolean;
  readonly isWorkspaceTrusted: boolean;
  readonly platform: NodeJS.Platform;
  /** `python.testing.cwd`, if set. */
  readonly pythonTestingCwd?: string | undefined;
  /** `python.testing.pytestArgs`, if set. */
  readonly pytestArgs?: readonly unknown[] | undefined;
  /** `rust-analyzer.linkedProjects`, if set. */
  readonly linkedProjects?: readonly unknown[] | undefined;
}

export interface Ready {
  readonly isOk: true;
  readonly repository: string;
  /** The file's repository-relative path. */
  readonly path: string;
  /** The file's text at HEAD, which the patch is built from. */
  readonly text: string;
  readonly test: FoundTest;
}

export interface Refused {
  readonly isOk: false;
  readonly message: string;
}

const MARKERS: Readonly<Record<Language, readonly string[]>> = {
  python: ["pyproject.toml", "setup.cfg", "pytest.ini"],
  rust: ["Cargo.toml"],
};

function refuse(message: string): Refused {
  return { isOk: false, message };
}

/** The nearest directory from `directory` up to `repository` that holds one of `markers`. */
function projectDirectory(
  directory: string,
  repository: string,
  markers: readonly string[],
): string | undefined {
  let current = directory;
  for (;;) {
    if (markers.some((m) => existsSync(path.join(current, m)))) {
      return current;
    }
    if (current === repository || path.dirname(current) === current) {
      return undefined;
    }
    current = path.dirname(current);
  }
}

/** Checks on the editor and the platform, before touching git. */
function editorProblem(request: PluckRequest): string | undefined {
  if (request.platform === "win32") {
    return "sundgren doesn't support Windows yet.";
  }
  if (!request.isWorkspaceTrusted) {
    return "Plucking a test runs the project's tests; trust this workspace first.";
  }
  return request.isDocumentDirty
    ? "Save the file first: sundgren measures the last commit, not unsaved edits."
    : undefined;
}

/** Checks that the repository is in a state where "the last commit" is well defined. */
async function repositoryProblem(repository: string): Promise<string | undefined> {
  if ((await headSha(repository)) === undefined) {
    return "This repository has no commits yet; commit first.";
  }
  const operation = await operationInProgress(repository);
  if (operation !== undefined) {
    return `Finish or abort ${operation} first.`;
  }
  const changed = await changedTrackedFiles(repository);
  if (changed === undefined) {
    return "Couldn't read the repository's status with git.";
  }
  if (changed.length > 0) {
    const sample = changed.slice(0, 3).join(", ");
    const more = changed.length > 3 ? ` and ${String(changed.length - 3)} more` : "";
    return `Commit or stash your changes first (${sample}${more}): sundgren compares against the last commit.`;
  }
  return undefined;
}

/** The file's committed text, or why the file can't be plucked from. */
async function committedText(
  repository: string,
  request: PluckRequest,
): Promise<string | { path: string; text: string }> {
  const file = repositoryPath(repository, request.file);
  if (file === undefined || !(await isTracked(repository, file))) {
    return "This file isn't committed; sundgren can only pluck tests that are in the last commit.";
  }
  const text = await headText(repository, file);
  if (text === undefined || text !== request.documentText) {
    return "The file differs from the last commit; commit it first.";
  }
  return text.includes("\r")
    ? "Files with Windows (CRLF) line endings aren't supported yet."
    : { path: file, text };
}

export async function checkPreconditions(request: PluckRequest): Promise<Ready | Refused> {
  const editor = editorProblem(request);
  if (editor !== undefined) {
    return refuse(editor);
  }
  const repository = await repositoryRoot(path.dirname(request.file));
  if (repository === undefined) {
    return refuse(
      "This file isn't in a git repository; sundgren compares two checkouts of a commit.",
    );
  }
  const state = await repositoryProblem(repository);
  if (state !== undefined) {
    return refuse(state);
  }
  const committed = await committedText(repository, request);
  if (typeof committed === "string") {
    return refuse(committed);
  }
  const test = findTests(request.language, committed.text).find(
    (t) => t.line === request.test.line && t.name === request.test.name,
  );
  if (test === undefined) {
    const where = `line ${String(request.test.line + 1)}`;
    return refuse(`Couldn't find ${request.test.name} at ${where} in the last commit.`);
  }
  const layout = layoutProblem(request, repository);
  return layout === undefined
    ? { isOk: true, repository, path: committed.path, text: committed.text, test }
    : refuse(layout);
}

/** Whether the project and VS Code's test settings say the tests run somewhere sundgren doesn't. */
function layoutProblem(request: PluckRequest, repository: string): string | undefined {
  const markers = MARKERS[request.language];
  const project = projectDirectory(path.dirname(request.file), repository, markers);
  if (project !== repository) {
    const where =
      project === undefined
        ? "no project file was found"
        : `it is in ${path.relative(repository, project)}/`;
    return `sundgren runs tests at the repository root, but this test's project isn't there (${where}). Projects in subfolders aren't supported yet.`;
  }
  return request.language === "python"
    ? pythonSettingsProblem(request, repository)
    : rustSettingsProblem(request);
}

function pythonSettingsProblem(request: PluckRequest, repository: string): string | undefined {
  const cwd = request.pythonTestingCwd?.trim() ?? "";
  const isRoot =
    cwd === "" ||
    cwd === "${workspaceFolder}" ||
    path.resolve(repository, cwd) === path.resolve(repository);
  if (!isRoot) {
    return `python.testing.cwd is set to "${cwd}"; sundgren only runs tests at the repository root for now.`;
  }
  const options = (request.pytestArgs ?? []).filter(
    (a) => typeof a !== "string" || a.startsWith("-"),
  );
  return options.length > 0
    ? `python.testing.pytestArgs has options (${options.map(String).join(" ")}); sundgren can't pass them yet.`
    : undefined;
}

function rustSettingsProblem(request: PluckRequest): string | undefined {
  return (request.linkedProjects ?? []).length > 0
    ? "rust-analyzer.linkedProjects is set, so the crate may not be at the repository root; not supported yet."
    : undefined;
}
