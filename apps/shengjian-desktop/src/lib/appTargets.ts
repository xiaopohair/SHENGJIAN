/**
 * 「打开应用」目标的纯函数工具：文本字段的合并/去重、目标身份展示与问题分类。
 *
 * 这里刻意不依赖 Vue 或 Tauri，方便单测；状态文案只返回 i18n key，由组件用 `tr()`
 * 翻译，避免把界面语言混进数据层。
 */
import type { TranslationKey } from "./i18n";
import type { AppTargetInfo } from "./types";

export type AppTargetMode = "launch" | "match";

/** 把方案设置里逗号分隔的「应用目标」文本拆成条目（去空白、去空项）。 */
export function splitAppTargets(value: string | null | undefined): string[] {
  return (value ?? "")
    .split(",")
    .map((target) => target.trim())
    .filter(Boolean);
}

/** 把条目拼回存储用的文本形式。 */
export function joinAppTargets(targets: readonly string[]): string {
  return targets.join(", ");
}

/**
 * 追加一个应用目标：先 trim，再按大小写不敏感去重（保留用户先看到的那个写法）。
 * 空串直接忽略，返回的数组始终是清理过的副本。
 */
export function mergeAppTarget(targets: readonly string[], next: string): string[] {
  const merged = targets.map((target) => target.trim()).filter(Boolean);
  const candidate = next.trim();
  if (!candidate) return merged;
  if (merged.some((target) => target.toLowerCase() === candidate.toLowerCase())) {
    return merged;
  }
  return [...merged, candidate];
}

/**
 * 新方案的默认名字：优先用选择器给的显示名（如 `WorkBuddy`），
 * 否则从目标里推出一个可读名（去掉目录与 `.exe`，首字母大写）。
 *
 * 用户没指定名字时用它，比一律叫「新方案」更容易认。
 */
export function defaultProfileName(target: string, displayName?: string | null): string {
  const given = (displayName ?? "").trim();
  if (given) return given;
  const raw = (target ?? "").trim();
  if (!raw) return "";
  const base = raw
    .replace(/^.*[\\/]/, "")
    .replace(/\.(exe|bat|cmd|lnk|msc)$/i, "")
    .trim();
  if (!base) return "";
  // 已带空格或大写（prose 名）就原样用，否则首字母大写便于阅读。
  if (/[\sA-Z]/.test(base)) return base;
  return base.charAt(0).toUpperCase() + base.slice(1);
}

/** `mergeAppTarget` 的文本字段版本，直接写回 `draft.appTargets`。 */
export function mergeAppTargetsText(current: string, next: string): string {
  return joinAppTargets(mergeAppTarget(splitAppTargets(current), next));
}

