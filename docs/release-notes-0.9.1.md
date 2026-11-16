# 声键 0.9.1 发布说明

日期：待发布

> 本文件在 `v0.9.1` tag 推送时由 release workflow 逐字发布（`body_path`）。

## 本次变更

### 语音热键支持纯修饰键组合（`Ctrl+Alt` / `Ctrl+右Ctrl`）

Chatterfly 与豆包一样丢弃 `SendInput` 注入的按键（`LLKHF_INJECTED`），只能用报告层把
语音键槽内替换成**真实按键**来拉起语音；而 Chatterfly 只接受修饰键之间的组合，裸 Alt 不收。

现在报告层可以把纯修饰键组合写进 HID 报告的 u16 键槽（编码 `0x1E00|mask`）：

- `Ctrl+Alt`（左 Ctrl + 左 Alt）
- `Ctrl+右Ctrl`

真机实测（RC003 + Chatterfly）：两键同时到达（间隔 4ms），语音条正常拉起、可以出字。

配置建议：对注入键免疫的输入法配 **`Ctrl+Alt`** 或单键；`Ctrl+Shift` 会被 Windows 的输入法
切换占用，不建议使用。

### 报告层每帧最多两个键

三键组合（例如 `Ctrl+Shift+Alt`）在这台遥控器上**不可表达**：HID 报告的第三个键槽写入
不产生任何按键事件。两次对照实验（u16 步长写 3/5/7、连续字节写 3/4/5）的结果逐字节吻合
u16 槽位模型。

因此三键组合不再被当作可合成目标，会回落到注入路径；对 Chatterfly / 豆包这类工具，等于
「这一按不会有反应」。程序会在诊断日志里留一行判据：

```
voice_config warning=hotkey_not_synthable tool=chatterfly keys=3 ...
```

## 升级注意

- **无破坏性变更**：已有配置照旧读取，不需要迁移。
- 如果你之前把语音热键设成三键组合，升级后请改成两键（推荐 `Ctrl+Alt`）。

## 已知限制

- 报告层仍是「改写遥控器报文」的方案，表达能力受器件描述符限制：每帧两个键、白名单内的
  usage。需要任意组合键（`Alt+W`、`Ctrl+Shift+X` 等）或兼容更多输入法时，只能改用虚拟
  HID 键盘（WinUHid / UMDF-VHF）路线，但会引入驱动安装与签名分发成本。
