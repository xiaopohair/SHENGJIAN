# RC003 enhanced-capture helper

RC003 增强捕获轨（Return/音量键完整捕获）的独立提权 helper，迁移自原始 Windows 实现。

## 构建

```powershell
cd helper/rc003
cargo build
```

产物：`target/debug/shengjian-helper.exe`。

## 运行边界

- 独立 Cargo workspace，不参与顶层 `cargo test --workspace`。
- 只负责需要管理员权限的增强捕获；基础语音/按键路径不得依赖它。
- 安装/卸载/计划任务脚本仍沿用原始 helper 的脚本边界，只是二进制名和内部标识改为 `shengjian-*`。
- E1–E6 真机验收尚未全部完成，保持 spike 状态。
