import type { MacroChord } from "./types";

/** Windows key names that do not map to a single printable character. */
const CODE_KEYS: Record<string, string> = {
  Space: "space",
  Enter: "enter",
  NumpadEnter: "enter",
  Tab: "tab",
  Backspace: "backspace",
  Delete: "delete",
  Insert: "insert",
  Home: "home",
  End: "end",
  PageUp: "pageup",
  PageDown: "pagedown",
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Semicolon: ";",
  Equal: "=",
  Comma: ",",
  Minus: "-",
  Period: ".",
  Slash: "/",
  Backquote: "`",
  BracketLeft: "[",
  Backslash: "\\",
  BracketRight: "]",
  Quote: "'",
  NumpadMultiply: "numpadmultiply",
  NumpadAdd: "numpadadd",
  NumpadSubtract: "numpadsubtract",
  NumpadDecimal: "numpaddecimal",
  NumpadDivide: "numpaddivide",
};

/** Normalize a keyboard event to the key name the macro runtime expects. */
export function normalizeShortcutKey(event: KeyboardEvent): string | null {
  const code = event.code;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5).toLowerCase();
  if (/^F([1-9]|1[0-2])$/.test(code)) return code.toLowerCase();
  if (/^Numpad[0-9]$/.test(code)) return code.toLowerCase();
  const mapped = CODE_KEYS[code];
  if (mapped) return mapped;
  // Modifier-only presses have a multi-character key name and are ignored.
  return event.key.length === 1 ? event.key.toLowerCase() : null;
}

/**
 * Read a chord from a keydown event. Returns null while only modifier keys are
 * held down, so the user can keep holding them until the real key arrives.
 */
export function chordFromKeyboardEvent(event: KeyboardEvent): MacroChord | null {
  const key = normalizeShortcutKey(event);
  if (!key) return null;
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("control");
  if (event.altKey) modifiers.push("alt");
  if (event.shiftKey) modifiers.push("shift");
  if (event.metaKey) modifiers.push("win");
  return { modifiers, key };
}
