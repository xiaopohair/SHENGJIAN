import { describe, expect, it } from "vitest";
import {
  buildMacroImportRows,
  collectIncomingMacros,
  composeMacroImportDocument,
  libraryWithMacros,
  localOnlyMacros,
  macroFileStem,
  macroFileStems,
  macroImportCounts,
  resolveMacroImportRows,
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

describe("collectIncomingMacros", () => {
  it("dedupes by macro id and lets the later file win", () => {
    const first = library(macro("a", "A", "1"), macro("b", "B", "1"));
    const second = library(macro("a", "A", "2"));
    const incoming = collectIncomingMacros([first, second]);
    expect(incoming.map((item) => `${item.macroId}:${item.version}`)).toEqual(["a:2", "b:1"]);
  });

  it("keeps first-seen order", () => {
    const incoming = collectIncomingMacros([library(macro("b", "B")), library(macro("a", "A"))]);
    expect(incoming.map((item) => item.macroId)).toEqual(["b", "a"]);
  });
});

describe("buildMacroImportRows", () => {
  const current = [macro("same", "Same", "1"), macro("old", "Old", "1")];

  it("marks new, updated and identical macros", () => {
    const rows = buildMacroImportRows(
      [macro("same", "Same", "1"), macro("old", "Old v2", "2"), macro("fresh", "Fresh", "1")],
      current,
    );
    expect(rows.map((row) => row.status)).toEqual(["same", "update", "new"]);
    expect(rows[1].localVersion).toBe("1");
    expect(rows[2].localVersion).toBeNull();
  });
});

describe("resolveMacroImportRows", () => {
  const rows = buildMacroImportRows(
    [macro("old", "Old v2", "2"), macro("same", "Same", "1")],
    [macro("old", "Old", "1"), macro("same", "Same", "1")],
  );

  it("follows the default policy for version clashes", () => {
    expect(resolveMacroImportRows(rows, "overwrite").map((row) => row.action)).toEqual([
      "overwrite",
      "skip",
    ]);
    expect(resolveMacroImportRows(rows, "keep").map((row) => row.action)).toEqual(["skip", "skip"]);
  });

  it("lets a single row override the policy", () => {
    const resolved = resolveMacroImportRows(rows, "overwrite", { old: "keep" });
    expect(resolved.map((row) => row.action)).toEqual(["skip", "skip"]);
  });

  it("counts applied and skipped work separately", () => {
    const overwrite = macroImportCounts(resolveMacroImportRows(rows, "overwrite"));
    expect(overwrite).toEqual({ added: 0, updated: 1, kept: 0, same: 1 });

    const keep = macroImportCounts(resolveMacroImportRows(rows, "keep"));
    expect(keep).toEqual({ added: 0, updated: 0, kept: 1, same: 1 });
  });
});

describe("composeMacroImportDocument", () => {
  const current = library(macro("old", "Old", "1"), macro("local", "Local only", "1"));

  it("merges into the current library and keeps local-only macros", () => {
    const rows = resolveMacroImportRows(
      buildMacroImportRows(
        [macro("old", "Old v2", "2"), macro("fresh", "Fresh", "1")],
        current.definitions,
      ),
      "overwrite",
    );
    const next = composeMacroImportDocument(current, rows, "merge");
    expect(next.definitions.map((item) => `${item.macroId}:${item.version}`)).toEqual([
      "old:2",
      "local:1",
      "fresh:1",
    ]);
  });

  it("replaces the whole library and takes the incoming format version", () => {
    const rows = resolveMacroImportRows(
      buildMacroImportRows([macro("fresh", "Fresh", "1")], current.definitions),
      "overwrite",
    );
    const next = composeMacroImportDocument(current, rows, "replace", 3);
    expect(next.definitions.map((item) => item.macroId)).toEqual(["fresh"]);
    expect(next.formatVersion).toBe(3);
  });

  it("never drops local macros when everything is kept", () => {
    const rows = resolveMacroImportRows(
      buildMacroImportRows([macro("old", "Old v2", "2")], current.definitions),
      "keep",
    );
    const next = composeMacroImportDocument(current, rows, "merge");
    expect(next.definitions.map((item) => `${item.macroId}:${item.version}`)).toEqual([
      "old:1",
      "local:1",
    ]);
  });
});

describe("localOnlyMacros", () => {
  it("lists the macros a whole-library replace would delete", () => {
    const current = library(macro("old", "Old"), macro("local", "Local"));
    const kept = localOnlyMacros(current.definitions, [macro("old", "Old")]);
    expect(kept.map((item) => item.macroId)).toEqual(["local"]);
  });
});
