// Finding the sundgren binary and running it, with a cancel that really stops it. No VS Code.
//
// sundgren runs as the leader of its own process group. Cancel sends it SIGTERM: sundgren stops
// its test commands, removes its worktrees and exits. If it is still running after KILL_AFTER_MS,
// the whole group gets SIGKILL, so the test processes stop even if sundgren's own handling fails.

import { spawn } from "node:child_process";
import { accessSync, constants, readFileSync } from "node:fs";
import { delimiter, isAbsolute, join } from "node:path";

export const KILL_AFTER_MS = 5000;

export type BinarySource = "setting" | "bundled" | "venv" | "path";

export interface Binary {
  readonly path: string;
  readonly source: BinarySource;
}

export function isExecutable(path: string): boolean {
  try {
    accessSync(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/**
 * The sundgren to run: the `sundgren.path` setting, else the bundled binary, else the
 * project's `.venv`, else `PATH`. A setting that doesn't point at an executable is an error,
 * not a reason to fall back silently.
 */
export function locateSundgren(options: {
  readonly setting: string;
  readonly bundled: string;
  readonly repo: string;
  readonly pathEnv: string | undefined;
}): Binary | { readonly error: string } | undefined {
  const setting = options.setting.trim();
  if (setting !== "") {
    if (!isAbsolute(setting) || !isExecutable(setting)) {
      return { error: `sundgren.path is set to "${setting}", which is not an executable file` };
    }
    return { path: setting, source: "setting" };
  }
  if (isExecutable(options.bundled)) {
    return { path: options.bundled, source: "bundled" };
  }
  const venv = join(options.repo, ".venv", "bin", "sundgren");
  if (isExecutable(venv)) {
    return { path: venv, source: "venv" };
  }
  for (const dir of (options.pathEnv ?? "").split(delimiter)) {
    const candidate = join(dir, "sundgren");
    if (dir !== "" && isAbsolute(dir) && isExecutable(candidate)) {
      return { path: candidate, source: "path" };
    }
  }
  return undefined;
}

/** `[major, minor, patch]` from `sundgren --version` output like "sundgren 0.2.2". */
export function parseVersion(output: string): readonly [number, number, number] | undefined {
  const m = /^sundgren (\d+)\.(\d+)\.(\d+)\s*$/u.exec(output.trim());
  if (m === null) {
    return undefined;
  }
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

export function versionAtLeast(have: readonly number[], want: readonly number[]): boolean {
  for (let i = 0; i < Math.max(have.length, want.length); i += 1) {
    const a = have[i] ?? 0;
    const b = want[i] ?? 0;
    if (a !== b) {
      return a > b;
    }
  }
  return true;
}

export interface RunResult {
  /** Exit code, or undefined if it ended by a signal. */
  readonly code: number | undefined;
  readonly signal: NodeJS.Signals | undefined;
  readonly stdout: string;
  readonly stderr: string;
  readonly cancelled: boolean;
  /** True if the group had to be killed because sundgren didn't stop in time after cancel. */
  readonly killed: boolean;
}

export interface RunHandle {
  readonly done: Promise<RunResult>;
  /** Ask sundgren to stop. Safe to call more than once and after it has ended. */
  cancel(): void;
}

export function runSundgren(
  binary: string,
  args: readonly string[],
  cwd: string,
  onStderr?: (chunk: string) => void,
): RunHandle {
  const child = spawn(binary, [...args], { cwd, detached: true, stdio: ["ignore", "pipe", "pipe"] });
  let stdout = "";
  let stderr = "";
  let cancelled = false;
  let killed = false;
  let killTimer: NodeJS.Timeout | undefined;
  let exited = false;
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
    onStderr?.(chunk);
  });

  const signalGroup = (signal: NodeJS.Signals): void => {
    if (child.pid === undefined) {
      return;
    }
    try {
      process.kill(-child.pid, signal);
    } catch {
      // The group is already gone.
    }
  };

  const done = new Promise<RunResult>((resolve) => {
    child.on("error", (error) => {
      exited = true;
      resolve({ code: 127, signal: undefined, stdout, stderr: `${stderr}${error.message}\n`, cancelled, killed });
    });
    // `exit` fires when sundgren itself ends; `close` only once every holder of its output pipes
    // is gone, which a stray test process can delay. So react to `exit`, resolve on `close`.
    child.on("exit", () => {
      exited = true;
      if (killTimer !== undefined) {
        clearTimeout(killTimer);
      }
      if (cancelled) {
        // Anything left in sundgren's group after a cancel is a stray test process.
        signalGroup("SIGKILL");
      }
    });
    child.on("close", (code, signal) => {
      resolve({ code: code ?? undefined, signal: signal ?? undefined, stdout, stderr, cancelled, killed });
    });
  });

  return {
    done,
    cancel(): void {
      if (exited || cancelled) {
        return;
      }
      cancelled = true;
      try {
        child.kill("SIGTERM");
      } catch {
        // Already exited.
      }
      killTimer = setTimeout(() => {
        if (!exited) {
          killed = true;
          signalGroup("SIGKILL");
        }
      }, KILL_AFTER_MS);
    },
  };
}

/** The parsed JSON in `path`, or undefined if it's missing or not JSON. */
export function readJson(path: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf8")) as unknown;
  } catch {
    return undefined;
  }
}
