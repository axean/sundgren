// Files at a commit as read-only documents (`git show oid:path`), for VS Code's diff views.

import * as vscode from "vscode";

import { git } from "./git.js";
import type { Revision } from "./messages.js";

export const REVISION_SCHEME = "sundgren-rev";

/** A document for `revision` in `repository`. The path is kept, so VS Code knows the language. */
export function revisionUri(repository: string, revision: Revision): vscode.Uri {
  return vscode.Uri.from({
    scheme: REVISION_SCHEME,
    path: `/${revision.path}`,
    query: new URLSearchParams({ repository, oid: revision.oid }).toString(),
  });
}

/** Serves `sundgren-rev:` documents; a file that isn't there at that commit is empty. */
export class RevisionProvider implements vscode.TextDocumentContentProvider {
  async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
    const query = new URLSearchParams(uri.query);
    const repository = query.get("repository");
    const oid = query.get("oid");
    if (repository === null || oid === null || !/^[\da-f]{4,64}$/u.test(oid)) {
      return "";
    }
    const result = await git(repository, ["show", `${oid}:${uri.path.slice(1)}`]);
    return result.code === 0 ? result.stdout : "";
  }
}
