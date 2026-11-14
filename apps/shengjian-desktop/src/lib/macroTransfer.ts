/**
 * 宏库的「单文件 / 多文件」导入导出纯函数。
 *
 * 单文件 = 整个库一个 JSON（原有行为）；多文件 = 每个宏一个 JSON（文件名用宏名）。
 * 导入分两步：先 [`buildMacroImportRows`] 算出每条宏相对本机的位置（新增 / 更新 / 相同），
 * 再由 [`composeMacroImportDocument`] 按用户选择合成新库——这样界面能先展示再落地。
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

/** 一条待导入的宏相对本机宏库的位置。 */
export type MacroImportStatus = "new" | "update" | "same";

/** 同 ID 且版本不同时的默认处理方式。 */
export type MacroImportPolicy = "overwrite" | "keep";

/** 导入方式：合并进现有库，或用文件整库替换。 */
export type MacroImportMode = "merge" | "replace";

export interface MacroImportRow {
  macro: MacroDefinition;
  status: MacroImportStatus;
  /** 本机同 ID 宏的版本；没有就是 null。 */
  localVersion: string | null;
}

/**
 * 多个文件按 `macroId` 去重：同一个宏出现在两个文件里时，后选的文件说话。
 * 顺序按第一次出现的位置，避免预览列表跳来跳去。
 */
export function collectIncomingMacros(
  documents: readonly MacroLibraryDocument[],
): MacroDefinition[] {
  const byId = new Map<string, MacroDefinition>();
  for (const document of documents) {
    for (const macro of document.definitions) byId.set(macro.macroId, macro);
  }
  return [...byId.values()];
}

/** 逐条对比导入的宏与本机库：新 ID = 新增，版本不同 = 更新，版本相同 = 相同。 */
export function buildMacroImportRows(
  incoming: readonly MacroDefinition[],
  current: readonly MacroDefinition[],
): MacroImportRow[] {
  const byId = new Map(current.map((macro) => [macro.macroId, macro] as const));
  return incoming.map((macro) => {
    const local = byId.get(macro.macroId);
    if (!local) return { macro, status: "new" as const, localVersion: null };
    if (local.version === macro.version) {
      return { macro, status: "same" as const, localVersion: local.version };
    }
    return { macro, status: "update" as const, localVersion: local.version };
  });
}

export interface ResolvedMacroImportRow extends MacroImportRow {
  action: "add" | "overwrite" | "skip";
}

/**
 * 决定每条宏怎么落地。
 *
 * 新增总是加、相同总是跳过；版本不同的那条按用户的默认策略走，
 * 单条覆盖优先于默认策略——和按键方案的导入面板保持同一套手势。
 */
export function resolveMacroImportRows(
  rows: readonly MacroImportRow[],
  policy: MacroImportPolicy,
  overrides: Record<string, MacroImportPolicy> = {},
): ResolvedMacroImportRow[] {
  return rows.map((row) => {
    if (row.status === "new") return { ...row, action: "add" as const };
    if (row.status === "same") return { ...row, action: "skip" as const };
    const choice = overrides[row.macro.macroId] ?? policy;
    const action: ResolvedMacroImportRow["action"] = choice === "overwrite" ? "overwrite" : "skip";
    return { ...row, action };
  });
}

export interface MacroImportCounts {
  added: number;
  updated: number;
  /** 本机有更新版本、这次被保留下来的条数。 */
  kept: number;
  /** 两边版本相同、无事可做的条数。 */
  same: number;
}

export function macroImportCounts(rows: readonly ResolvedMacroImportRow[]): MacroImportCounts {
  const counts: MacroImportCounts = { added: 0, updated: 0, kept: 0, same: 0 };
  for (const row of rows) {
    if (row.action === "add") counts.added += 1;
    else if (row.action === "overwrite") counts.updated += 1;
    else if (row.status === "update") counts.kept += 1;
    else counts.same += 1;
  }
  return counts;
}

/** 本机独有的宏：整库替换时它们会被删掉，界面要提前说清楚。 */
export function localOnlyMacros(
  current: readonly MacroDefinition[],
  incoming: readonly MacroDefinition[],
): MacroDefinition[] {
  const ids = new Set(incoming.map((macro) => macro.macroId));
  return current.filter((macro) => !ids.has(macro.macroId));
}

/**
 * 合成导入后的宏库。
 *
 * - `merge`：保留本机独有的宏，按每条的动作加/覆盖，顺序沿用本机；
 * - `replace`：整个库变成文件内容（动作不参与，界面会把这条路径标成有损）。
 */
export function composeMacroImportDocument(
  current: MacroLibraryDocument,
  rows: readonly ResolvedMacroImportRow[],
  mode: MacroImportMode,
  incomingFormatVersion: number = current.formatVersion,
): MacroLibraryDocument {
  if (mode === "replace") {
    return {
      formatVersion: incomingFormatVersion,
      definitions: rows.map((row) => row.macro),
    };
  }

  const definitions = [...current.definitions];
  const indexById = new Map(definitions.map((macro, index) => [macro.macroId, index] as const));

  for (const row of rows) {
    if (row.action === "skip") continue;
    const index = indexById.get(row.macro.macroId);
    if (index === undefined) {
      indexById.set(row.macro.macroId, definitions.length);
      definitions.push(row.macro);
    } else {
      definitions[index] = row.macro;
    }
  }

  return {
    formatVersion: Math.max(current.formatVersion, incomingFormatVersion),
    definitions,
  };
}
