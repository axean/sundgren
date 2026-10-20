// Everything checked before a pluck runs, in order; the first failure stops it. No VS Code: the
// editor facts come in as plain values. sundgren's `run --check` checks the rest (the patch
// applies, the preset resolves, its tools are installed).

import { existsSync } from "node:fs";
import { dirname, join, relative } from "node:path";

import { changedTrackedFiles, headSha, headText, isTracked, operationInProgress, repoPath, repoRoot } from "./git.js";
import { findTests, type FoundTest, type Language } from "./tests.js";

export interface PluckRequest {
  readonly file: string;
  readonly language: Language;
  /** The test as the code lens saw it. */
  readonly test: FoundTest;
  /** The editor's current text of the file. */
  readonly documentText: string;
  readonly documentDirty: boolean;
  readonly workspaceTrusted: boolean;
  readonly platform: NodeJS.Platform;
  /** `python.testing.cwd`, if set. */
  readonly pythonTestingCwd?: string | undefined;
  /** `python.testing.pytestArgs`, if set. */
  readonly pytestArgs?: readonly unknown[] | undefined;
  /** `rust-analyzer.linkedProjects`, if set. */
  readonly linkedProjects?: readonly unknown[] | undefined;
}

export interface Ready {
  readonly ok: true;
  readonly repo: string;
  /** The file's repo-relative path. */
  readonly path: string;
  /** The file's text at HEAD, which the patch is built from. */
  readonly text: string;
  readonly test: FoundTest;
}

export interface Refused {
  readonly ok: false;
  readonly message: string;
}

const MARKERS: Readonly<Record<Language, readonly string[]>> = {
  python: ["pyproject.toml", "setup.cfg", "pytest.ini"],
  rust: ["Cargo.toml"],
};

function refuse(message: string): Refused {
  return { ok: false, message };
}

/** The nearest directory from `dir` up to `repo` that holds one of `markers`. */
function projectDir(dir: string, repo: string, markers: readonly string[]): string | undefined {
  let current = dir;
  for (;;) {
    if (markers.some((m) => existsSync(join(current, m)))) {
      return current;
    }
    if (current === repo || dirname(current) === current) {
      return undefined;
    }
    current = dirname(current);
  }
}

export async function checkPreconditions(req: PluckRequest): Promise<Ready | Refused> {
  if (req.platform === "win32") {
    return refuse("sundgren doesn't support Windows yet.");
  }
  if (!req.workspaceTrusted) {
    return refuse("Plucking a test runs the project's tests; trust this workspace first.");
  }
  if (req.documentDirty) {
    return refuse("Save the file first: sundgren measures the last commit, not unsaved edits.");
  }
  const repo = await repoRoot(dirname(req.file));
  if (repo === undefined) {
    return refuse("This file isn't in a git repository; sundgren compares two checkouts of a commit.");
  }
  if ((await headSha(repo)) === undefined) {
    return refuse("This repository has no commits yet; commit first.");
  }
  const operation = await operationInProgress(repo);
  if (operation !== undefined) {
    return refuse(`Finish or abort ${operation} first.`);
  }
  const changed = await changedTrackedFiles(repo);
  if (changed === undefined) {
    return refuse("Couldn't read the repository's status with git.");
  }
  if (changed.length > 0) {
    const sample = changed.slice(0, 3).join(", ");
    const more = changed.length > 3 ? ` and ${String(changed.length - 3)} more` : "";
    return refuse(
      `Commit or stash your changes first (${sample}${more}): sundgren compares against the last commit.`,
    );
  }
  const path = repoPath(repo, req.file);
  if (path === undefined || !(await isTracked(repo, path))) {
    return refuse("This file isn't committed; sundgren can only pluck tests that are in the last commit.");
  }
  const text = await headText(repo, path);
  if (text === undefined || text !== req.documentText) {
    return refuse("The file differs from the last commit; commit it first.");
  }
  if (text.includes("\r")) {
    return refuse("Files with Windows (CRLF) line endings aren't supported yet.");
  }
  const test = findTests(req.language, text).find((t) => t.line === req.test.line && t.name === req.test.name);
  if (test === undefined) {
    return refuse(`Couldn't find ${req.test.name} at line ${String(req.test.line + 1)} in the last commit.`);
  }
  const project = projectDir(dirname(req.file), repo, MARKERS[req.language]);
  if (project !== repo) {
    const where = project === undefined ? "no project file was found" : `it is in ${relative(repo, project)}/`;
    return refuse(
      `sundgren runs tests at the repository root, but this test's project isn't there (${where}). Projects in subfolders aren't supported yet.`,
    );
  }
  const layout = layoutProblem(req, repo);
  if (layout !== undefined) {
    return refuse(layout);
  }
  return { ok: true, repo, path, text, test };
}

/** VS Code settings that say the tests run somewhere sundgren doesn't. */
function layoutProblem(req: PluckRequest, repo: string): string | undefined {
  if (req.language === "python") {
    const cwd = req.pythonTestingCwd?.trim();
    if (cwd !== undefined && cwd !== "" && cwd !== "${workspaceFolder}" && join(repo, cwd) !== repo && cwd !== repo) {
      return `python.testing.cwd is set to "${cwd}"; sundgren only runs tests at the repository root for now.`;
    }
    const flags = (req.pytestArgs ?? []).filter((a) => typeof a !== "string" || a.startsWith("-"));
    if (flags.length > 0) {
      return `python.testing.pytestArgs has options (${flags.map(String).join(" ")}); sundgren can't pass them yet.`;
    }
  } else if ((req.linkedProjects ?? []).length > 0) {
    return "rust-analyzer.linkedProjects is set, so the crate may not be at the repository root; not supported yet.";
  }
  return undefined;
}
