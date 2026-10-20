// Finding the sundgren binary and running it, with a cancel that really stops it. No VS Code.
//
// Cancel sends sundgren SIGTERM: it stops its test commands (each in its own process group),
// removes its worktrees and exits, normally within about a second. sundgren runs as the leader of
// its own process group; once it has exited after a cancel, anything left in that group is killed.
// If sundgren is still running after KILL_AFTER_MS, the group gets SIGKILL; on Linux its direct
// children then die with it (sundgren sets PR_SET_PDEATHSIG).
/* eslint-disable security/detect-non-literal-fs-filename -- paths come from the workspace and settings */

import { spawn } from "node:child_process";
import { accessSync, constants, readFileSync } from "node:fs";
import path from "node:path";

export const KILL_AFTER_MS = 5000;

export type BinarySource = "setting" | "bundled" | "venv" | "path";

export interface Binary {
  readonly path: string;
  readonly source: BinarySource;
}

export function isExecutable(file: string): boolean {
  try {
    accessSync(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

// The sundgren to run: the `sundgren.path` setting, else the bundled binary, else the project's
// `.venv`, else `PATH`. A setting that doesn't point at an executable is an error, not a reason
// to fall back silently.
export function locateSundgren(options: {
  readonly setting: string;
  readonly bundled: string;
  readonly repository: string;
  readonly pathEnvironment: string | undefined;
}): Binary | { readonly error: string } | undefined {
  const setting = options.setting.trim();
  if (setting !== "") {
    return path.isAbsolute(setting) && isExecutable(setting)
      ? { path: setting, source: "setting" }
      : { error: `sundgren.path is set to "${setting}", which is not an executable file.` };
  }
  if (isExecutable(options.bundled)) {
    return { path: options.bundled, source: "bundled" };
  }
  const venv = path.join(options.repository, ".venv", "bin", "sundgren");
  if (isExecutable(venv)) {
    return { path: venv, source: "venv" };
  }
  const directories = (options.pathEnvironment ?? "").split(path.delimiter);
  for (const directory of directories) {
    const candidate = path.join(directory, "sundgren");
    if (path.isAbsolute(directory) && isExecutable(candidate)) {
      return { path: candidate, source: "path" };
    }
  }
  return undefined;
}

export interface RunResult {
  /** Exit code, or undefined if it ended by a signal. */
  readonly code: number | undefined;
  readonly signal: NodeJS.Signals | undefined;
  readonly stdout: string;
  readonly stderr: string;
  readonly isCancelled: boolean;
  /** True if the group had to be killed because sundgren didn't stop in time after cancel. */
  readonly isKilled: boolean;
}

export interface RunHandle {
  readonly done: Promise<RunResult>;
  /** Ask sundgren to stop. Safe to call more than once and after it has ended. */
  cancel(): void;
}

export function runSundgren(
  binary: string,
  commandArguments: readonly string[],
  cwd: string,
  onStderr?: (chunk: string) => void,
): RunHandle {
  const child = spawn(binary, [...commandArguments], {
    cwd,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  let isCancelled = false;
  let isKilled = false;
  let isExited = false;
  let killTimer: NodeJS.Timeout | undefined;
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
    onStderr?.(chunk);
  });

  const killGroup = (): void => {
    if (child.pid === undefined) {
      return;
    }
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch {
      // The group is already gone.
    }
  };

  const done = new Promise<RunResult>((resolve) => {
    child.on("error", (error) => {
      isExited = true;
      resolve({
        code: 127,
        signal: undefined,
        stdout,
        stderr: `${stderr}${error.message}\n`,
        isCancelled,
        isKilled,
      });
    });
    // `exit` fires when sundgren itself ends; `close` only once every holder of its output pipes
    // is gone, which a stray test process can delay. So react to `exit`, resolve on `close`.
    child.on("exit", () => {
      isExited = true;
      if (killTimer !== undefined) {
        clearTimeout(killTimer);
      }
      if (isCancelled) {
        killGroup();
      }
    });
    child.on("close", (code, signal) => {
      resolve({
        code: code ?? undefined,
        signal: signal ?? undefined,
        stdout,
        stderr,
        isCancelled,
        isKilled,
      });
    });
  });

  return {
    done,
    cancel(): void {
      if (isExited || isCancelled) {
        return;
      }
      isCancelled = true;
      child.kill("SIGTERM");
      killTimer = setTimeout(() => {
        if (isExited) {
          return;
        }
        isKilled = true;
        killGroup();
      }, KILL_AFTER_MS);
    },
  };
}

/** The parsed JSON in `file`, or undefined if it's missing or not JSON. */
export function readJson(file: string): unknown {
  try {
    return JSON.parse(readFileSync(file, "utf8")) as unknown;
  } catch {
    return undefined;
  }
}
