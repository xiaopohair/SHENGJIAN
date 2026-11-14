import { describe, expect, it } from "vitest";
import {
  libraryWithMacros,
  macroFileStem,
  macroFileStems,
  mergeMacroLibraries,
} from "./macroTransfer";
import type { MacroDefinition, MacroLibraryDocument } from "./types";

function macro(macroId: string, name: string, version = "1"): MacroDefinition {
  return {
    schemaVersion: "1.0",
    macroId,
    version,
    name,
    summary: null,
    steps: [],
  };
}

function library(...macros: MacroDefinition[]): MacroLibraryDocument {
  return { formatVersion: 1, definitions: macros };
}

describe("libraryWithMacros", () => {
  it("keeps only the picked macros but preserves the format version", () => {
    const doc = library(macro("a", "A"), macro("b", "B"), macro("c", "C"));
    const picked = libraryWithMacros(doc, [doc.definitions[0]!, doc.definitions[2]!]);
    expect(picked.definitions.map((item) => item.macroId)).toEqual(["a", "c"]);
    expect(picked.formatVersion).toBe(1);
  });

  it("keeps the library order when the picks are given out of order", () => {
    const doc = library(macro("a", "A"), macro("b", "B"));
    const picked = libraryWithMacros(doc, [doc.definitions[1]!]);
    expect(picked.definitions.map((item) => item.macroId)).toEqual(["b"]);
  });
});

describe("macro file names", () => {
  it("uses the macro name and strips illegal characters", () => {
    expect(macroFileStem(macro("a", "打开 记事本"))).toBe("打开 记事本");
    expect(macroFileStem(macro("a", "a/b:c*?"))).toBe("a-b-c--");
    expect(macroFileStem(macro("a", "   "))).toBe("a");
  });

  it("suffixes duplicate names so files do not overwrite each other", () => {
    const stems = macroFileStems([macro("a", "同名"), macro("b", "同名"), macro("c", "同名")]);
    expect(stems).toEqual(["同名", "同名-2", "同名-3"]);
  });
});

describe("mergeMacroLibraries", () => {
  it("adds new macros, replaces newer versions and skips identical ones", () => {
    const current = library(
      macro("keep", "Keep"),
      macro("same", "Same", "1"),
      macro("old", "Old", "1"),
    );
    const incoming = library(
      macro("same", "Same", "1"),
      macro("old", "Old v2", "2"),
      macro("fresh", "Fresh", "1"),
    );
    const merged = mergeMacroLibraries(current, incoming);
    expect(merged.added).toBe(1);
    expect(merged.updated).toBe(1);
    expect(merged.skipped).toBe(1);
    expect(merged.document.definitions.map((item) => `${item.macroId}:${item.version}`)).toEqual([
      "keep:1",
      "same:1",
      "old:2",
      "fresh:1",
    ]);
  });

  it("is idempotent: merging the same library twice changes nothing", () => {
    const current = library(macro("a", "A", "2"));
    const once = mergeMacroLibraries(current, library(macro("a", "A", "2")));
    expect(once.added).toBe(0);
    expect(once.updated).toBe(0);
    expect(once.skipped).toBe(1);
    expect(once.document.definitions).toHaveLength(1);
  });
});
