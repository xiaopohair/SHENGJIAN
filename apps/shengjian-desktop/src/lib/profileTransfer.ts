import { profileAppliesToModel } from "./buttons";
import type { ButtonProfile, ButtonProfilesDocument } from "./types";

/** 导出形态：一个可再次导入的备份文件，或每个方案一个推荐方案文件。 */
export type TransferFormat = "document" | "preset";

/** 一条导入方案相对本机现状的位置。 */
export type ImportStatus = "new" | "update" | "conflict";

/** 单条方案最终怎么落地。 */
export type ImportAction = "add" | "overwrite" | "keep" | "skip";

/** 同名方案的默认处理方式。 */
export type ConflictPolicy = "overwrite" | "keep" | "skip";

const TRIGGERS = ["single", "double", "long"] as const;

export interface ProfileStats {
  binds: number;
  appCount: number;
  macroIds: string[];
}

/** 一条方案占了多少键位、绑了几个应用、引用了哪些宏。 */
export function profileStats(profile: ButtonProfile): ProfileStats {
  const macroIds = new Set<string>();
  let binds = 0;
  for (const byTrigger of Object.values(profile.bindings ?? {})) {
    if (!byTrigger || typeof byTrigger !== "object") continue;
    for (const trigger of TRIGGERS) {
      const action = (byTrigger as Record<string, { type?: string; macro_id?: unknown }>)[trigger];
      if (!action || typeof action !== "object") continue;
      binds += 1;
      if (action.type === "run-macro" && typeof action.macro_id === "string" && action.macro_id) {
        macroIds.add(action.macro_id);
      }
    }
  }
  return {
    binds,
    appCount: (profile.appTargets ?? []).filter((target) => target.trim().length > 0).length,
    macroIds: [...macroIds],
  };
}

export interface TransferSummary {
  count: number;
  binds: number;
  macros: number;
}

/** 一组方案的合计：方案数、键位数、引用到的宏数。 */
export function transferSummary(profiles: ButtonProfile[]): TransferSummary {
  const macroIds = new Set<string>();
  let binds = 0;
  for (const profile of profiles) {
    const stats = profileStats(profile);
    binds += stats.binds;
    for (const id of stats.macroIds) macroIds.add(id);
  }
  return { count: profiles.length, binds, macros: macroIds.size };
}

/** 按导出时的缩进格式估算文件字节数，供按钮上标「约 xx KB」。 */
export function estimateDocumentBytes(document: Pick<ButtonProfilesDocument, "profiles">): number {
  const json = JSON.stringify(document, null, 2);
  if (typeof TextEncoder === "undefined") return json.length;
  return new TextEncoder().encode(json).length;
}

export function formatKilobytes(bytes: number): string {
  const kb = bytes / 1024;
  return kb >= 100 ? `${Math.round(kb)} KB` : `${kb.toFixed(1)} KB`;
}

/** 导出文件名里的日期片段（本地时区）。 */
export function dateStamp(date = new Date()): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

/** 方案名 → 推荐方案 id 片段：保留中英文与数字，其余折叠成连字符。 */
export function presetSlug(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[\s_/\\]+/g, "-")
    .replace(/[^a-z0-9\u4e00-\u9fa5-]/g, "")
    .replace(/-{2,}/g, "-")
    .replace(/^-|-$/g, "");
  return slug || "custom";
}

/** 生成不重复的推荐方案 id（`preset.<型号>.<名称>`）。 */
export function presetIdsFor(
  profiles: ButtonProfile[],
  fallbackModel: string | null,
): Map<string, string> {
  const used = new Set<string>();
  const result = new Map<string, string>();
  for (const profile of profiles) {
    const model = (profile.deviceModel ?? fallbackModel ?? "generic").trim().toLowerCase();
    const base = `preset.${model}.${presetSlug(profile.name)}`;
    let candidate = base;
    let suffix = 2;
    while (used.has(candidate)) {
      candidate = `${base}-${suffix}`;
      suffix += 1;
    }
    used.add(candidate);
    result.set(profile.id, candidate);
  }
  return result;
}

export interface ImportRow {
  profile: ButtonProfile;
  status: ImportStatus;
  /** 与本机哪条方案对应（按 id 更新，或按同名覆盖）。 */
  matchId: string | null;
  /** 默认方案是型号基线，不允许被导入内容覆盖。 */
  locked: boolean;
  compatible: boolean;
  binds: number;
  macros: number;
}

/**
 * 把文件里的方案逐条对到本机现状。
 *
 * 规则与用户能理解的话一一对应：同一个 id = 更新，同名不同 id = 冲突，
 * 型号对不上 = 不兼容（仍可导入，只是当前遥控器用不到）。
 */
