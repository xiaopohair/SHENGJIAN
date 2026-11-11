# 开发计划：方案按设备型号配置（第一步）

状态：**待实施（2026-11-12 制定）**

## 背景

按键方案当前有两个缺陷：

1. 方案与设备型号无关，所有型号共用同一份方案列表
2. 界面设备状态曾硬编码为「RC003 · 已连接」（已于 2026-11-11 修复）

真实约束是：不同型号的按键布局不同——RC003 有 TV 键，ARN9 没有。方案必须按型号隔离。

## 已确认的决策

| 议题 | 决策 |
| --- | --- |
| 型号识别策略 | **C 降级方案**：识别不到型号时回落到「上次连接的型号」，再回落「通用」默认方案；后续用真机校验补强 |
| 现有数据归属 | 全部归给 **RC003** |
| 默认方案显示名 | **「默认」** |
| 型号字符串格式 | **全大写**，如 `RC003`（与 `remote_model_from_model_number` 一致） |
| 「通用」兜底方案 | **需要**，型号完全未知且无历史记录时使用 |
| 实施范围 | 分两步走；**第一步只做模型结构，不改变按键布局与产品图** |

## 语义更正：全局方案 → 默认方案

「全局方案」的语义改为「默认方案」，且不再是全库唯一实体，而是 **(型号 × 默认方案)** 的集合。

| | 旧「全局方案」 | 新「默认方案」 |
| --- | --- | --- |
| 数量 | 全库 1 个 | 每个型号 1 个 + 1 个通用 |
| 标识 | 固定 id `global` | `isDefault` 布尔标记 |
| 归属 | 不属于任何设备 | 属于具体型号（或通用） |
| 继承来源 | 工厂 `button-mappings.json` | 工厂（不变） |
| 被谁继承 | 所有方案 | **同型号**方案；通用默认被所有型号继承 |

## 分层继承模型（第一步）

```
工厂 button-mappings.json
  └─ 该型号的默认方案（isDefault = true，deviceModel = "RC003"）
       └─ 该型号的其他方案（deviceModel = "RC003"）
            └─ 通用方案（deviceModel = null，对所有型号可见）
```

方案筛选顺序（`ButtonProfileService::selected_profile`）：

1. 手动激活的方案，且与当前型号兼容
2. 否则：`auto_apply` 且匹配前台 exe 的方案（需型号兼容）
3. 否则：当前型号的默认方案
4. 型号未知：用「上次连接的型号」重复 1–3
5. 仍然未知：通用默认方案

型号兼容判定：`profile.deviceModel === null` 或 `profile.deviceModel === currentModel`。

## 存储布局

```
%LOCALAPPDATA%\Shengjian\
├── profiles.json                             索引：schemaVersion + activeManualProfileId
└── profiles\
    ├── RC003-默认-7f3a2b1c.json              默认方案（不可删除）
    ├── RC003-音乐播放-9d4e5f6a.json           普通方案
    └── 通用-默认-00000000.json                 通用兜底默认方案
```

文件名规则：`<型号或「通用」>-<清洗后名称>-<id 尾段>.json`

- 型号段便于分享时识别
- id 尾段避免重名冲突
- 清洗规则沿用现有 `sanitize_profile_name`（Windows 非法字符 → `-`、折叠空白、限长 40）

## 实施阶段

### 阶段 1：契约层

**文件**：`crates/shengjian-contract/src/button_profile.rs`、`src/lib.rs`

新增字段：

```rust
pub struct ButtonProfile {
    // …现有字段
    /// 归属设备型号（全大写，如 "RC003"）。None = 通用方案，对所有型号可见。
    #[serde(default)]
    pub device_model: Option<String>,
    /// 是否为该型号的默认方案：不可删除、不可改名、不关联应用、不自动应用。
    #[serde(default)]
    pub is_default: bool,
}
```

语义更名：

| 删除 | 新增 |
| --- | --- |
| `GLOBAL_PROFILE_ID`（`"global"`） | — |
| `GLOBAL_PROFILE_NAME`（`"全局"`） | `DEFAULT_PROFILE_NAME = "默认"` |
| `ButtonProfile::global()` | `ButtonProfile::default_for(model: Option<&str>)` |

`normalized()` 新增约束：

1. 每个出现过的型号（含 `None` 通用）必须有且仅有一个 `is_default`
2. 默认方案名称强制为「默认」、`app_targets` 清空、`auto_apply = false`
3. 默认方案排在同型号方案最前
4. 缺失时自动补建，保证列表非空

