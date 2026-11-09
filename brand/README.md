# 声键 · 品牌资产

> 声音，就是快捷键。 / Your voice is the shortcut.

完整规范见 [`BRAND.md`](BRAND.md)；视觉版见 [`brand-board.html`](brand-board.html)。

## 快速取用

| 需求 | 文件 |
| --- | --- |
| 深色背景上的主标志 | `logo/shengjian-lockup-horizontal-dark.svg`（378.45 × 152） |
| 浅色背景上的主标志 | `logo/shengjian-lockup-horizontal-light.svg` |
| 方形 / 竖排场景 | `logo/shengjian-lockup-stacked-dark.svg`（224.85 × 298） |
| 单色横排锁定（单色印刷 / 水印 / 浅底） | `logo/shengjian-lockup-horizontal-mono.svg`（`currentColor`） |
| 单色竖排锁定 | `logo/shengjian-lockup-stacked-mono.svg`（`currentColor`） |
| 只要键帽标记 | `logo/shengjian-mark.svg`（viewBox `12 14 40 33`） |
| 单色 / 水印 | `logo/shengjian-mark-mono.svg`（`currentColor`） |
| 浅色背景 / 浅色任务栏标记 | `logo/shengjian-mark-on-light.svg`（松青·深 `#0F7A63`） |
| 极端对比 / 单色标记（备用） | `logo/shengjian-mark-ink.svg`（墨色 `#0B0F14`） |
| Windows 应用图标 | `logo/shengjian-app-icon.svg` + `png/app-icon-*.png` + `app-icon.ico` |
| **深色任务栏托盘** | `logo/shengjian-tray-16-color.svg`（亮松青，16 / 32 PNG 已导出） |
| **浅色任务栏托盘** | `logo/shengjian-tray-16-ink.svg`（墨色，16 / 32 PNG 已导出） |
| 托盘图标（备用） | `logo/shengjian-tray-16.svg`（白色） |
| 字标（不含标记） | `logo/shengjian-wordmark-dark.svg` / `-light.svg` |
| 浅色背景对比示意 | `png/light-surface-contrast.png`（亮松青 1.86:1 vs 松青·深 4.8:1） |
| GitHub 社交预览 | `social/github-social-preview.png`（1280 × 640） |

## 颜色

```css
--ink:   #0B0F14;  /* 墨 · 背景 */
--slate: #151D27;  /* 石板 · 面板 */
--teal:  #5AC8AA;  /* 松青 · 深色背景主色 */
--pine-deep: #0F7A63; /* 松青·深 · 浅色背景主色 */
--mint:  #8BE0C7;  /* 薄荷 · 高光（深底） */
--deep:  #2E8F76;  /* 深潭 · 深底键沿 */
--snow:  #E9EFF7;  /* 雪 · 深底正文 */
--paper: #F2F5F4;  /* 纸 · 浅底 */
--mist:  #8B99AC;  /* 雾 · 辅助文本 */
```

## 使用要点

- 深底用 `*-dark`，浅底用 `*-light` / `mark-on-light`（浅底键帽已经是松青·深）；不要用滤镜改变标志颜色。
- 浅色背景的品牌绿只用松青·深 `#0F7A63`；亮松青 `#5AC8AA` 只用于深色背景或大面积图形。
- 主锁定最小宽度 160px；主标记最小 16px。
- 标志四周留出约 28 单位（“键”字高度 1/3）的安全边距。
- 声波是**透明挖空**，不是黑色填充；放在任何底色上都应显示底色。
- 单色锁定（`*-mono`）与单色标记只用于单色印刷、水印、单色底；不要用它替代彩色主标志做品牌主视觉。
- 托盘/任务栏跟随明暗：深色用 `tray-16-color` / `mark`，浅色用 `tray-16-deep` / `mark-on-light`；白色/墨色单色版仅作备用。

## 重建

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File brand/tools/build_wordmark.ps1
python brand/tools/build_assets.py
powershell -NoProfile -ExecutionPolicy Bypass -File brand/tools/render_png.ps1
python brand/tools/build_packaging.py
```

依赖：Windows + PowerShell + Python（Pillow）+ 系统 Edge；字标重建还需要已安装的 Maple Mono NF CN Bold（SIL OFL 1.1）。如果 `python` 不在 PATH，先设置 `$env:PYTHON` 指向解释器。

## 许可

- 代码：GPL-3.0-only
- 本手册与品牌说明文本：CC BY 4.0
- “声键”名称与标志：项目作者保留权利；允许在提及本项目时按规范使用
- 字标轮廓：Maple Mono NF CN Bold（SIL OFL 1.1），资产中不附带字体文件
