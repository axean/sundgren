// Text for the history page. No DOM.

import type { CommitStat } from "../git.js";

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/** Unix seconds as local `YYYY-MM-DD HH:MM`. */
export function formatTime(seconds: number): string {
  const d = new Date(seconds * 1000);
  const date = `${String(d.getFullYear())}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  return `${date} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Unix seconds as local `HH:MM`. */
export function clock(seconds: number): string {
  const d = new Date(seconds * 1000);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** The local day of unix seconds, to tell when the day changes. */
export function dayKey(seconds: number): string {
  return formatTime(seconds).slice(0, 10);
}

/** The time axis' day label: `Oct 30`, with the year (`Oct 30 '25`) if it isn't `year`. */
export function dayLabel(seconds: number, year: number): string {
  const d = new Date(seconds * 1000);
  const day = `${MONTHS[d.getMonth()] ?? ""} ${String(d.getDate())}`;
  return d.getFullYear() === year
    ? day
    : `${day} '${String(d.getFullYear() % 100).padStart(2, "0")}`;
}

/** A kind of change to a file, as the stats column colours it. */
export type FileKind = "added" | "modified" | "deleted";

/** The kinds of change that aren't zero, in the order added, modified, deleted. */
export function fileKinds(
  stat: Pick<CommitStat, "filesAdded" | "filesModified" | "filesDeleted">,
): [FileKind, number][] {
  const kinds: [FileKind, number][] = [
    ["added", stat.filesAdded],
    ["modified", stat.filesModified],
    ["deleted", stat.filesDeleted],
  ];
  return kinds.filter(([, count]) => count > 0);
}

/** `2 added, 3 modified, 1 deleted`, without the kinds that are zero. */
export function fileKindsText(kinds: readonly [FileKind, number][]): string {
  return kinds.map(([kind, count]) => `${String(count)} ${kind}`).join(", ");
}
