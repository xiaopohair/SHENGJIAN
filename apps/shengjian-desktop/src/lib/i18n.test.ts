import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { hasTranslation } from "./i18n";

function sourceFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(full);
    return /\.(vue|ts)$/.test(entry.name) && !entry.name.endsWith(".test.ts") ? [full] : [];
  });
}

describe("i18n dictionary", () => {
  it("covers every literal key passed to tr()", () => {
    const missing = new Set<string>();
    for (const file of sourceFiles(path.resolve(process.cwd(), "src"))) {
      if (file.endsWith("i18n.ts")) continue;
      const source = fs.readFileSync(file, "utf8");
      for (const match of source.matchAll(/\btr\(\s*"((?:[^"\\]|\\.)*)"/g)) {
        let key = match[1];
        try {
          key = JSON.parse(`"${key}"`) as string;
        } catch {
          // Keep the raw source if it is not a valid quoted string.
        }
        if (!hasTranslation(key)) missing.add(key);
      }
    }
    expect([...missing]).toEqual([]);
  });
});
