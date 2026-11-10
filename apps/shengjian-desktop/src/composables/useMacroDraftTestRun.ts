import { onScopeDispose, ref, type Ref } from "vue";
import { cancelMacro, getMacroRunStatus, testMacroDraft } from "../lib/api";
import type { MacroDefinition, MacroRunStatus } from "../lib/types";

type ErrorReporter = (cause: unknown) => void;

/**
 * Owns the transient draft run id, status polling, and cancellation.
 * Polling is serial: the next tick starts only after the current IPC call ends.
 */
export function useMacroDraftTestRun(
  editor: Ref<{ macroId: string } | null>,
  reportError: ErrorReporter,
) {
  const draftRunStatus = ref<MacroRunStatus | null>(null);
  const draftRunMacroId = ref<string | null>(null);
  const draftTesting = ref(false);
  let pollTimer: number | undefined;

  function clearPoll() {
    if (pollTimer !== undefined) {
      window.clearTimeout(pollTimer);
      pollTimer = undefined;
    }
  }

  function resetDraftRun() {
    clearPoll();
    draftRunStatus.value = null;
    draftRunMacroId.value = null;
    draftTesting.value = false;
  }

  function schedulePoll(macroId: string) {
    clearPoll();
    const tick = async () => {
      if (!editor.value || draftRunMacroId.value !== macroId) {
        clearPoll();
        return;
      }
      try {
        const status = await getMacroRunStatus(macroId);
        if (draftRunMacroId.value !== macroId) return;
        draftRunStatus.value = status;
        if (status.running) {
          pollTimer = window.setTimeout(() => void tick(), 400);
        } else {
          clearPoll();
        }
      } catch {
        clearPoll();
      }
    };
    void tick();
  }

  async function testDraft(definition: MacroDefinition) {
    if (!editor.value || draftTesting.value) return;
    draftTesting.value = true;
    draftRunStatus.value = null;
    draftRunMacroId.value = definition.macroId;
    try {
      await testMacroDraft(definition);
      schedulePoll(definition.macroId);
    } catch (cause) {
      reportError(cause);
      draftRunMacroId.value = null;
    } finally {
      draftTesting.value = false;
    }
  }

  async function stopDraft() {
    if (!draftRunMacroId.value) return;
    try {
      await cancelMacro(draftRunMacroId.value);
      schedulePoll(draftRunMacroId.value);
    } catch (cause) {
      reportError(cause);
    }
  }

  function closeDraftRun() {
    if (draftRunMacroId.value && draftRunStatus.value?.running) {
      void cancelMacro(draftRunMacroId.value);
    }
    resetDraftRun();
  }

  onScopeDispose(() => {
    if (draftRunMacroId.value && draftRunStatus.value?.running) {
      void cancelMacro(draftRunMacroId.value);
    }
    clearPoll();
  });

  return {
    draftRunStatus,
    draftRunMacroId,
    draftTesting,
    testDraft,
    stopDraft,
    resetDraftRun,
    closeDraftRun,
  };
}
