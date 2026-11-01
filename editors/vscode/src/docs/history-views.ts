// Which of the history demo's views made in advance answers what the page asks for. No DOM.

import type { View } from "../messages.js";
import type { DemoGraph } from "./host.js";

const DAY = 86_400;

/** The Show mode a view was made with. */
export function modeOf(view: DemoGraph): string {
  return view.show[0] === "--like" ? "like" : (view.show[1] ?? "");
}

/** How alike two lists of hidden branches are: 1 for the same, 0 for nothing in common. */
export function alike(a: readonly string[], b: readonly string[]): number {
  const all = new Set([...a, ...b]);
  return all.size === 0 ? 1 : a.filter((name) => b.includes(name)).length / all.size;
}

/**
What a date interval shows of the demo history (`days`: its UTC days with commits): `any` with no
interval, else the days it covers, or `before`/`after` the history when it covers none.
*/
export function daysKey(
  days: readonly number[],
  since: number | undefined,
  until: number | undefined,
): string {
  if (since === undefined && until === undefined) {
    return "any";
  }
  const from = since ?? 0;
  const to = until ?? Number.MAX_SAFE_INTEGER;
  const covered = days.filter((day) => day <= to && day + DAY - 1 >= from);
  if (covered.length > 0) {
    return covered.join(",");
  }
  return to < (days[0] ?? 0) ? "before" : "after";
}

/** The view for `view`, and whether it is exactly that one (else the closest there is). */
export function closest(
  graphs: readonly DemoGraph[],
  days: readonly number[],
  view: View,
): { readonly view: DemoGraph | undefined; readonly isExact: boolean } {
  const wanted = daysKey(days, view.since, view.until);
  const keyOf = (made: DemoGraph): string =>
    made.dates === undefined ? "any" : daysKey(days, made.dates[0], made.dates[1]);
  const same = graphs.filter((made) => modeOf(made) === view.show);
  const pool = same.length > 0 ? same : graphs;
  // With a first day, the page is the days; else it is `commits` back.
  const exact = same.find(
    (made) =>
      keyOf(made) === wanted &&
      made.hides.length === view.hides.length &&
      alike(made.hides, view.hides) === 1 &&
      (view.since !== undefined || made.commits === view.commits),
  );
  if (exact !== undefined) {
    return { view: exact, isExact: true };
  }
  const score = (made: DemoGraph): number =>
    2 * (keyOf(made) === wanted ? 1 : 0) + alike(made.hides, view.hides);
  const best = Math.max(...pool.map((made) => score(made)));
  const nearest = pool
    .filter((made) => score(made) === best)
    .toSorted((a, b) => Math.abs(a.commits - view.commits) - Math.abs(b.commits - view.commits));
  return { view: nearest[0], isExact: false };
}