**暂不改** `PROFILE_BUTTONS`（保持 12 键 RC003 布局），加 `// STEP 2` 标记。

### 阶段 2：存储层

**文件**：`crates/shengjian-windows-adapter/src/lib.rs`

- `profile_file_name()` 增加型号段
- `load()`：一次性迁移旧格式（无 `deviceModel` 或 id 为 `global`）
- 迁移动作：全部方案写入 `device_model = "RC003"`；`global` 改为 `is_default = true`、名称「默认」；旧文件改名 `.migrated.bak`

**迁移安全**：迁移前把 `profiles/` 整目录复制为 `profiles.backup-<时间戳>`，失败可从备份恢复。

### 阶段 3：服务层

**文件**：`crates/shengjian-app/src/profiles.rs`、`src/engine.rs`

```rust
pub fn set_current_device_model(&self, model: Option<String>);
pub fn current_device_model(&self) -> Option<String>;
```

- `layered_base()` 改为取「当前型号的默认方案」
- `selected_profile()` 按型号兼容性筛选
- `engine` 在连接快照变化时调用 `set_current_device_model(remote.model)`

### 阶段 4：前端

**文件**：`apps/shengjian-desktop/src/lib/buttons.ts`、`pages/ButtonsPage.vue`、`components/ProfileWorkbench.vue`

- 删除 `GLOBAL_PROFILE_ID` / `isGlobalProfile`；新增 `isDefaultProfile(profile)`
- `ProfileDraft` 增加 `deviceModel: string | null`、`isDefault: boolean`
- 方案列表按当前型号过滤；型号来源：`engineStore.snapshot.connection.remote.model` → `localStorage` 上次型号 → `null`
- 默认方案：隐藏删除按钮，属性抽屉中禁用名称与应用目标编辑
- 记住上次连接型号（新增 `localStorage` 键 `shengjian.last-model`）

### 阶段 5：测试

**Rust**

| 测试 | 覆盖点 |
| --- | --- |
| `default_profile_created_per_model` | 每个型号恰好一个默认方案 |
| `normalized_enforces_default_shape` | 名称/无应用/不自动应用/排首位 |
| `layering_uses_model_default` | 未绑定处继承该型号默认方案，再退工厂 |
| `model_mismatched_profile_is_ignored` | 其他型号方案不生效 |
| `unknown_model_falls_back_to_generic` | 型号未知的回落链 |
| `storage_file_name_includes_model` | 文件名规则 |
| `legacy_profiles_migrate_to_rc003` | 旧数据迁移 |

**前端**：`ButtonsPage.test.ts` 增加默认方案识别与按型号过滤用例。

### 阶段 6：真机验证

1. 未连接：显示上次型号的方案列表 + 该型号默认方案
2. 连接 RC003：只显示 RC003 方案
3. 默认方案无法删除、无法改名
4. 编辑默认方案按键 → 其他 RC003 方案继承到新值
5. 重置默认方案 → 回到工厂默认
6. 旧数据迁移后方案不丢、文件名正确

## 风险与对策

| 风险 | 对策 |
| --- | --- |
| 迁移丢数据 | 迁移前整目录备份；旧文件留 `.migrated.bak` |
| 型号未知时看不到方案 | 「上次型号」兜底；首次启动才用通用默认 |
| `isDefault` 与固定 id 双轨 | 彻底移除 `global` 常量，只认 `isDefault` |
| 默认方案被误删 | 后端 `normalized()` 强制重建 + 前端隐藏按钮 |
| RC001/ARN9 型号误判 | 本步不启用它们的方案；`device-identity.md` 解冻后再做第二步 |

## 不在本次范围（第二步）

以下需要真机证据与额外资源，留到第二步：

- ARN9 / RC001 的按键布局（ARN9 无 TV 键）
- `PROFILE_BUTTONS` 改为按型号返回（`buttons_for_model`）
- RC001 / ARN9 / 通用三套产品图与热区坐标标定
- 型号识别修复（见 `device-identity.md`，需采集 GATT Model Number、HID product string、是否存在 TV 键）

## 参考

- `docs/device-identity.md` — 型号识别现状与阻塞原因
- `crates/shengjian-contract/src/button_profile.rs` — 方案契约与合成
- `crates/shengjian-app/src/profiles.rs` — 方案选择与分层继承
- `crates/shengjian-windows-adapter/src/lib.rs` — 方案持久化
