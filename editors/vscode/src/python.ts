// Which Python interpreter a pluck uses: the one selected in Microsoft's Python extension, through
// its official API, else the project's `.venv`, else none (sundgren then uses `python` on PATH).
/* eslint-disable security/detect-non-literal-fs-filename -- paths come from the open workspace */

import { existsSync } from "node:fs";
import path from "node:path";

import { PythonExtension } from "@vscode/python-extension";
import * as vscode from "vscode";

export interface Interpreter {
  readonly path: string;
  readonly source: "python-extension" | "venv";
}

const PYTHON_EXTENSION_ID = "ms-python.python";

async function selectedInterpreter(file: vscode.Uri): Promise<string | undefined> {
  if (vscode.extensions.getExtension(PYTHON_EXTENSION_ID) === undefined) {
    return undefined;
  }
  try {
    const api = await PythonExtension.api();
    const selected = api.environments.getActiveEnvironmentPath(file);
    const environment = await api.environments.resolveEnvironment(selected);
    return environment?.executable.uri?.fsPath;
  } catch {
    // The extension is installed but not usable (e.g. disabled).
    return undefined;
  }
}

export async function findInterpreter(
  repository: string,
  file: vscode.Uri,
): Promise<Interpreter | undefined> {
  const selected = await selectedInterpreter(file);
  if (selected !== undefined && existsSync(selected)) {
    return { path: selected, source: "python-extension" };
  }
  const venv = path.join(repository, ".venv", "bin", "python");
  return existsSync(venv) ? { path: venv, source: "venv" } : undefined;
}