export function buildImportRows(
  incoming: ButtonProfile[],
  current: ButtonProfile[],
  model: string | null,
): ImportRow[] {
  const byId = new Map(current.map((profile) => [profile.id, profile]));
  const byName = new Map(current.map((profile) => [profile.name, profile]));

  return incoming.map((profile) => {
    const sameId = byId.get(profile.id);
    const sameName = byName.get(profile.name);
    const stats = profileStats(profile);

    let status: ImportStatus = "new";
    let match: ButtonProfile | null = null;
    if (sameId && sameId.name === profile.name) {
      status = "update";
      match = sameId;
    } else if (sameName) {
      status = "conflict";
      match = sameName;
    } else if (sameId) {
      // id 撞了但名字不同：同样当冲突处理，避免悄悄改掉别人的方案。
      status = "conflict";
      match = sameId;
    }

    return {
      profile,
      status,
      matchId: match?.id ?? null,
      locked: Boolean(match?.isDefault),
      compatible: profileAppliesToModel(profile, model),
      binds: stats.binds,
      macros: stats.macroIds.length,
    };
  });
}

export interface ResolvedImportRow extends ImportRow {
  action: ImportAction;
}

export interface ResolveImportOptions {
  policy: ConflictPolicy;
  /** 单条覆盖：命中即优先于策略。 */
  overrides: Record<string, ImportAction>;
  onlyCompatible: boolean;
  /** 导入时一并带来的宏数量（用于提示，不参与落地）。 */
  model: string | null;
}

/**
 * 决定每条方案怎么落地。
 *
 * 不兼容与默认方案是硬规则（跳过），同名方案走用户选择的策略，
 * 同一 id 的更新默认覆盖——这正是「导入」这个词的意思。
 */
export function resolveImportRows(
  rows: ImportRow[],
  options: Pick<ResolveImportOptions, "policy" | "overrides" | "onlyCompatible">,
): ResolvedImportRow[] {
  return rows.map((row) => {
    if (!row.compatible && options.onlyCompatible) return { ...row, action: "skip" };
    if (row.locked) return { ...row, action: "skip" };

    const override = options.overrides[row.profile.id];
    if (override) return { ...row, action: override };
    if (row.status === "conflict") return { ...row, action: options.policy };
    if (row.status === "update") return { ...row, action: "overwrite" };
    return { ...row, action: "add" };
  });
}

/** 导入/导出结束后结果页要展示的那点数据。 */
export type TransferResultState =
  | {
      kind: "export";
      count: number;
      binds: number;
      macros: number;
      size: string;
      path: string;
    }
  | { kind: "import"; counts: ImportCounts }
  | { kind: "error"; message: string; title?: string };

export interface ImportCounts {
  add: number;
  overwrite: number;
  keep: number;
  skip: number;
}

export function importCounts(rows: ResolvedImportRow[]): ImportCounts {
  const counts: ImportCounts = { add: 0, overwrite: 0, keep: 0, skip: 0 };
  for (const row of rows) counts[row.action] += 1;
  return counts;
}

/** 落地后的方案总数 = 真正会出现在列表里的条数。 */
export function importedTotal(counts: ImportCounts): number {
  return counts.add + counts.overwrite + counts.keep;
}

function uniqueName(base: string, taken: Set<string>): string {
  let name = base;
  let index = 2;
  while (taken.has(name)) {
    name = `${base} ${index}`;
    index += 1;
  }
  return name;
}

function uniqueId(base: string, taken: Set<string>): string {
  let id = base;
  let index = 2;
  while (taken.has(id)) {
    id = `${base}-${index}`;
    index += 1;
  }
  return id;
}

/**
 * 把解决完的导入行合进当前文档。
 *
 * 覆盖保留本机 id（活动方案、快捷键引用都还认得它），保留两者则生成
 * 新 id 并把名字标成「副本」，所以导入永远不会让本机少一条方案。
 */
export function composeImportDocument(
  current: ButtonProfilesDocument,
  rows: ResolvedImportRow[],
): ButtonProfilesDocument {
  const profiles = [...current.profiles];
  const ids = new Set(profiles.map((profile) => profile.id));
  const names = new Set(profiles.map((profile) => profile.name));

  for (const row of rows) {
    if (row.action === "skip") continue;

    if (row.action === "overwrite" && row.matchId) {
      const index = profiles.findIndex((profile) => profile.id === row.matchId);
      if (index >= 0) {
        const replaced = profiles[index];
        names.delete(replaced.name);
        names.add(row.profile.name);
        profiles[index] = { ...row.profile, id: replaced.id };
      }
      continue;
    }

    if (row.action === "add") {
      profiles.push({ ...row.profile });
      ids.add(row.profile.id);
      names.add(row.profile.name);
      continue;
    }

    if (row.action === "keep") {
      const name = uniqueName(`${row.profile.name}（副本）`, names);
      const id = uniqueId(row.profile.id, ids);
      names.add(name);
      ids.add(id);
      profiles.push({ ...row.profile, id, name });
    }
  }

  return { ...current, profiles };
}
