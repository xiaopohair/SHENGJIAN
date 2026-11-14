#!/usr/bin/env node
// 把「按键」页导出的用户方案库转成推荐方案目录（`config/button-profiles.json`）。
//
// 应用导出的文件是**用户方案库**结构：
//   { "schemaVersion": 5, "profiles": [ ... ], "activeManualProfileId": "..." }
// 推荐目录是另一种结构，还有额外约束（id 前缀、禁止 isDefault、适配型号等）：
//   { "schemaVersion": 1, "bundleVersion": "2026.11.1", "profiles": [ ... ] }
//
// 本脚本负责这段转换，并在写盘前按 `shengjian-domain::button_presets::validate_catalog`
// 的同一套规则校验，避免「改完重启才发现整份目录被拒、静默回退内嵌副本」。
//
// 用法：
//   node scripts/profile-to-preset.mjs --list --input exported.json
//   node scripts/profile-to-preset.mjs --input exported.json --profile "豆包输入" \
//        --id preset.rc003.doubao --models RC003 --dry-run
//   node scripts/profile-to-preset.mjs --input exported.json --profile 2 --id preset.x
//   node scripts/profile-to-preset.mjs --check            # 校验仓库里的配置
//   node scripts/profile-to-preset.mjs --check --out "%LOCALAPPDATA%\\声键\\resources\\config\\button-profiles.json"

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "..");
const DEFAULT_CATALOG = resolve(REPO_ROOT, "config/button-profiles.json");
const MACRO_CATALOG = resolve(REPO_ROOT, "config/recommended-macros.json");

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

const USAGE = `把应用导出的按键方案转成推荐方案目录条目。

必需参数：
  --input <file>        应用「按键」页导出的 button-profiles.json

选择要转换的方案（三选一）：
  --profile <name|id|index>   方案名 / 方案 id / 1 起的序号；只有一条非默认方案时可省略
  --list                      只列出导出文件里的方案，不做转换

推荐目录标识（转换模式）：
  --id <preset.xxx>     推荐方案 id，必须以 "preset." 开头；源 id 已是 preset.* 时可省略
  --name <显示名>        覆盖方案名（默认沿用导出时的名字）
  --models <A,B>        适配型号，空 = 全部型号（默认空）
  --device-model <M>    复制后的归属型号（默认 null = 通用）

输出：
  --out <file>          目标目录文件（默认 config/button-profiles.json；
                        已安装也可指向 <安装目录>/resources/config/button-profiles.json）
  --bundle-version <v>  指定版本号（默认把现有版本号末段 +1）
  --no-bump             不自动递增 bundleVersion
  --dry-run             只打印结果，不写盘
  --check               不转换，只校验配置文件。默认校验 config/ 下四个目录文件；
                        --out 给目录则校验该目录，给具体文件名则只校验那一个

示例：
  node scripts/profile-to-preset.mjs --list --input "%USERPROFILE%\\Desktop\\button-profiles.json"
  node scripts/profile-to-preset.mjs --input button-profiles.json --profile "豆包输入" \\
       --id preset.rc003.doubao --models RC003 --dry-run
  node scripts/profile-to-preset.mjs --check
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
    bundleVersion: null,
    bump: true,
    dryRun: false,
    check: false,
    list: false,
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

function runCheck(args) {
  const base = resolve(REPO_ROOT, "config");
  const target = args.out ? resolve(process.cwd(), args.out) : base;
  const isFile = /\.[jt]s$|\.json$/i.test(target) && !target.endsWith("\\") && !target.endsWith("/");
  let files;
  if (isFile) {
    const fileName = target.split(/[\\/]/).pop();
    if (!CATALOGS[fileName]) {
      fail(`--out 指向的文件名不认识：${fileName}（可选：${Object.keys(CATALOGS).join(" / ")}）`);
    }
    files = [fileName];
  } else {
    files = Object.keys(CATALOGS);
  }

  console.log(`校验配置目录：${isFile ? target : target}\n`);
  let ok = true;
  for (const fileName of files) {
    const path = isFile ? target : resolve(target, fileName);
    if (!existsSync(path)) {
      // 已安装的目录里可能缺文件（旧版本安装包），缺文件本身不会让程序崩溃，
      // 只提示一句，不算失败。
      console.log(`· ${CATALOGS[fileName].label}  ${path}（不存在，跳过）`);
      continue;
    }
    if (!checkOne(fileName, path)) ok = false;
  }

  if (!ok) {
    console.error(
      "\n任意一条不合规都会让桌面端拒绝整份目录并回退到内嵌副本，界面上看是「配置没更新」。",
    );
    process.exit(1);
  }
  console.log("\n全部通过。");
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

  const target = args.out ? resolve(process.cwd(), args.out) : DEFAULT_CATALOG;
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

  console.log(`${replaced ? "已替换" : "已新增"}推荐方案：${preset.name}（${preset.id}）`);
  console.log(`  适配型号：${preset.compatibleModels.length ? preset.compatibleModels.join(" / ") : "通用"}`);
  console.log(`  绑定按键：${Object.keys(preset.bindings).join(" / ") || "（空）"}`);
  console.log(`  应用目标：${preset.appTargets.join(" / ") || "（无，仅手动复制使用）"}`);
  console.log(`  自动应用：${preset.autoApply ? "是" : "否"}`);
  console.log(`  bundleVersion：${catalog.bundleVersion}（改过之后用户在按键页会看到「有更新」）`);

  if (args.dryRun) {
    console.log("\n--dry-run：未写盘，下面是将要写入的条目：");
    console.log(JSON.stringify(preset, null, 2));
    return;
  }
  writeCatalog(target, catalog);
  console.log(`\n已写入 ${target}`);
  console.log("重新构建安装包，或直接改安装目录里的 resources/config/button-profiles.json 后重启应用即可生效。");
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(USAGE);
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
