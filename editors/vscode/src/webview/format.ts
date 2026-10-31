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
