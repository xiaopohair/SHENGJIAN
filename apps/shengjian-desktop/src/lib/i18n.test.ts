import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { hasTranslation, setLanguagePreference } from "./i18n";
import { profileDisplayName } from "./buttons";
import { CHORD_KEY_GROUPS, CHORD_MODIFIER_OPTIONS } from "./chordOptions";
import { hotkeyKeyLabel, MODIFIER_OPTIONS } from "./hotkey";

function sourceFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(full);
    return /\.(vue|ts)$/.test(entry.name) && !entry.name.endsWith(".test.ts") ? [full] : [];
  });
}

/** A TS string literal may escape quotes/backslashes; accept either spelling. */
function spellings(raw: string): string[] {
  return [raw, raw.replace(/\\'/g, "'").replace(/\\"/g, '"').replace(/\\\\/g, "\\")];
}

function missingKeys(
  files: string[],
  extract: (source: string) => string[],
): string[] {
  const missing = new Set<string>();
  for (const file of files) {
    for (const key of extract(fs.readFileSync(file, "utf8"))) {
      if (!spellings(key).some(hasTranslation)) missing.add(key);
    }
  }
  return [...missing].sort();
}

describe("i18n dictionary", () => {
  const srcRoot = path.resolve(process.cwd(), "src");
  const files = sourceFiles(srcRoot).filter((file) => !file.endsWith("i18n.ts"));

  it("covers every literal key passed to tr() / trDynamic()", () => {
    const missing = missingKeys(files, (source) =>
      [...source.matchAll(/\btr(?:Dynamic)?\(\s*(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)')/g)]
        .map((match) => match[1] ?? match[2])
        .filter(Boolean),
    );
    expect(missing).toEqual([]);
  });

  it("covers every Chinese literal fed to a PageHeader / SettingRow / EmptyState prop", () => {
    const missing = missingKeys(files, (source) =>
      [...source.matchAll(/(?<![:\w-])(?:aria-label|label|hint|description|eyebrow|title)="([^"]*)"/g)]
        .map((match) => match[1])
        .filter((value) => /[\u4e00-\u9fff]/.test(value) && !/[({`$]/.test(value)),
    );
    expect(missing).toEqual([]);
  });

  it("covers the labels rendered by the hotkey and chord pickers", () => {
    const labels = [
      ...MODIFIER_OPTIONS.map((option) => option.label),
      ...CHORD_MODIFIER_OPTIONS.map((option) => option.label),
      ...CHORD_KEY_GROUPS.flatMap((group) => [group.label, ...group.keys.map((key) => key.label)]),
    ];
    const missing = [...new Set(labels)].filter(
      (label) => /[\u4e00-\u9fff]/.test(label) && !hasTranslation(label),
    );
    expect(missing).toEqual([]);
  });

  it("localizes the default profile name only", () => {
    setLanguagePreference("en");
    try {
      expect(profileDisplayName("默认")).toBe("Default");
      expect(profileDisplayName("WorkBuddy")).toBe("WorkBuddy");
    } finally {
      setLanguagePreference("system");
    }
  });

  it("localizes the hotkey key caps", () => {
    setLanguagePreference("en");
    try {
      expect(hotkeyKeyLabel("Space")).toBe("Space");
      expect(hotkeyKeyLabel("Enter")).toBe("Enter");
      expect(hotkeyKeyLabel("Backspace")).toBe("Backspace");
      expect(hotkeyKeyLabel("ControlRight")).toBe("Right Ctrl");
    } finally {
      setLanguagePreference("system");
    }
  });
});
