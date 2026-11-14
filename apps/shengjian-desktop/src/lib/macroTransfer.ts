/**
 * 宏库的「单文件 / 多文件」导入导出纯函数。
 *
 * 单文件 = 整个库一个 JSON（原有行为）；多文件 = 每个宏一个 JSON（文件名用宏名）。
 * 这里只做数据变换，不碰文件系统与 Tauri，方便单测；实际读写由调用方负责。
 */
import type { MacroDefinition, MacroLibraryDocument } from "./types";

/** 取出指定的宏，组成一个可直接写盘的库文档。 */
export function libraryWithMacros(
  document: MacroLibraryDocument,
  macros: readonly MacroDefinition[],
): MacroLibraryDocument {
  const picked = new Set(macros.map((macro) => macro.macroId));
  return {
    formatVersion: document.formatVersion,
    definitions: document.definitions.filter((macro) => picked.has(macro.macroId)),
  };
}

/** 单个宏导出时的文件名主干（不含扩展名）：去掉非法字符，重名时加序号交给调用方。 */
export function macroFileStem(macro: MacroDefinition): string {
  const clean = (value: string) =>
    value
      .replace(/[\\/:*?"<>|]/g, "-")
      .replace(/\s+/g, " ")
      .trim();
  // 名字清洗后可能为空（例如只剩空格），退到 macroId，再退到固定名。
  return clean(macro.name) || clean(macro.macroId) || "macro";
}

/** 多文件导出时的一批文件名：重名自动加 `-2`、`-3`…，避免互相覆盖。 */
export function macroFileStems(macros: readonly MacroDefinition[]): string[] {
  const used = new Map<string, number>();
  return macros.map((macro) => {
    const stem = macroFileStem(macro);
    const seen = used.get(stem) ?? 0;
    used.set(stem, seen + 1);
    return seen === 0 ? stem : `${stem}-${seen + 1}`;
  });
}

export interface MacroMergeResult {
  document: MacroLibraryDocument;
  added: number;
  updated: number;
  skipped: number;
}

/**
 * 把导入的宏合并进当前库（多文件导入用）。
 *
 * 冲突策略（刻意简单、可预期）：
 * - 同 `macroId` + 同 `version` → 视为同一个宏，**跳过**（不覆盖用户当前版本）；
 * - 同 `macroId` 但版本不同 → 用导入的覆盖，计 `updated`；
 * - 新 `macroId` → 追加，计 `added`。
 */
export function mergeMacroLibraries(
  current: MacroLibraryDocument,
  incoming: MacroLibraryDocument,
): MacroMergeResult {
  const byId = new Map(current.definitions.map((macro) => [macro.macroId, macro] as const));
  const result = new Map(byId);
  let added = 0;
  let updated = 0;
  let skipped = 0;

  for (const macro of incoming.definitions) {
    const existing = byId.get(macro.macroId);
    if (!existing) {
      result.set(macro.macroId, macro);
      added += 1;
      continue;
    }
    if (existing.version === macro.version) {
      skipped += 1;
      continue;
    }
    result.set(macro.macroId, macro);
    updated += 1;
  }

  return {
    document: {
      formatVersion: Math.max(current.formatVersion, incoming.formatVersion),
      definitions: [...result.values()],
    },
    added,
    updated,
    skipped,
  };
}