/** 非空的 target 字符串才算「已填写应用目标」。 */
export function isOpenAppTarget(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

/** 从路径/目标串里取可执行文件名（同时兼容 Windows 与 POSIX 分隔符）。 */
export function executableNameFromPath(path: string): string {
  const value = (path ?? "").trim();
  if (!value) return "";
  const parts = value.split(/[\\/]/);
  return (parts[parts.length - 1] ?? "").trim();
}

/** 展示用的友好名：系统显示名 → exe 名 → 原始 target。 */
export function displayNameOf(info: AppTargetInfo | null | undefined): string {
  if (!info) return "";
  const display = info.displayName?.trim();
  if (display) return display;
  const executable = info.executableName?.trim();
  if (executable) return executable;
  return info.target?.trim() ?? "";
}

/** 展示用的副标题：与友好名不同的 exe 名 / 完整 target；没有可显示的就为空。 */
export function subtitleOf(info: AppTargetInfo | null | undefined): string {
  if (!info) return "";
  const display = displayNameOf(info);
  const executable = info.executableName?.trim();
  if (executable && executable !== display) return executable;
  const target = info.target?.trim() ?? "";
  return target && target !== display ? target : "";
}

/** issue token → 具体原因的文案 key（用于「应用不可用：{reason}」）。 */
export function issueReasonKey(issue: string | null | undefined): TranslationKey {
  switch (issue) {
    case "empty":
      return "未填写应用目标";
    case "unknown_target":
      return "无法识别的目标";
    case "preset_not_installed":
    case "aumid_not_installed":
      return "应用未安装";
    case "path_missing":
      return "路径不存在";
    case "bare_name_unresolved":
      return "找不到该可执行文件";
    case "lnk_unresolved":
      return "快捷方式无法解析";
    case "no_executable_name":
      return "无法获取可执行文件名";
    case "non_executable_target":
      return "目标不是可执行文件";
    case "invalid":
      return "目标无效";
    default:
      return "未知问题";
  }
}

/** 问题徽标上的短标签。 */
export function issueBadgeKey(issue: string | null | undefined): TranslationKey {
  switch (issue) {
    case "path_missing":
      return "路径不存在";
    case "lnk_unresolved":
    case "no_executable_name":
    case "unknown_target":
    case "invalid":
    case "bare_name_unresolved":
      return "无法解析";
    case "preset_not_installed":
    case "aumid_not_installed":
      return "未安装";
    case "non_executable_target":
      return "仅启动";
    default:
      return "应用不可用";
  }
}

export interface AppTargetStatus {
  /** 徽标文案；null = 目标正常，不显示徽标。 */
  labelKey: TranslationKey | null;
  /** 徽标语义色。 */
  tone: "neutral" | "info" | "warning" | "danger";
  /** 完整说明（tooltip/详情），可能包含 {reason} 占位。 */
  detailKey: TranslationKey | null;
  /** detailKey 中 {reason} 的取值 key。 */
  reasonKey: TranslationKey | null;
  /** 是否应提供「修复」入口（重新选择应用）。 */
  repair: boolean;
}

const NO_PROBLEM: AppTargetStatus = {
  labelKey: null,
  tone: "neutral",
  detailKey: null,
  reasonKey: null,
  repair: false,
};

/**
 * 计算一个已描述的 target 在给定模式下的展示状态。
 *
 * - `launch`（动作/宏）：旧式、未安装、路径不存在、无法解析都算问题并给「修复」；
 *   能启动但不能自动匹配只提示「仅启动」。
 * - `match`（方案 appTargets）：这里本来就是 exe 名，不能把裸名当「旧式」噪音；
 *   只有 `bare_name_unresolved`（无法匹配）才告警；其余不可匹配的目标给中性提示。
 */
export function appTargetStatus(
  info: AppTargetInfo | null | undefined,
  mode: AppTargetMode = "launch",
): AppTargetStatus {
  if (!info) return NO_PROBLEM;

  if (mode === "match") {
    if (info.issue === "bare_name_unresolved") {
      return {
        labelKey: "无法解析",
        tone: "danger",
        detailKey: "应用不可用：{reason}",
        reasonKey: issueReasonKey(info.issue),
        repair: true,
      };
    }
    if (!info.matchable) {
      return {
        labelKey: "不支持自动匹配",
        tone: "neutral",
        detailKey: "该应用不支持自动匹配",
        reasonKey: null,
        repair: false,
      };
    }
    return NO_PROBLEM;
  }

  if (info.legacy || info.kind === "legacyName") {
    return {
      labelKey: "旧式",
      tone: "warning",
      detailKey: "旧式目标，建议重新选择",
      reasonKey: null,
      repair: true,
    };
  }

  if (!info.launchable) {
    return {
      labelKey: issueBadgeKey(info.issue),
      tone: "danger",
      detailKey: "应用不可用：{reason}",
      reasonKey: issueReasonKey(info.issue),
      repair: true,
    };
  }

  if (!info.matchable) {
    return {
      labelKey: "仅启动",
      tone: "neutral",
      detailKey: "仅支持启动，不会自动切换方案",
      reasonKey: null,
      repair: false,
    };
  }

  return NO_PROBLEM;
}
