import { computed, ref, toRaw, watch, type Ref } from "vue";
import { createMacroStep, defaultMacroParameters, type MacroDraft } from "../lib/macros";
import { stepIssue as calculateStepIssue, stepSummary } from "../lib/macroEditor";
import type { MacroStep } from "../lib/types";

export interface MacroStepRow {
  step: MacroStep;
  index: number;
  issue: string;
  summary: string;
}

export function useMacroDraftEditor(draft: Ref<MacroDraft>) {
  const selectedStepId = ref<string | null>(null);

  const selectedStep = computed(
    () => draft.value.steps.find((step) => step.stepId === selectedStepId.value) ?? null,
  );
  const selectedStepIndex = computed(() =>
    draft.value.steps.findIndex((step) => step.stepId === selectedStepId.value),
  );
  const stepRows = computed<MacroStepRow[]>(() =>
    draft.value.steps.map((step, index) => ({
      step,
      index,
      issue: calculateStepIssue(step),
      summary: stepSummary(step),
    })),
  );
  const issues = computed(() => stepRows.value.filter((row) => row.issue));
  const canTest = computed(() => draft.value.steps.length > 0 && issues.value.length === 0);

  watch(
    () => draft.value.steps.map((step) => step.stepId),
    () => {
      if (!draft.value.steps.some((step) => step.stepId === selectedStepId.value)) {
        selectedStepId.value = draft.value.steps[0]?.stepId ?? null;
      }
    },
    { immediate: true },
  );

  function commit(next: MacroDraft) {
    draft.value = next;
  }

  function setName(name: string) {
    commit({ ...draft.value, name });
  }

  function setSummary(summary: string) {
    commit({ ...draft.value, summary });
  }

  function selectStep(step: MacroStep) {
    selectedStepId.value = step.stepId;
  }

  function addStep() {
    insertStep(draft.value.steps.length);
  }

  function insertStep(index: number) {
    const step = createMacroStep();
    const steps = [...draft.value.steps];
    steps.splice(index, 0, step);
    commit({ ...draft.value, steps });
    selectedStepId.value = step.stepId;
  }

  function removeStep(stepId: string) {
    const index = draft.value.steps.findIndex((step) => step.stepId === stepId);
    if (index < 0) return;
    const steps = draft.value.steps.filter((step) => step.stepId !== stepId);
    commit({ ...draft.value, steps });
    if (selectedStepId.value === stepId) {
      selectedStepId.value = steps[index]?.stepId ?? steps[index - 1]?.stepId ?? null;
    }
  }

  function moveStep(stepId: string, delta: number) {
    const index = draft.value.steps.findIndex((step) => step.stepId === stepId);
    const target = index + delta;
    if (index < 0 || target < 0 || target >= draft.value.steps.length) return;
    const steps = [...draft.value.steps];
    const [step] = steps.splice(index, 1);
    steps.splice(target, 0, step);
    commit({ ...draft.value, steps });
  }

  function duplicateStep(stepId: string) {
    const index = draft.value.steps.findIndex((step) => step.stepId === stepId);
    if (index < 0) return;
    // The draft holds reactive proxies, which structuredClone cannot copy.
    // Read the fields into a plain object, then clone that.
    const source = draft.value.steps[index];
    const copy: MacroStep = {
      stepId: `step-${crypto.randomUUID()}`,
      action: source.action,
      timeoutMs: source.timeoutMs ?? null,
      parameters: structuredClone(toRaw(source.parameters)),
    };
    const steps = [...draft.value.steps];
    steps.splice(index + 1, 0, copy);
    commit({ ...draft.value, steps });
    selectedStepId.value = copy.stepId;
  }

  function dropStep(from: number, to: number) {
    if (from === to) return;
    const steps = [...draft.value.steps];
    const [step] = steps.splice(from, 1);
    steps.splice(to, 0, step);
    commit({ ...draft.value, steps });
  }

  function updateStep(stepId: string, nextStep: MacroStep) {
    const steps = draft.value.steps.map((step) => (step.stepId === stepId ? nextStep : step));
    commit({ ...draft.value, steps });
  }

  function changeAction(stepId: string, action: string) {
    const step = draft.value.steps.find((item) => item.stepId === stepId);
    if (!step) return;
    updateStep(stepId, {
      ...step,
      action,
      parameters: defaultMacroParameters(action),
    });
  }

  return {
    selectedStepId,
    selectedStep,
    selectedStepIndex,
    stepRows,
    issues,
    canTest,
    setName,
    setSummary,
    selectStep,
    addStep,
    insertStep,
    removeStep,
    moveStep,
    duplicateStep,
    dropStep,
    updateStep,
    changeAction,
  };
}
