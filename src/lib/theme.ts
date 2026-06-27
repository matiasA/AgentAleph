/**
 * Selectable accent tones. The dark base stays the same; each tone only
 * changes the accent color (see [data-accent] in app.css). The value is
 * persisted in localStorage and applied to <html> via data-accent.
 */

export type AccentId = "blue" | "indigo" | "emerald" | "amber" | "rose" | "green";

export interface Accent {
  id: AccentId;
  label: string;
  /** Representative color for the selector swatch. */
  swatch: string;
}

/** Accents over the "Anysphere Dark" base. Blue is Cursor's original accent;
 *  the rest come from the Anysphere syntax palette. */
export const ACCENTS: Accent[] = [
  { id: "blue", label: "Zed", swatch: "#4d7fff" },
  { id: "indigo", label: "Lavender", swatch: "#aaa0fa" },
  { id: "emerald", label: "Teal", swatch: "#83d6c5" },
  { id: "amber", label: "Amber", swatch: "#e3b341" },
  { id: "rose", label: "Rose", swatch: "#e394a5" },
  { id: "green", label: "Green", swatch: "#a8cc7c" },
];

const STORAGE_KEY = "aleph.accent";
const DEFAULT: AccentId = "blue";

function isAccent(v: string | null): v is AccentId {
  return !!v && ACCENTS.some((a) => a.id === v);
}

export function getAccent(): AccentId {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (isAccent(v)) return v;
  } catch {}
  return DEFAULT;
}

export function applyAccent(id: AccentId) {
  document.documentElement.setAttribute("data-accent", id);
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {}
}

/* ----------------------- Mode (dark / light) ----------------------- */

export type Mode = "dark" | "light";
const MODE_KEY = "aleph.mode";
const DEFAULT_MODE: Mode = "dark";

export function getMode(): Mode {
  try {
    const v = localStorage.getItem(MODE_KEY);
    if (v === "dark" || v === "light") return v;
  } catch {}
  return DEFAULT_MODE;
}

export function applyMode(mode: Mode) {
  document.documentElement.setAttribute("data-mode", mode);
  try {
    localStorage.setItem(MODE_KEY, mode);
  } catch {}
}

/** Call once on startup, before the first render. */
export function initAccent() {
  applyAccent(getAccent());
  applyMode(getMode());
}
