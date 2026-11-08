import { describe, expect, it } from "vitest";
import { compareMacroVersions, latestMacroDefinitions, nextMacroVersion } from "./macros";
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
});
