// Which repositories the commands can work in: those of the open folders, their submodules, and
// the active file's. No VS Code.

import path from "node:path";

/** An open folder's repository and its checked-out submodules (absolute paths). */
export interface Found {
  readonly top: string;
  readonly submodules: readonly string[];
}

/** A repository to pick from. */
export interface Choice {
  readonly path: string;
  /** Its folder, under its open folder's repository for a submodule: `meta/local/sundgren`. */
  readonly label: string;
  /** The one the commands work in now. */
  readonly isCurrent: boolean;
  /** The active file's. */
  readonly isActive: boolean;
}

function label(top: string, repository: string): string {
  const name = path.basename(top);
  return top === repository
    ? name
    : [name, ...path.relative(top, repository).split(path.sep)].join("/");
}

/**
Every repository found, each once: the current one first, then the active file's, then each open
folder's repository followed by its submodules.
*/
export function repositoryChoices(
  found: readonly Found[],
  active: string | undefined,
  current: string | undefined,
): Choice[] {
  const labels = new Map<string, string>();
  for (const { top, submodules } of found) {
    for (const repository of [top, ...submodules]) {
      if (!labels.has(repository)) {
        labels.set(repository, label(top, repository));
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
