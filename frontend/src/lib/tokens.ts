/**
 * Design token constants for JavaScript/TypeScript consumers.
 *
 * The canonical source of truth for colours is `app.css` (CSS custom
 * properties).  This module re-exports the same values so component logic
 * that needs a colour at runtime (e.g. canvas drawing, dynamic SVG) can
 * import it directly instead of reading from `getComputedStyle`.
 *
 * Keep this in sync with `app.css` whenever tokens change.
 */

/** All ED design tokens, keyed by theme name. */
export const ED_TOKENS = {
  dark: {
    bg: "#0d1117",
    panel: "#161b22",
    elevated: "#1c2128",
    border: "#30363d",
    borderSubtle: "#21262d",
    text: "#e6edf3",
    textMuted: "#8b949e",
    textFaint: "#6e7681",
    accent: "#58a6ff",
    accentSoft: "rgba(88,166,255,0.12)",
    added: "#3fb950",
    addedBg: "rgba(63,185,80,0.10)",
    addedGutter: "rgba(63,185,80,0.30)",
    removed: "#f85149",
    removedBg: "rgba(248,81,73,0.10)",
    removedGutter: "rgba(248,81,73,0.30)",
    risky: "#d29922",
    riskyBg: "rgba(210,153,34,0.10)",
    selection: "rgba(88,166,255,0.18)",
    sx: {
      keyword: "#ff7b72",
      type: "#79c0ff",
      string: "#a5d6ff",
      number: "#79c0ff",
      comment: "#8b949e",
      anno: "#d2a8ff",
      method: "#d2a8ff",
      punct: "#e6edf3",
    },
  },
  light: {
    bg: "#ffffff",
    panel: "#f6f8fa",
    elevated: "#ffffff",
    border: "#d0d7de",
    borderSubtle: "#eaeef2",
    text: "#1f2328",
    textMuted: "#59636e",
    textFaint: "#8c959f",
    accent: "#0969da",
    accentSoft: "rgba(9,105,218,0.10)",
    added: "#1a7f37",
    addedBg: "#dafbe1",
    addedGutter: "#aceebb",
    removed: "#cf222e",
    removedBg: "#ffebe9",
    removedGutter: "#ff818266",
    risky: "#9a6700",
    riskyBg: "#fff8c5",
    selection: "rgba(9,105,218,0.10)",
    sx: {
      keyword: "#cf222e",
      type: "#953800",
      string: "#0a3069",
      number: "#0550ae",
      comment: "#6e7781",
      anno: "#8250df",
      method: "#8250df",
      punct: "#1f2328",
    },
  },
} as const;

/** Union of valid theme names. */
export type Theme = keyof typeof ED_TOKENS;

/** Token values for a single theme. */
export type ThemeTokens = (typeof ED_TOKENS)[Theme];
