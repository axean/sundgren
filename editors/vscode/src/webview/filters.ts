// The History page's filters: the authors on the page, the branch groups of the branch list, and
// the calendar of the date filter. Builders take callbacks; the page keeps the state.

import type { Branch, Graph } from "../graph.js";
import { element } from "./common.js";
import { dayKey } from "./format.js";

/** Someone who wrote commits, by email. */
export interface Author {
  readonly email: string;
  /** The name on their newest commit here. */
  readonly name: string;
  /** Their commits on the page. */
  readonly commits: number;
}

/** The authors of the page's commits and of the listed branches' own commits, most commits first. */
export function authorsOf(graph: Graph): Author[] {
  const names = new Map<string, string>();
  const counts = new Map<string, number>();
  // Newest first, so the first name seen is the newest.
  for (const commit of graph.commits) {
    if (!names.has(commit.email)) {
      names.set(commit.email, commit.author);
    }
    counts.set(commit.email, (counts.get(commit.email) ?? 0) + 1);
  }
  for (const branch of graph.branches) {
    for (const email of branch.authors) {
      if (!names.has(email)) {
        names.set(email, email);
      }
    }
  }
  return [...names]
    .map(([email, name]) => ({ email, name, commits: counts.get(email) ?? 0 }))
    .toSorted((a, b) => b.commits - a.commits || a.name.localeCompare(b.name));
}

/** A group of the branch list: which of its branches (with their rank) belong to it. */
export interface Group {
  readonly label: string;
  readonly help: string;
  readonly has: (branch: Branch, rank: number) => boolean;
}

/** The branch list's groups: the first `first`, local, remote, and by the shown authors. */
export function groups(first: number, hiddenAuthors: ReadonlySet<string>): Group[] {
  const found: Group[] = [
    {
      label: "All",
      help: "Every branch in the list (as filtered): tick them all, or untick them all.",
      has: () => true,
    },
    {
      label: `First ${String(first)}`,
      help: `The first ${String(first)} by Show: tick them all, or untick them all.`,
      has: (_branch, rank) => rank < first,
    },
    {
      label: "Local",
      help: "Branches with a local branch: tick them all, or untick them all.",
      has: (branch) => branch.local,
    },
    {
      label: "Remote",
      help: "Branches only on a remote (no local branch): tick them all, or untick them all.",
      has: (branch) => !branch.local,
    },
  ];
  if (hiddenAuthors.size > 0) {
    found.push({
      label: "By shown authors",
      help: "Branches with commits of their own by the authors ticked in Authors.",
      has: (branch) => branch.authors.some((email) => !hiddenAuthors.has(email)),
    });
  }
  return found;
}

/** A day as unix seconds at its local start. */
function dayStart(year: number, month: number, day: number): number {
  return new Date(year, month, day).getTime() / 1000;
}

/** The interval of whole local days from `first` to `last` (unix seconds of any time in them). */
export function wholeDays(first: number, last: number): { since: number; until: number } {
  const [a, b] = first <= last ? [first, last] : [last, first];
  const start = new Date(a * 1000);
  const end = new Date(b * 1000);
  return {
    since: dayStart(start.getFullYear(), start.getMonth(), start.getDate()),
    until: dayStart(end.getFullYear(), end.getMonth(), end.getDate() + 1) - 1,
  };
}

/** The last `days` days, today included. */
export function lastDays(days: number, now: Date): { since: number; until: number } {
  const today = dayStart(now.getFullYear(), now.getMonth(), now.getDate());
  return wholeDays(today - (days - 1) * 86_400 + 3600, today + 3600);
}

const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** What the calendar shows and what it calls back. */
export interface CalendarOptions {
  /** Any time in the month shown. */
  readonly month: number;
  readonly since: number | undefined;
  readonly until: number | undefined;
  /** The first day picked, waiting for the last. */
  readonly pending: number | undefined;
  /** Days with commits on the page (`dayKey`). */
  readonly busy: ReadonlySet<string>;
  readonly onMonth: (month: number) => void;
  readonly onDay: (day: number) => void;
}

/** A month of days, Monday first: click the first day, then the last. */
export function calendar(options: CalendarOptions): HTMLElement {
  const shown = new Date(options.month * 1000);
  const year = shown.getFullYear();
  const month = shown.getMonth();
  const node = element("div", "calendar");

  const head = element("div", "calendar-head");
  const step = (text: string, help: string, delta: number): HTMLButtonElement => {
    const button = element("button", "calendar-step", text);
    button.type = "button";
    button.title = help;
    button.addEventListener("click", () => {
      options.onMonth(dayStart(year, month + delta, 1));
    });
    return button;
  };
  head.append(
    step("‹", "The month before", -1),
    element("span", "calendar-month", `${MONTH_NAMES.at(month) ?? ""} ${String(year)}`),
    step("›", "The month after", 1),
  );
  node.append(head);

  const grid = element("div", "calendar-grid");
  for (const name of ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]) {
    grid.append(element("span", "calendar-weekday", name));
  }
  // Monday is 0.
  const lead = (new Date(year, month, 1).getDay() + 6) % 7;
  for (let index = 0; index < lead; index++) {
    grid.append(element("span", "calendar-blank"));
  }
  const days = new Date(year, month + 1, 0).getDate();
  const today = dayKey(Date.now() / 1000);
  for (let day = 1; day <= days; day++) {
    grid.append(dayButton(options, dayStart(year, month, day), today));
  }
  node.append(grid);
  return node;
}

function dayButton(options: CalendarOptions, start: number, today: string): HTMLButtonElement {
  const key = dayKey(start);
  const button = element("button", "calendar-day", String(new Date(start * 1000).getDate()));
  button.type = "button";
  button.title = key;
  const end = start + 86_399;
  const isIn =
    options.since !== undefined &&
    options.until !== undefined &&
    end >= options.since &&
    start <= options.until;
  button.classList.toggle("in-range", isIn);
  button.classList.toggle(
    "edge",
    (options.since !== undefined && dayKey(options.since) === key) ||
      (options.until !== undefined && dayKey(options.until) === key),
  );
  button.classList.toggle(
    "pending",
    options.pending !== undefined && dayKey(options.pending) === key,
  );
  button.classList.toggle("busy", options.busy.has(key));
  button.classList.toggle("today", key === today);
  if (options.busy.has(key)) {
    button.title = `${key}: commits on the page`;
  }
  button.addEventListener("click", () => {
    options.onDay(start);
  });
  return button;
}
