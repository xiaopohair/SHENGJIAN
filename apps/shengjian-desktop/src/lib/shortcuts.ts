const SHORTCUT_KEY_LABELS: Record<string, string> = {
  control: "Ctrl",
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Win",
  meta: "Win",
  space: "Space",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  escape: "Esc",
};

export function formatShortcutKey(key: string): string {
  const normalized = key.trim();
  const mapped = SHORTCUT_KEY_LABELS[normalized.toLowerCase()];
  if (mapped) return mapped;
  if (/^f([1-9]|1[0-2])$/i.test(normalized)) return normalized.toUpperCase();
  if (normalized.length === 1) return normalized.toUpperCase();
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

export function formatShortcut(keys: readonly string[]): string {
  return keys.map(formatShortcutKey).join("+");
}
