import { computed, onMounted, onScopeDispose, ref, shallowRef, type Ref } from "vue";
import { getMacroLibrary, getMacroRunStatus } from "../lib/api";
import { compareMacroVersions, latestMacroDefinitions, nextMacroVersion } from "../lib/macros";
import type { MacroDefinition, MacroLibraryDocument, MacroRunStatus } from "../lib/types";

export function useMacroLibrary(reportError: (cause: unknown) => void) {
  const macroDocument = ref<MacroLibraryDocument | null>(null);
  const loading = shallowRef(true);

  const latestMacros = computed(() =>
    latestMacroDefinitions(macroDocument.value?.definitions ?? []),
  );

  const versionsByMacro = computed(() => {
    const versions = new Map<string, MacroDefinition[]>();
    for (const definition of macroDocument.value?.definitions ?? []) {
      const list = versions.get(definition.macroId) ?? [];
      list.push(definition);
      versions.set(definition.macroId, list);
    }
    for (const list of versions.values()) {
      list.sort((left, right) => compareMacroVersions(right.version, left.version));
    }
    return versions;
  });

  function versionsFor(macroId: string): MacroDefinition[] {
    return versionsByMacro.value.get(macroId) ?? [];
  }

  function nextVersion(macroId: string): string {
    return nextMacroVersion(macroDocument.value?.definitions ?? [], macroId);
  }

  async function load() {
    try {
      macroDocument.value = await getMacroLibrary();
    } catch (cause) {
      reportError(cause);
    } finally {
      loading.value = false;
    }
  }

  return {
    macroDocument,
    loading,
    latestMacros,
    versionsFor,
    nextVersion,
    load,
  };
}

export function useMacroRunStates(latestMacros: Readonly<Ref<MacroDefinition[]>>) {
  const runStates = ref<Record<string, MacroRunStatus>>({});
  let runTimer: number | undefined;
  let refreshing = false;

  async function refreshRunStates() {
    if (refreshing) return;
    refreshing = true;
    try {
      const entries = await Promise.all(
        latestMacros.value.map(async (definition) => {
          try {
            const status = await getMacroRunStatus(definition.macroId);
            return [definition.macroId, status] as const;
          } catch {
            return null;
          }
        }),
      );
      const next: Record<string, MacroRunStatus> = {};
      for (const entry of entries) {
        if (entry) next[entry[0]] = entry[1];
      }
      runStates.value = next;
    } finally {
      refreshing = false;
    }
  }

  onMounted(() => {
    runTimer = window.setInterval(() => {
      if (window.document.visibilityState === "visible") void refreshRunStates();
    }, 3000);
  });

  onScopeDispose(() => {
    if (runTimer !== undefined) window.clearInterval(runTimer);
  });

  return { runStates, refreshRunStates };
}
