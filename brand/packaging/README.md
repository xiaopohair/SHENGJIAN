# NSIS 安装器物料

Tauri NSIS 安装向导的三处品牌图形，对应 `tauri.conf.json → bundle.windows.nsis`：

| 配置项 | 尺寸 | 文件 | 状态 |
| --- | --- | --- | --- |
| `installerIcon` / `uninstallerIcon` | `.ico`（16–256） | 复用 `icons/icon.ico` | 已接线 |
| `headerImage` / `uninstallerHeaderImage` | 150 × 57，BMP | `installer-header.bmp` | 已接线 |
| `sidebarImage` | 164 × 314，BMP | `installer-sidebar.bmp` | 已接线 |

源文件与预览：

- `installer-header.png` / `installer-sidebar.png`：预览用。
- `.bmp`：24 位 BMP，NSIS 实际使用；Tauri 从 `apps/shengjian-desktop/src-tauri/windows/` 读取副本。
- 生成脚本：`brand/tools/build_packaging.py`（从 `brand/png` 组合，不依赖字体文件；slogan 用系统雅黑渲染）。

## 设计约束（按 `BRAND.md`）

- 底色墨 `#0B0F14`；一张物料只放一个重点：header 是标记 + 字标，sidebar 是竖排锁定 + slogan。
- 只用键帽标记、字标、细松青分隔线；不用库存照片、3D 渲染、大面积渐变或夸张光效。
- 不给标志加投影/描边/发光；字标保持原比例。

## 重新生成

```powershell
python brand/tools/build_packaging.py
Copy-Item brand/packaging/installer-header.bmp apps/shengjian-desktop/src-tauri/windows/installer-header.bmp -Force
Copy-Item brand/packaging/installer-sidebar.bmp apps/shengjian-desktop/src-tauri/windows/installer-sidebar.bmp -Force
```
