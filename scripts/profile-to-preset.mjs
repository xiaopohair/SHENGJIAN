#!/usr/bin/env node
// 推荐按键方案的源文件管理 + 转换 + 聚合 + 校验。
//
// 源码侧是**一方案一文件**（便于 review / 减少冲突）：
//   config/button-profiles/catalog.json              { schemaVersion, bundleVersion }
//   config/button-profiles/preset.desktop.shortcuts.json
//   config/button-profiles/preset.netease.controls.json
//
// 运行时/安装包仍然只认单个聚合文件（`include_str!` 内嵌兜底 + 资源文件）：
//   config/button-profiles.json   { schemaVersion, bundleVersion, profiles: [...] }
// 聚合文件由 `--build` 从目录生成，必须保持与目录一致（CI 用 `--check` 强制）。
//
// 另外负责把应用「按键」页导出的用户方案库转成推荐方案条目：
//   { "schemaVersion": 5, "profiles": [...], "activeManualProfileId": "..." }
//
// 用法：
//   node scripts/profile-to-preset.mjs --build               # 目录 → 聚合文件
//   node scripts/profile-to-preset.mjs --check               # 校验目录 + 聚合一致性
//   node scripts/profile-to-preset.mjs --list --input exported.json
//   node scripts/profile-to-preset.mjs --input exported.json --profile "豆包输入" \
//        --id preset.rc003.doubao --models RC003 --dry-run

import { readFileSync, writeFileSync, existsSync, readdirSync, mkdirSync, unlinkSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, resolve, basename } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "..");
const CONFIG_DIR = resolve(REPO_ROOT, "config");
/** 一方案一文件的源目录。 */
const PROFILE_DIR = resolve(CONFIG_DIR, "button-profiles");
/** 源目录里的版本清单（文件名固定，不当方案解析）。 */
const DIR_MANIFEST = "catalog.json";
/** 聚合产物：运行时与安装包唯一读取的文件。 */
const DEFAULT_CATALOG = resolve(CONFIG_DIR, "button-profiles.json");
const MACRO_CATALOG = resolve(CONFIG_DIR, "recommended-macros.json");

/** 与 `shengjian-contract` 的 `PROFILE_BUTTONS` 保持一致（12 键，TV/Menu 含在内）。 */
const PROFILE_BUTTONS = [
  "back",
  "ok",
  "home",
  "tv",
  "menu",
  "left",
  "right",
  "down",
  "up",
  "power",
  "volume-up",
  "volume-down",
];

const PRESET_ID_PREFIX = "preset.";
const MAX_BUTTON_PRESETS = 200;
const TRIGGERS = ["single", "double", "long"];
const ACTION_TYPES = [
  "disabled",
  "shortcut",
  "open-app",
  "focus-input",
  "run-macro",
  "mouse",
  "common-phrase",
  "system-action",
];

const USAGE = `推荐按键方案：一方案一文件的源目录 → 聚合文件，兼做「我的方案」转换。

目录结构（源码侧，手工编辑这里是主要入口）：
  config/button-profiles/catalog.json            { schemaVersion, bundleVersion }
  config/button-profiles/<方案 id>.json          一个文件一条推荐方案
聚合产物（不要手改，由 --build 生成；运行时与安装包只认它）：
  config/button-profiles.json                    { schemaVersion, bundleVersion, profiles }

目录维护：
  --build               目录 → 聚合文件（写完建议跑一次 --check）
  --check               校验目录 + 聚合文件是否与目录一致（CI 用）

把应用导出的方案转成推荐方案：
  --input <file>        应用「按键」页导出的 button-profiles.json
  --profile <name|id|index>   方案名 / 方案 id / 1 起的序号；只有一条非默认方案时可省略
  --list                      只列出导出文件里的方案，不做转换

推荐方案标识（转换模式）：
  --id <preset.xxx>     推荐方案 id，必须以 "preset." 开头；源 id 已是 preset.* 时可省略
  --name <显示名>        覆盖方案名（默认沿用导出时的名字）
  --models <A,B>        适配型号，空 = 全部型号（默认空）
  --device-model <M>    复制后的归属型号（默认 null = 通用）

输出与选项：
  --dir <dir>           源目录（默认 config/button-profiles）
  --out <file>          只把聚合结果写到指定文件（用于已安装机器，例如
                        <安装目录>/resources/config/button-profiles.json）
  --bundle-version <v>  指定版本号（默认把 catalog.json 里的末段 +1）
  --no-bump             不自动递增 bundleVersion
  --dry-run             只打印结果，不写盘

示例：
  node scripts/profile-to-preset.mjs --build
  node scripts/profile-to-preset.mjs --check
  node scripts/profile-to-preset.mjs --list --input "%USERPROFILE%\\Desktop\\button-profiles.json"
  node scripts/profile-to-preset.mjs --input button-profiles.json --profile "豆包输入" \\
       --id preset.rc003.doubao --models RC003 --dry-run
`;

