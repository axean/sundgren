// Text for the history page. No DOM.

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
