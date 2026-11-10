import { describe, expect, it } from "vitest";
import { ref } from "vue";
import { useMacroDraftEditor } from "./useMacroDraftEditor";
import type { MacroDraft } from "../lib/macros";

function draft(): MacroDraft {
  return {
    macroId: "macro-1",
    name: "Test",
    summary: "",
    version: "1.0.0",
    steps: [
      {
        stepId: "step-1",
        action: "send_shortcut",
        timeoutMs: 500,
        parameters: { chord: { modifiers: ["control"], key: "k" } },
      },
    ],
  };
}

describe("useMacroDraftEditor", () => {
  it("duplicates a step with a fresh id and independent parameters", () => {
    const model = ref(draft());
    const { duplicateStep, stepRows } = useMacroDraftEditor(model);

    duplicateStep("step-1");

    expect(model.value.steps).toHaveLength(2);
    const [original, copy] = model.value.steps;
    expect(copy.stepId).not.toBe(original.stepId);
    expect(copy.action).toBe(original.action);
    expect(copy.timeoutMs).toBe(500);
    expect(copy.parameters).toEqual(original.parameters);

    // Mutating the copy must not touch the original.
    copy.parameters.chord = { modifiers: [], key: "z" };
    expect(original.parameters.chord).toEqual({ modifiers: ["control"], key: "k" });
    expect(stepRows.value).toHaveLength(2);
  });
});