function fail(message) {
  console.error(`错误：${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = {
    input: null,
    profile: null,
    id: null,
    name: null,
    models: null,
    deviceModel: null,
    out: null,
    dir: null,
    bundleVersion: null,
    bump: true,
    dryRun: false,
    check: false,
    list: false,
    build: false,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    const next = () => {
      const value = argv[index + 1];
      if (value === undefined || value.startsWith("--")) fail(`参数 ${token} 缺少取值`);
      index += 1;
      return value;
    };
    switch (token) {
      case "--input":
        args.input = next();
        break;
      case "--profile":
        args.profile = next();
        break;
      case "--id":
        args.id = next();
        break;
      case "--name":
        args.name = next();
        break;
      case "--models":
        args.models = next();
        break;
      case "--device-model":
        args.deviceModel = next();
        break;
      case "--out":
        args.out = next();
        break;
      case "--dir":
        args.dir = next();
        break;
      case "--build":
        args.build = true;
        break;
      case "--bundle-version":
        args.bundleVersion = next();
        break;
      case "--no-bump":
        args.bump = false;
        break;
      case "--dry-run":
        args.dryRun = true;
        break;
      case "--check":
        args.check = true;
        break;
      case "--list":
        args.list = true;
        break;
      case "-h":
      case "--help":
        args.help = true;
        break;
      default:
        fail(`未知参数 ${token}（用 --help 查看用法）`);
    }
  }
  return args;
}

function readJsonFile(path, label) {
  if (!existsSync(path)) fail(`找不到${label}：${path}`);
  try {
    // PowerShell / 记事本另存的文件常带 UTF-8 BOM，先剥掉再解析。
    const text = readFileSync(path, "utf8").replace(/^\uFEFF/, "");
    return JSON.parse(text);
  } catch (error) {
    fail(`${label}不是合法 JSON（${path}）：${error.message}`);
  }
}

/** 型号/目标归一化，等价于 Rust 的 `normalized_model` / `normalized_targets`。 */
function normalizeModel(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed.toUpperCase() : null;
}

function normalizeTarget(value) {
  if (typeof value !== "string") return "";
  return value.trim().toLowerCase();
}

function recommendedMacroIds() {
  if (!existsSync(MACRO_CATALOG)) return new Set();
  const catalog = readJsonFile(MACRO_CATALOG, "推荐宏配置");
  const macros = Array.isArray(catalog.macros) ? catalog.macros : [];
  return new Set(macros.map((macro) => macro?.macroId).filter(Boolean));
}

// ---------------------------------------------------------------------------
// 源目录（一方案一文件）→ 聚合文件
// ---------------------------------------------------------------------------

/** 目录里参与聚合的方案文件名（排除 catalog.json 与隐藏文件），按文件名排序。 */
function listProfileFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => name.toLowerCase().endsWith(".json"))
    .filter((name) => name.toLowerCase() !== DIR_MANIFEST)
    .filter((name) => !name.startsWith("."))
    .sort((left, right) => left.localeCompare(right, "en"));
}

/**
 * 读取源目录，返回聚合对象；同时收集结构性问题（不抛出，便于 `--check` 一次报全）。
 *
 * 排序规则：按文件名升序 —— 界面里推荐方案的顺序就是这个顺序，想调顺序就改文件名前缀。
 */
function readProfileDir(dir) {
  const issues = [];
  if (!existsSync(dir)) {
    return { aggregate: null, issues: [`源目录不存在：${dir}`], files: [] };
  }
  const manifestPath = resolve(dir, DIR_MANIFEST);
  let manifest = { schemaVersion: 1, bundleVersion: "" };
  if (existsSync(manifestPath)) {
    try {
      manifest = JSON.parse(readFileSync(manifestPath, "utf8").replace(/^\uFEFF/, ""));
    } catch (error) {
      issues.push(`${DIR_MANIFEST}: 不是合法 JSON：${error.message}`);
    }
  } else {
    issues.push(`缺少 ${DIR_MANIFEST}（至少要写 { "schemaVersion": 1, "bundleVersion": "..." }）`);
  }
  const bundleVersion =
    typeof manifest?.bundleVersion === "string" ? manifest.bundleVersion.trim() : "";
  if (!bundleVersion) issues.push(`${DIR_MANIFEST}: bundleVersion 不能为空`);

  const files = listProfileFiles(dir);
  const profiles = [];
  for (const fileName of files) {
    const path = resolve(dir, fileName);
    let preset;
    try {
      preset = JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
    } catch (error) {
      issues.push(`${fileName}: 不是合法 JSON：${error.message}`);
      continue;
    }
    const stem = basename(fileName, ".json");
    if (typeof preset?.id === "string" && preset.id !== stem) {
      // 文件名与 id 不一致时，改 id 会悄悄改变「谁是这条方案」，早报早改。
      issues.push(`${fileName}: 文件内 id "${preset.id}" 与文件名 "${stem}" 不一致`);
    }
    profiles.push(preset);
  }

  return {
    aggregate: { schemaVersion: manifest?.schemaVersion ?? 1, bundleVersion, profiles },
    issues,
    files,
  };
}

/** 聚合对象序列化为文件内容（先按 2 空格缩进，再由 prettier 统一排版）。 */
function serializeAggregate(aggregate) {
  return `${JSON.stringify(aggregate, null, 2)}\n`;
}

/**
 * 用仓库里已安装的 prettier 格式化聚合文件。
 *
 * 排版统一交给 prettier（和 `pnpm config:build` 一致），脚本本身不实现格式化，
 * 免得两边规则漂移导致 `prettier --check` 失败。找不到 prettier 时只提示，
 * 不阻断——`pnpm config:check` 会再拦一次。
 */
function formatAggregate(path) {
  // 直接跑 prettier 的 bin 脚本（node 调用，不经 shell，避免转义与跨平台问题）。
  const bin = resolve(REPO_ROOT, "apps/shengjian-desktop/node_modules/prettier/bin/prettier.cjs");
  if (!existsSync(bin)) return false;
  const result = spawnSync(process.execPath, [bin, "--write", path], {
    cwd: REPO_ROOT,
    stdio: "ignore",
  });
  return result.status === 0;
}

/** 目录 → 聚合文件；返回写出的路径。校验失败时不写盘。 */
function buildFromDir(dir, outPath = DEFAULT_CATALOG, { dryRun = false, format = true } = {}) {
  const { aggregate, issues } = readProfileDir(dir);
  const allIssues = [...issues];
  if (aggregate) {
    allIssues.push(...validateCatalog(aggregate));
  }
  if (allIssues.length > 0) {
    console.error(`✗ 源目录不合法（${dir}），未生成聚合文件：`);
    for (const issue of allIssues) console.error(`  - ${issue}`);
    process.exit(1);
  }
  const json = serializeAggregate(aggregate);
  if (dryRun) return { aggregate, json, formatted: false };
  writeFileSync(outPath, json, "utf8");
  const formatted = format ? formatAggregate(outPath) : false;
  return { aggregate, json, formatted };
}

/**
 * 比较「目录聚合结果」与「已提交的聚合文件」内容是否一致。
 *
 * 只比**结构**（键排序后深比较），不比字节：聚合文件的排版由 prettier 负责
 * （`pnpm config:build` 会格式化），这里只关心运行时读到的东西对不对。
 */
function describeAggregateDrift(dir, aggregatePath = DEFAULT_CATALOG) {
  if (!existsSync(aggregatePath)) {
    return [`聚合文件缺失：${aggregatePath}（运行 pnpm config:build 生成）`];
  }
  const { aggregate } = readProfileDir(dir);
  if (!aggregate) return [];

  let onDisk;
  try {
    onDisk = JSON.parse(readFileSync(aggregatePath, "utf8").replace(/^\uFEFF/, ""));
  } catch (error) {
    return [`聚合文件不是合法 JSON：${error.message}`, "  修复：pnpm config:build"];
  }
  if (canonicalJson(onDisk) === canonicalJson(aggregate)) return [];

  // 给出可操作的差异摘要，而不是整份 diff。
  const issues = [`聚合文件与源目录不一致：${aggregatePath}`];
  const expectedIds = aggregate.profiles.map((profile) => profile?.id);
  const actualIds = Array.isArray(onDisk?.profiles)
    ? onDisk.profiles.map((profile) => profile?.id)
    : [];
  const missing = expectedIds.filter((id) => !actualIds.includes(id));
  const extra = actualIds.filter((id) => !expectedIds.includes(id));
  if (missing.length) issues.push(`  目录里有但聚合文件没有：${missing.join(", ")}`);
  if (extra.length) issues.push(`  聚合文件有但目录里没有：${extra.join(", ")}`);
  if (onDisk?.bundleVersion !== aggregate.bundleVersion) {
    issues.push(
      `  bundleVersion 不同：目录=${aggregate.bundleVersion} 聚合文件=${onDisk?.bundleVersion}`,
    );
  }
  if (!missing.length && !extra.length && onDisk?.bundleVersion === aggregate.bundleVersion) {
    issues.push("  方案内容有差异（字段不同），运行 pnpm config:build 重新生成");
  }
  issues.push("  修复：pnpm config:build");
  return issues;
}

/** 键排序后的 JSON 文本，用于与排版无关的深比较。 */
function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    const keys = Object.keys(value).sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

/** 把一条推荐方案写进源目录（同名 id 覆盖），返回文件路径。 */
function writeProfileFile(dir, preset) {
  mkdirSync(dir, { recursive: true });
  const path = resolve(dir, `${preset.id}.json`);
  // 先删掉 id 相同但文件名不同的旧文件，避免聚合时出现重复 id。
  const stem = `${preset.id}.json`;
  for (const fileName of listProfileFiles(dir)) {
    if (fileName === stem) continue;
    const candidate = resolve(dir, fileName);
    try {
      const existing = JSON.parse(readFileSync(candidate, "utf8").replace(/^\uFEFF/, ""));
      if (existing?.id === preset.id) unlinkSync(candidate);
    } catch {
      // 解析不了的文件留给 --check 报，不在这里删。
    }
  }
  writeFileSync(path, `${JSON.stringify(preset, null, 2)}\n`, "utf8");
  return path;
}

function validateAction(action, label, macroIds, issues) {
  if (!action || typeof action !== "object" || Array.isArray(action)) {
    issues.push(`${label}: 动作必须是对象`);
    return;
  }
  const type = action.type;
  if (typeof type !== "string" || !ACTION_TYPES.includes(type)) {
    issues.push(`${label}: 未知动作类型 ${JSON.stringify(type)}`);
    return;
  }
  if (type === "run-macro") {
    const macroId = action.macroId;
    if (typeof macroId !== "string" || !macroId.trim()) {
      issues.push(`${label}: run-macro 缺少 macroId`);
    } else if (!macroIds.has(macroId)) {
      issues.push(
        `${label}: run-macro 引用的宏 ${macroId} 不在 config/recommended-macros.json 中（整份目录会被拒绝）`,
      );
    }
  }
  if (type === "shortcut") {
    if (!Array.isArray(action.keys) || action.keys.length === 0) {
      issues.push(`${label}: shortcut 需要有非空 keys 数组`);
    }
  }
  if (type === "mouse" && typeof action.kind !== "string") {
    issues.push(`${label}: mouse 需要有 kind`);
  }
  if (type === "system-action" && typeof action.key !== "string") {
    issues.push(`${label}: system-action 需要有 key`);
  }
}

/**
 * 与 `button_presets::validate_catalog` 对齐的校验。
 * 任一条不合规都会让整个目录被拒绝（桌面端回退内嵌副本），所以这里全部收集后一起报。
 */
function validateCatalog(catalog, macroIds = recommendedMacroIds()) {
  const issues = [];
  if (!catalog || typeof catalog !== "object" || Array.isArray(catalog)) {
    return ["目录必须是 JSON 对象"];
  }
  if (!Array.isArray(catalog.profiles)) {
    return ["目录缺少 profiles 数组"];
  }
  if (catalog.profiles.length > MAX_BUTTON_PRESETS) {
    issues.push(`推荐方案过多：${catalog.profiles.length} > ${MAX_BUTTON_PRESETS}`);
  }
  if (typeof catalog.bundleVersion !== "string" || !catalog.bundleVersion.trim()) {
    issues.push("bundleVersion 必须是非空字符串");
  }

  const seenIds = new Set();
  catalog.profiles.forEach((preset, index) => {
    const rawId = typeof preset?.id === "string" ? preset.id : "";
    const label = rawId.trim() ? rawId : `<第 ${index + 1} 条：空 id>`;
    if (!rawId.startsWith(PRESET_ID_PREFIX) || rawId.length === PRESET_ID_PREFIX.length) {
      issues.push(`${label}: id 必须以 "${PRESET_ID_PREFIX}" 开头`);
    }
    if (rawId !== rawId.trim()) {
      issues.push(`${label}: id 前后不能有空白`);
    }
    if (seenIds.has(rawId.trim())) {
      issues.push(`${label}: id 重复`);
    }
    seenIds.add(rawId.trim());
    if (preset?.isDefault) {
      issues.push(`${label}: 推荐方案不能是默认方案（isDefault）`);
    }
    if (typeof preset?.name !== "string" || !preset.name.trim()) {
      issues.push(`${label}: name 不能为空`);
    }
    if (preset?.compatibleModels !== undefined) {
      if (!Array.isArray(preset.compatibleModels)) {
        issues.push(`${label}: compatibleModels 必须是数组`);
      } else if (preset.compatibleModels.some((model) => !normalizeModel(model))) {
        issues.push(`${label}: compatibleModels 里有空型号`);
      }
    }
    if (preset?.appTargets !== undefined && !Array.isArray(preset.appTargets)) {
      issues.push(`${label}: appTargets 必须是数组`);
    }
    if (preset?.holdRepeat !== undefined && !TRIGGERS.includes(preset.holdRepeat)) {
      issues.push(`${label}: holdRepeat 只能是 ${TRIGGERS.join(" / ")}`);
    }

    const bindings = preset?.bindings;
    if (bindings !== undefined && (typeof bindings !== "object" || Array.isArray(bindings))) {
      issues.push(`${label}: bindings 必须是对象`);
    } else if (bindings) {
      const buttons = Object.keys(bindings);
      if (buttons.length === 0) {
        issues.push(`${label}: bindings 为空，推荐方案没有任何绑定`);
      }
      for (const button of buttons) {
        if (!PROFILE_BUTTONS.includes(button)) {
          issues.push(`${label}: 按键 ${button} 不可在方案里编辑（整份目录会被拒绝）`);
        }
        const triggers = bindings[button];
        if (!triggers || typeof triggers !== "object" || Array.isArray(triggers)) {
          issues.push(`${label}: ${button} 的触发绑定必须是对象`);
          continue;
        }
        for (const [trigger, action] of Object.entries(triggers)) {
          if (!TRIGGERS.includes(trigger)) {
            issues.push(`${label}: ${button} 的触发 ${trigger} 不合法`);
          }
          validateAction(action, `${label} ${button}.${trigger}`, macroIds, issues);
        }
      }
    }

    const byButton = preset?.holdRepeatByButton;
    if (byButton !== undefined) {
      if (!byButton || typeof byButton !== "object" || Array.isArray(byButton)) {
        issues.push(`${label}: holdRepeatByButton 必须是对象`);
      } else {
        for (const [button, trigger] of Object.entries(byButton)) {
          if (!PROFILE_BUTTONS.includes(button)) {
            issues.push(`${label}: holdRepeatByButton 含不可编辑按键 ${button}`);
          }
          if (trigger !== null && !TRIGGERS.includes(trigger)) {
            issues.push(`${label}: holdRepeatByButton.${button} 只能是 null 或 ${TRIGGERS.join("/")}`);
          }
        }
      }
    }
  });
  return issues;
}

/** 从导出文件里取出方案数组（兼容导出文档 / 裸数组 / 单条方案）。 */
function profilesFromDocument(document, path) {
  if (Array.isArray(document)) return document;
  if (document && Array.isArray(document.profiles)) return document.profiles;
  if (document && typeof document === "object" && "bindings" in document) return [document];
  fail(`无法从 ${path} 读取方案：既没有 profiles 数组，也不是单条方案对象`);
}

function describeProfile(profile, index) {
  const name = typeof profile?.name === "string" ? profile.name : "<无名>";
  const bindCount = profile?.bindings ? Object.keys(profile.bindings).length : 0;
  const flag = profile?.isDefault ? "（默认方案，不能转推荐方案）" : "";
  return `  ${index + 1}. ${name}  id=${profile?.id ?? "<空>"}  绑定=${bindCount}${flag}`;
}

function selectProfile(profiles, selector) {
  if (selector === null) {
    const candidates = profiles.filter((profile) => !profile?.isDefault);
    if (candidates.length === 1) return candidates[0];
    fail(
      candidates.length === 0
        ? "导出文件里没有可转换的方案（只剩默认方案）"
        : `导出文件里有 ${candidates.length} 条可选方案，请用 --profile 指定：\n${profiles
            .map(describeProfile)
            .join("\n")}`,
    );
  }
  const asIndex = /^\d+$/.test(selector) ? Number(selector) : null;
  if (asIndex !== null) {
    const profile = profiles[asIndex - 1];
    if (!profile) fail(`--profile ${selector} 超出范围（共 ${profiles.length} 条）`);
    return profile;
  }
  const byId = profiles.find((profile) => profile?.id === selector);
  if (byId) return byId;
  const byName = profiles.find((profile) => profile?.name === selector);
  if (byName) return byName;
  fail(`找不到方案 "${selector}"，现有方案：\n${profiles.map(describeProfile).join("\n")}`);
}

function buildPreset(source, args) {
  const sourceId = typeof source?.id === "string" ? source.id : "";
  const id = args.id ?? (sourceId.startsWith(PRESET_ID_PREFIX) ? sourceId : null);
  if (!id) {
    fail(
      `需要 --id：源方案 id "${sourceId}" 不以 "${PRESET_ID_PREFIX}" 开头，` +
        `例如 --id preset.rc003.demo`,
    );
  }
  if (!id.startsWith(PRESET_ID_PREFIX) || id.length === PRESET_ID_PREFIX.length) {
    fail(`--id 必须以 "${PRESET_ID_PREFIX}" 开头且后面还有内容`);
  }
  const models = (args.models ?? "")
    .split(",")
    .map((model) => normalizeModel(model))
    .filter(Boolean);

  // 字段顺序与 config/button-profiles.json 现有条目一致；空值一律省略，
  // 让文件保持可读（Rust 侧这些字段都是 #[serde(default)]）。
  const preset = {
    id,
    name: (args.name ?? source?.name ?? "").trim(),
    deviceModel: normalizeModel(args.deviceModel),
    compatibleModels: models,
    appTargets: Array.isArray(source?.appTargets) ? source.appTargets.slice() : [],
    autoApply: Boolean(source?.autoApply),
  };
  if (source?.holdRepeat) preset.holdRepeat = source.holdRepeat;
  if (source?.holdRepeatByButton && Object.keys(source.holdRepeatByButton).length > 0) {
    preset.holdRepeatByButton = source.holdRepeatByButton;
  }
  if (source?.voiceInputTool) preset.voiceInputTool = source.voiceInputTool;
  preset.bindings = source?.bindings && typeof source.bindings === "object" ? source.bindings : {};
  return preset;
}

/** 版本号末段 +1（"2026.11.1" → "2026.11.2"）；无数字时追加 ".1"。 */
function bumpVersion(current) {
  const text = typeof current === "string" ? current.trim() : "";
  if (!text) return "1";
  const parts = text.split(".");
  const lastIndex = parts.length - 1;
  if (/^\d+$/.test(parts[lastIndex])) {
    parts[lastIndex] = String(Number(parts[lastIndex]) + 1);
    return parts.join(".");
  }
  return `${text}.1`;
}

function writeCatalog(path, catalog) {
  const json = `${JSON.stringify(catalog, null, 2)}\n`;
  writeFileSync(path, json, "utf8");
}

// ---------------------------------------------------------------------------
// 其余三个配置目录的轻量结构校验（对齐 Rust 侧的主要拒绝原因，
// 不追求逐字段等价：给发布前一个快速自检，避免改坏 JSON 后静默回退内嵌副本）。
// ---------------------------------------------------------------------------

function validateAppPresets(catalog) {
  const issues = [];
  if (!catalog || typeof catalog !== "object" || !Array.isArray(catalog.apps)) {
    return ["缺少 apps 数组"];
  }
  if (catalog.apps.length === 0) issues.push("apps 为空（Rust 侧会拒绝并回退内嵌副本）");
  const seen = new Set();
  catalog.apps.forEach((app, index) => {
    const id = typeof app?.id === "string" ? app.id.trim() : "";
    const label = id || `<第 ${index + 1} 条>`;
    if (!id) issues.push(`${label}: id 不能为空`);
    if (typeof app?.name !== "string" || !app.name.trim()) issues.push(`${label}: name 不能为空`);
    if (id) {
      if (seen.has(id)) issues.push(`${label}: id 重复`);
      seen.add(id);
    }
    for (const field of ["exeNames", "installPaths", "shortcutNames"]) {
      if (app?.[field] !== undefined && !Array.isArray(app[field])) {
        issues.push(`${label}: ${field} 必须是数组`);
      }
    }
  });
  return issues;
}

function validateVoiceTools(catalog) {
  const issues = [];
  if (!catalog || typeof catalog !== "object" || !Array.isArray(catalog.tools)) {
    return ["缺少 tools 数组"];
  }
  if (catalog.tools.length === 0) issues.push("tools 为空（Rust 侧会拒绝并回退内嵌副本）");
  const seen = new Set();
  catalog.tools.forEach((tool, index) => {
    const id = typeof tool?.id === "string" ? tool.id.trim() : "";
    const label = id || `<第 ${index + 1} 条>`;
    if (!id) issues.push(`${label}: id 不能为空`);
    if (typeof tool?.label !== "string" || !tool.label.trim()) issues.push(`${label}: label 不能为空`);
    if (typeof tool?.kind !== "string" || !tool.kind.trim()) {
      issues.push(`${label}: kind 不能为空（wechat / doubao / vokie / chatterfly / custom）`);
    }
    if (id) {
      const key = id.toLowerCase();
      if (seen.has(key)) issues.push(`${label}: id 重复（不区分大小写）`);
      seen.add(key);
    }
    if (tool?.defaultHotkey !== undefined && !Array.isArray(tool.defaultHotkey)) {
      issues.push(`${label}: defaultHotkey 必须是数组`);
    }
  });
  return issues;
}

function validateRecommendedMacros(catalog) {
  const issues = [];
  if (!catalog || typeof catalog !== "object" || !Array.isArray(catalog.macros)) {
    return ["缺少 macros 数组"];
  }
  const seen = new Set();
  catalog.macros.forEach((macro, index) => {
    const id = typeof macro?.macroId === "string" ? macro.macroId.trim() : "";
    const label = id || `<第 ${index + 1} 条>`;
    if (!id) {
      issues.push(`${label}: macroId 不能为空`);
      return;
    }
    if (seen.has(id)) issues.push(`${label}: macroId 重复`);
    seen.add(id);
    if (!Array.isArray(macro?.steps) || macro.steps.length === 0) {
      issues.push(`${label}: steps 不能为空`);
    }
  });
  return issues;
}

const CATALOGS = {
  "button-profiles.json": {
    label: "推荐按键方案",
    validate: validateCatalog,
    summarize: (catalog) => `方案数=${catalog.profiles?.length ?? 0} bundleVersion=${catalog.bundleVersion ?? "-"}`,
  },
  "recommended-macros.json": {
    label: "推荐宏",
    validate: validateRecommendedMacros,
    summarize: (catalog) => `推荐宏=${catalog.macros?.length ?? 0} bundleVersion=${catalog.bundleVersion ?? "-"}`,
  },
  "app-presets.json": {
    label: "推荐应用",
    validate: validateAppPresets,
    summarize: (catalog) => `应用=${catalog.apps?.length ?? 0}`,
  },
  "voice-tools.json": {
    label: "语音输入工具",
    validate: validateVoiceTools,
    summarize: (catalog) => `工具=${catalog.tools?.length ?? 0}`,
  },
};

function checkOne(fileName, path) {
  const meta = CATALOGS[fileName];
  let catalog;
  try {
    // PowerShell / 记事本另存的文件常带 UTF-8 BOM，先剥掉再解析。
    catalog = JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
  } catch (error) {
    console.error(`✗ ${meta.label}（${path}）`);
    console.error(`  - 不是合法 JSON：${error.message}`);
    return false;
  }
  const issues = meta.validate(catalog);
  if (issues.length > 0) {
    console.error(`✗ ${meta.label}（${path}）`);
    for (const issue of issues) console.error(`  - ${issue}`);
    return false;
  }
  console.log(`✓ ${meta.label}  ${path}`);
  console.log(`  ${meta.summarize(catalog)}`);
  return true;
}

/** 仓库内模式：源目录 + 聚合一致性 + 其余三个目录文件。 */
function checkRepository() {
  let ok = true;

  // 1) 一方案一文件的源目录
  const { aggregate, issues, files } = readProfileDir(PROFILE_DIR);
  const dirIssues = [...issues];
  if (aggregate) dirIssues.push(...validateCatalog(aggregate));
  if (dirIssues.length > 0) {
    console.error(`✗ 推荐按键方案源目录（${PROFILE_DIR}）`);
    for (const issue of dirIssues) console.error(`  - ${issue}`);
    ok = false;
  } else {
    console.log(`✓ 推荐按键方案源目录  ${PROFILE_DIR}`);
    console.log(
      `  ${files.length} 个文件  bundleVersion=${aggregate.bundleVersion}  型号=${
        summarizeModels(aggregate) || "-"
      }`,
    );
    for (const fileName of files) console.log(`    · ${fileName}`);
  }

  // 2) 聚合文件必须与目录一致（改了目录忘记 --build 时在这里拦住）
  const drift = describeAggregateDrift(PROFILE_DIR, DEFAULT_CATALOG);
  if (drift.length > 0) {
    console.error(`✗ 推荐按键方案聚合文件（${DEFAULT_CATALOG}）`);
    for (const issue of drift) console.error(`  - ${issue}`);
    ok = false;
  } else {
    console.log(`✓ 推荐按键方案聚合文件  ${DEFAULT_CATALOG}`);
    console.log(`  与源目录一致  方案数=${aggregate?.profiles.length ?? 0}`);
  }

  // 3) 其余三个目录文件（button-profiles 已由上面两步覆盖）
  for (const fileName of Object.keys(CATALOGS)) {
    if (fileName === "button-profiles.json") continue;
    const path = resolve(CONFIG_DIR, fileName);
    if (!existsSync(path)) {
      console.log(`· ${CATALOGS[fileName].label}  ${path}（不存在，跳过）`);
      continue;
    }
    if (!checkOne(fileName, path)) ok = false;
  }
  return ok;
}

function summarizeModels(catalog) {
  const models = new Set();
  for (const preset of catalog?.profiles ?? []) {
    const list = Array.isArray(preset?.compatibleModels) ? preset.compatibleModels : [];
    if (list.length === 0) models.add("通用");
    else list.forEach((model) => models.add(model));
  }
  return [...models].join(" / ");
}

/** 已安装目录模式：只校验产物本身（没有源目录）。 */
function checkInstalled(target) {
  const isFile = /\.[jt]s$|\.json$/i.test(target) && !target.endsWith("\\") && !target.endsWith("/");
  const fileNameOf = (path) => path.split(/[\\/]/).pop();
  const files = isFile ? [fileNameOf(target)] : Object.keys(CATALOGS);
  let ok = true;
  for (const fileName of files) {
    const path = isFile ? target : resolve(target, fileName);
    if (!CATALOGS[fileName]) {
      fail(`不认识的文件名：${fileName}（可选：${Object.keys(CATALOGS).join(" / ")}）`);
    }
    if (!existsSync(path)) {
      console.log(`· ${CATALOGS[fileName].label}  ${path}（不存在，跳过）`);
      continue;
    }
    if (!checkOne(fileName, path)) ok = false;
  }
  return ok;
}

function runCheck(args) {
  // --out 指向已安装目录/文件时只校验产物；否则是仓库内模式。
  if (args.out) {
    const target = resolve(process.cwd(), args.out);
    console.log(`校验配置文件：${target}\n`);
    if (!checkInstalled(target)) {
      console.error(
        "\n任意一条不合规都会让桌面端拒绝整份目录并回退到内嵌副本，界面上看是「配置没更新」。",
      );
      process.exit(1);
    }
    console.log("\n全部通过。");
    return;
  }

  console.log(`校验配置：${CONFIG_DIR}\n`);
  if (!checkRepository()) {
    console.error(
      "\n修复后重跑。源目录改完记得 `pnpm config:build`；任何一条不合规都会让桌面端" +
        "拒绝整份目录并回退到内嵌副本，界面上看是「配置没更新」。",
    );
    process.exit(1);
  }
  console.log("\n全部通过。");
}

function runBuild(args) {
  const dir = args.dir ? resolve(process.cwd(), args.dir) : PROFILE_DIR;
  const out = args.out ? resolve(process.cwd(), args.out) : DEFAULT_CATALOG;
  const { aggregate, formatted } = buildFromDir(dir, out, { dryRun: args.dryRun });
  const count = aggregate.profiles.length;
  if (args.dryRun) {
    console.log(`--dry-run：${dir} → ${out}（${count} 条方案，未写盘）`);
    console.log(serializeAggregate(aggregate));
    return;
  }
  console.log(`✓ 已生成 ${out}`);
  console.log(
    `  方案数=${count} bundleVersion=${aggregate.bundleVersion} 型号=${
      summarizeModels(aggregate) || "-"
    }`,
  );
  console.log("  运行时与安装包读取的是这个聚合文件（由 include_str! 内嵌兜底）。");
  if (!formatted) {
    console.log("  提示：没找到 prettier，聚合文件未排版；跑一次 `pnpm config:build` 可格式化。");
  }
}


function runList(args) {
  if (!args.input) fail("--list 需要 --input");
  const inputPath = resolve(process.cwd(), args.input);
  const profiles = profilesFromDocument(readJsonFile(inputPath, "导出文件"), inputPath);
  console.log(`${inputPath}`);
  console.log(`共 ${profiles.length} 条方案：`);
  console.log(profiles.map(describeProfile).join("\n"));
}

function runConvert(args) {
  if (!args.input) fail("需要 --input（应用「按键」页导出的 button-profiles.json）");
  const inputPath = resolve(process.cwd(), args.input);
  const profiles = profilesFromDocument(readJsonFile(inputPath, "导出文件"), inputPath);
  if (profiles.length === 0) fail(`${inputPath} 里没有任何方案`);

  const source = selectProfile(profiles, args.profile);
  if (source?.isDefault) {
    fail("选中的是默认方案：默认方案不能作为推荐方案（会被校验拒绝），请另建普通方案后导出");
  }
  const preset = buildPreset(source, args);

  const dir = args.dir ? resolve(process.cwd(), args.dir) : PROFILE_DIR;

  // --out：只把聚合结果写到指定文件（已安装机器），不动源目录。
  if (args.out) {
    const target = resolve(process.cwd(), args.out);
    const catalog = existsSync(target)
      ? readJsonFile(target, "推荐方案目录")
      : { schemaVersion: 1, bundleVersion: "1", profiles: [] };
    if (!Array.isArray(catalog.profiles)) catalog.profiles = [];
    catalog.schemaVersion = 1;
    catalog.bundleVersion =
      args.bundleVersion ?? (args.bump ? bumpVersion(catalog.bundleVersion) : catalog.bundleVersion);
    const existingIndex = catalog.profiles.findIndex((item) => item?.id === preset.id);
    const replaced = existingIndex >= 0;
    if (replaced) catalog.profiles[existingIndex] = preset;
    else catalog.profiles.push(preset);

    const issues = validateCatalog(catalog);
    if (issues.length > 0) {
      console.error("✗ 转换后的目录不合法，未写盘：");
      for (const issue of issues) console.error(`  - ${issue}`);
      process.exit(1);
    }
    reportPreset(preset, catalog.bundleVersion, replaced);
    if (args.dryRun) {
      console.log("\n--dry-run：未写盘，下面是将要写入的条目：");
      console.log(JSON.stringify(preset, null, 2));
      return;
    }
    writeFileSync(target, serializeAggregate(catalog), "utf8");
    formatAggregate(target);
    console.log(`\n已写入 ${target}`);
    console.log("重启应用即可生效。");
    return;
  }

  // 默认：写进一方案一文件的源目录，再重建聚合文件。
  const manifestPath = resolve(dir, DIR_MANIFEST);
  const manifest = existsSync(manifestPath)
    ? readJsonFile(manifestPath, "源目录清单")
    : { schemaVersion: 1, bundleVersion: "1" };
  const existingFiles = listProfileFiles(dir);
  let replaced = false;
  for (const fileName of existingFiles) {
    try {
      const existing = JSON.parse(
        readFileSync(resolve(dir, fileName), "utf8").replace(/^\uFEFF/, ""),
      );
      if (existing?.id === preset.id) replaced = true;
    } catch {
      // 坏文件留给 --check 报。
    }
  }
  const nextVersion =
    args.bundleVersion ?? (args.bump ? bumpVersion(manifest.bundleVersion) : manifest.bundleVersion);

  if (args.dryRun) {
    reportPreset(preset, nextVersion, replaced);
    console.log(`\n--dry-run：将写入 ${resolve(dir, `${preset.id}.json`)} 并重建聚合文件，未写盘。`);
    console.log(JSON.stringify(preset, null, 2));
    return;
  }

  const presetPath = writeProfileFile(dir, preset);
  manifest.schemaVersion = manifest.schemaVersion ?? 1;
  manifest.bundleVersion = nextVersion;
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  reportPreset(preset, nextVersion, replaced);
  console.log(`\n已写入 ${presetPath}`);
  buildFromDir(dir, DEFAULT_CATALOG);
  console.log(`已重建聚合文件 ${DEFAULT_CATALOG}`);
  console.log("\n检查一遍：pnpm config:check");
}

function reportPreset(preset, bundleVersion, replaced) {
  console.log(`${replaced ? "已替换" : "已新增"}推荐方案：${preset.name}（${preset.id}）`);
  console.log(
    `  适配型号：${preset.compatibleModels.length ? preset.compatibleModels.join(" / ") : "通用"}`,
  );
  console.log(`  绑定按键：${Object.keys(preset.bindings).join(" / ") || "（空）"}`);
  console.log(`  应用目标：${preset.appTargets.join(" / ") || "（无，仅手动复制使用）"}`);
  console.log(`  自动应用：${preset.autoApply ? "是" : "否"}`);
  console.log(`  bundleVersion：${bundleVersion}（改过之后用户在按键页会看到「有更新」）`);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(USAGE);
    return;
  }
  if (args.build) {
    runBuild(args);
    return;
  }
  if (args.check) {
    runCheck(args);
    return;
  }
  if (args.list) {
    runList(args);
    return;
  }
  if (!args.input) {
    console.log(USAGE);
    process.exit(1);
  }
  runConvert(args);
}

main();
