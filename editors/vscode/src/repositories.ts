// Which repositories the commands can work in: those of the open folders, the ones below them,
// their submodules, and the active file's. No VS Code.

import type { Dirent } from "node:fs";
import { readdir } from "node:fs/promises";
import path from "node:path";

/**
The repositories found for one open folder (absolute paths), labelled from `root`: the folder's
repository if it is in one, else the folder.
*/
export interface Found {
  readonly root: string;
  readonly repositories: readonly string[];
}

/** Folders not searched: hidden ones, and dependencies and build output. */
const SKIPPED = new Set(["node_modules", "target"]);

/**
The repositories in folders below `folder`, at most `depth` levels down, sorted. A repository is a
folder with a `.git` folder: a `.git` file is a worktree or a submodule, which belongs to another
repository. Symbolic links are not followed.
*/
export async function repositoriesBelow(folder: string, depth: number): Promise<string[]> {
  const found: string[] = [];
  const visit = async (directory: string, level: number): Promise<void> => {
    let entries: Dirent[];
    try {
      // eslint-disable-next-line security/detect-non-literal-fs-filename -- the open folders and folders below them
      entries = await readdir(directory, { withFileTypes: true });
    } catch {
      return;
    }
    if (level > 0 && entries.some((entry) => entry.name === ".git" && entry.isDirectory())) {
      found.push(directory);
    }
    if (level < depth) {
      await Promise.all(
        entries
          .filter(
            (entry) =>
              entry.isDirectory() && !entry.name.startsWith(".") && !SKIPPED.has(entry.name),
          )
          .map((entry) => visit(path.join(directory, entry.name), level + 1)),
      );
    }
  };
  await visit(folder, 0);
  return found.toSorted((x, y) => x.localeCompare(y));
}

/** A repository to pick from. */
export interface Choice {
  readonly path: string;
  /** Its folder, under its open folder's repository (or the folder): `meta/local/sundgren`. */
  readonly label: string;
  /** The one the commands work in now. */
  readonly isCurrent: boolean;
  /** The active file's. */
  readonly isActive: boolean;
}

function label(root: string, repository: string): string {
  const name = path.basename(root);
  return root === repository
    ? name
    : [name, ...path.relative(root, repository).split(path.sep)].join("/");
}

/**
Every repository found, each once: the current one first, then the active file's, then each open
folder's, in the order found.
*/
export function repositoryChoices(
  found: readonly Found[],
  active: string | undefined,
  current: string | undefined,
): Choice[] {
  const labels = new Map<string, string>();
  for (const { root, repositories } of found) {
    for (const repository of repositories) {
      if (!labels.has(repository)) {
        labels.set(repository, label(root, repository));
      }
    }
  }
  for (const extra of [active, current]) {
    if (extra !== undefined && !labels.has(extra)) {
      labels.set(extra, path.basename(extra));
    }
  }
  const rank = (repository: string): number => {
    if (repository === current) {
      return 0;
    }
    return repository === active ? 1 : 2;
  };
  return [...labels]
    .map(([repository, name]) => ({
      path: repository,
      label: name,
      isCurrent: repository === current,
      isActive: repository === active,
    }))
    .toSorted((a, b) => rank(a.path) - rank(b.path));
}
