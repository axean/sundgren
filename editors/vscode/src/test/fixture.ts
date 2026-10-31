// Small git repositories for the tests, made with the git on PATH.
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

// Fixed identity and dates; the user's global and system config are ignored.
const GIT_ENV = {
  ...process.env,
  GIT_CONFIG_GLOBAL: "/dev/null",
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "Ada",
  GIT_AUTHOR_EMAIL: "ada@example.com",
  GIT_COMMITTER_NAME: "Ada",
  GIT_COMMITTER_EMAIL: "ada@example.com",
  GIT_AUTHOR_DATE: "2026-01-01T00:00:00Z",
  GIT_COMMITTER_DATE: "2026-01-01T00:00:00Z",
};

export function git(repository: string, ...gitArguments: string[]): string {
  return execFileSync("git", ["-C", repository, ...gitArguments], {
    env: GIT_ENV,
    encoding: "utf8",
  });
}

/** main: a - b - m, where m merges feature (a - f); tag v1 on m. */
export function makeRepository(): string {
  const parent = path.join(tmpdir(), "sundgren-fixtures");
  mkdirSync(parent, { recursive: true });
  const repository = mkdtempSync(path.join(parent, "repo-"));
  git(repository, "init", "-q", "-b", "main");
  git(repository, "commit", "-q", "--allow-empty", "-m", "a");
  git(repository, "switch", "-q", "-c", "feature");
  git(repository, "commit", "-q", "--allow-empty", "-m", "f");
  git(repository, "switch", "-q", "main");
  git(repository, "commit", "-q", "--allow-empty", "-m", "b");
  git(repository, "merge", "-q", "--no-ff", "-m", "m", "feature");
  git(repository, "tag", "v1");
  return repository;
}

function commitFile(repository: string, file: string, text: string, message: string): void {
  writeFileSync(path.join(repository, file), text);
  git(repository, "add", "--", file);
  git(repository, "commit", "-q", "-m", message);
}

function code(name: string): string {
  return Array.from(
    { length: 10 },
    (_, index) => `${name}_${String(index)} = compute_${name}(${String(index)})\n`,
  ).join("");
}

/** main has app.py; dev adds one.py and two.py; extract adds one.py; other adds three.py. */
export function makeContainmentRepository(): string {
  const parent = path.join(tmpdir(), "sundgren-fixtures");
  mkdirSync(parent, { recursive: true });
  const repository = mkdtempSync(path.join(parent, "contain-"));
  git(repository, "init", "-q", "-b", "main");
  commitFile(repository, "app.py", "def main():\n    pass\n", "start");
  git(repository, "switch", "-q", "-c", "dev");
  commitFile(repository, "one.py", code("one"), "one");
  commitFile(repository, "two.py", code("two"), "two");
  git(repository, "switch", "-q", "-c", "extract", "main");
  commitFile(repository, "one.py", code("one"), "one");
  git(repository, "switch", "-q", "-c", "other", "main");
  commitFile(repository, "three.py", code("three").replaceAll("compute", "fetch"), "three");
  git(repository, "switch", "-q", "main");
  return repository;
}
