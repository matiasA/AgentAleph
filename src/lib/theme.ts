/**
 * Tonos seleccionables. La base oscura es siempre la misma; cada tono sólo
 * cambia el acento (ver [data-accent] en app.css). El valor se persiste en
 * localStorage y se aplica al <html> vía data-accent.
 */

export type AccentId = "blue" | "indigo" | "emerald" | "amber" | "rose" | "green";

export interface Accent {
  id: AccentId;
  label: string;
  /** color representativo para el swatch del selector */
  swatch: string;
}

/** Acentos sobre la base "Anysphere Dark". El azul es el acento original de
 *  Cursor; los demás son tomados de la paleta de sintaxis de Anysphere. */
export const ACCENTS: Accent[] = [
  { id: "blue", label: "Zed", swatch: "#4d7fff" },
  { id: "indigo", label: "Lavanda", swatch: "#aaa0fa" },
  { id: "emerald", label: "Teal", swatch: "#83d6c5" },
  { id: "amber", label: "Ámbar", swatch: "#e3b341" },
  { id: "rose", label: "Rosa", swatch: "#e394a5" },
  { id: "green", label: "Verde", swatch: "#a8cc7c" },
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

/* ----------------------- Modo (oscuro / claro) ----------------------- */

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

/** Llamar una vez al arrancar, antes del primer render. */
export function initAccent() {
  applyAccent(getAccent());
  applyMode(getMode());
}
