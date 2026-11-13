import { describe, expect, it } from "vitest";
import {
  bumpMacroVersion,
  compareMacroVersions,
  latestMacroDefinitions,
  mergeRecommendedMacros,
  nextMacroVersion,
} from "./macros";
import type { MacroDefinition } from "./types";

function definition(macroId: string, version: string, name = "Macro"): MacroDefinition {
  return {
    schemaVersion: "0.1",
    macroId,
    version,
    name,
    steps: [
      {
        stepId: "step-1",
        action: "focus_frontmost",
        parameters: {},
      },
    ],
  };
}

describe("macro version helpers", () => {
  it("compares semver numerically, not lexically", () => {
    expect(compareMacroVersions("1.0.10", "1.0.9")).toBeGreaterThan(0);
    expect(compareMacroVersions("1.2.0", "1.10.0")).toBeLessThan(0);
    expect(compareMacroVersions("1.0.0", "1.0.0")).toBe(0);
  });

  it("keeps one latest definition per macro id", () => {
    const latest = latestMacroDefinitions([
      definition("macro-a", "1.0.0", "old"),
      definition("macro-b", "1.0.0", "b"),
      definition("macro-a", "1.0.2", "new"),
      definition("macro-a", "1.0.1", "middle"),
    ]);
    expect(latest).toHaveLength(2);
    expect(latest.find((item) => item.macroId === "macro-a")?.name).toBe("new");
  });

  it("bumps the patch version from the latest definition", () => {
    const definitions = [definition("macro-a", "1.0.0"), definition("macro-a", "1.0.9")];
    expect(nextMacroVersion(definitions, "macro-a")).toBe("1.0.10");
    expect(nextMacroVersion(definitions, "macro-b")).toBe("1.0.0");
  });

  it("bumps a specific version's patch number", () => {
    expect(bumpMacroVersion("1.0.1")).toBe("1.0.2");
    expect(bumpMacroVersion("2.3")).toBe("2.3.1");
    expect(bumpMacroVersion("1.0.9")).toBe("1.0.10");
  });
});

describe("recommended macro merging", () => {
  it("keeps user definitions for ids that also exist in the recommended bundle", () => {
    const merged = mergeRecommendedMacros(
      [definition("shared", "1.0.0", "用户版本")],
      [
        definition("shared", "9.0.0", "推荐版本"),
        definition("recommended-only", "1.0.0", "推荐宏"),
      ],
    );

    expect(merged.map((item) => item.macroId)).toEqual(["shared", "recommended-only"]);
    const latest = latestMacroDefinitions(merged);
    expect(latest.find((item) => item.macroId === "shared")?.name).toBe("用户版本");
    expect(latest.some((item) => item.name === "推荐版本")).toBe(false);
  });

  it("includes recommended-only macros and leaves the inputs untouched", () => {
    const user = [definition("user-macro", "1.0.0", "用户宏")];
    const recommended = [definition("recommended-only", "1.0.0", "推荐宏")];
    const merged = mergeRecommendedMacros(user, recommended);

    expect(merged.map((item) => item.macroId)).toEqual(["user-macro", "recommended-only"]);
    expect(user).toHaveLength(1);
    expect(recommended).toHaveLength(1);
  });

  it("handles empty libraries", () => {
    expect(mergeRecommendedMacros([], [])).toEqual([]);
    expect(mergeRecommendedMacros([], [definition("recommended", "1.0.0")])).toHaveLength(1);
    expect(mergeRecommendedMacros([definition("user", "1.0.0")], [])).toHaveLength(1);
  });
});
