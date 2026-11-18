# Chromecast Remote 探测（2026-11-18）

状态：**research**。身份、GATT 服务、HID 集合已探明；**按键在 Windows 上不产生任何 HID 上报**（重新配对后复测仍然如此），语音链路未做握手验证。尚不具备接入条件。

设备：Google「Chromecast Remote」（Chromecast with Google TV 语音遥控器），
本机已配对，BLE 地址 `a7:60:**:**:**:**`（完整地址见本地采集日志）。

## 结论先行

- **身份唯一可靠字段是 PnP ID `0x2A50`**：vendor id source = USB IF（`0x02`）、
  **VID `0x18D1`（Google）/ PID `0x9450` / REV `0x011B`**。
  广播名 `Chromecast Remote` 也够唯一，但不要靠型号号——它读出来是 **`hid_mouse`**，不是产品型号。
- **GATT 侧是"活的"**：8 个服务可枚举、DIS 可读、电量 88%、ATVV 服务与小米三款**UUID 完全一致**。
- **HID 侧是"死的"**：四个采集窗口（含不建立 GATT 会话的对照、含重新配对之后）里，
  遥控器没有向 Windows 投递过**任何** HID 上报；按音量+ 时系统音量条不弹出（用户实机确认）。
  同一窗口内本机物理键盘/鼠标事件正常收到，采集管道本身已被证明是通的。
- **语音侧是"活的"（2026-11-18 20:50 实测）**：ATVV 握手成功，
  `GET_CAPS` 回 `version=0x0100 / codec=0x02(16 kHz) / frame=247`，
  `MIC_OPEN` 后设备立刻 `STREAM_START` 推流，60 秒收到 3732 帧 / 477,696 字节，
  解出 954,902 采样（59.68 s @ 16 kHz）；人声段 RMS −20 ~ −45 dBFS、峰值触顶、削波 24 采样，
  安静段落在 −58 dBFS 底噪。**麦克风真实收声，且不需要按任何键**。
- 因此 声键 对这台设备的定位应该是：**能当一支纯语音麦克风，不能当遥控器**。
  按键与宏这条线判死；语音这条线只差应用侧接入（扫描白名单 + 型号映射 + 负增益）。

## 身份（`examples/gatt_probe -- enum`）

| 字段 | 值 |
| --- | --- |
| 广播名 `0x2A00` | `Chromecast Remote` |
| Appearance `0x2A01` | `0x03C3`（Remote Control） |
| PnP ID `0x2A50` | `02 D1 18 50 94 1B 01` → source=USB，VID `0x18D1`，PID `0x9450`，REV `0x011B` |
| 制造商 `0x2A29` | `zhuhai_jieli`（珠海杰理） |
| 型号号 `0x2A24` | `hid_mouse` |
| 序列号 `0x2A25` | `25091QUH1000GOCR PVT/MP0.0.1` |
| 硬件版本 `0x2A27` | `CR PVT/MP0.0.1` |
| 固件版本 `0x2A26` | `0.0.1` |
| 软件版本 `0x2A28` | `0.0.1` |
| System ID `0x2A23` | 全 `00`（占位） |
| IEEE `0x2A2A` | 空 |
| PPCP `0x2A04` | `08 00 0A 00 64 00 58 02` → 间隔 10–12.5 ms、从机延迟 100、超时 6 s |
| 电量 `0x2A19` | `0x58` = 88%（read + notify） |

## GATT 服务（8 个）

| 服务 | 特征 | 说明 |
| --- | --- | --- |
| `0x1800` GAP | `0x2A00` / `0x2A01` / `0x2A04` | 名称、Appearance、连接参数 |
| `0x1801` GATT | `0x2A05` indicate | 订阅 CCCD 写入返回 `ACCESS_DENIED`（Windows 不让用户态订） |
| `0x180A` DIS | 9 个身份字段 | 见上表 |
| `0x180F` 电池 | `0x2A19` read+notify | 可用 |
| `0x1812` HID | 特征枚举全部 `Unreachable` | 被 Windows HID 驱动独占，拿不到报告描述符（与小米三款同） |
| `0xAE40` | `0xAE41` write-nr / `0xAE42` notify | 厂商通道；订阅成功，整个采集期无通知 |
| `AB5E0001` ATVV | `0002` write-nr+write / `0003` read+notify / `0004` read+notify | **与 `xiaomi-rc001/rc003/arn9` 完全一致** |
| `D343BFC0` | `BFC1`–`BFC4` write / `BFC5` notify | Google 私有；订阅成功，无通知 |

ATVV 的 `0003` / `0004` 在静态读取下都返回 0 字节——与小米款一致，属协议设计
（能力由 `GET_CAPS` 请求换，不是静态读）。**本轮没有向 `0002` 写任何字节**，
所以"语音可用"尚未验证，只能说"协议面具备"。

## HID 集合（`GetRawInputDeviceList` + `RIDI_DEVICEINFO`）

| 集合 | 类型 | Usage Page / Usage | Windows 绑定 |
| --- | --- | --- | --- |
| `Col01` | 键盘 | 1 / 6 | `kbdhid`（HID Keyboard Device） |
| `Col02` | Consumer | `0x0C` / 1 | HID-compliant consumer control device |
| `Col03` | 鼠标 | 1 / 2 | `mouhid`（HID-compliant mouse，对应加速度计） |
| `Col04` | 厂商 | **`0xFF01` / 1** | HID-compliant vendor-defined device |
| `Col05` | 厂商 | `0xFF80` / 0 | 同上；**usage=0，无法单独注册 Raw Input** |

`RID_DEVICE_INFO` 只对 `RIM_TYPEHID` 有效；键盘/鼠标集合读到的 page/usage 是联合体里的
其它成员，不能当厂商页用（本次 `Col01` 因此显示成 `0x0C/0`，是假值）。

## 实测：按键零上报

`examples/chromecast_probe` 同时开两条通道：GATT 通知（电池 / `0xAE42` / ATVV 控制 /
`0xD343BFC5`）+ Raw Input（键盘页、鼠标页、Consumer 页、厂商页 `0xFF01`，按设备路径
`VID_18D1&PID_9450` 过滤，剔除本机物理键鼠）。

| 轮次 | 窗口 | GATT 会话 | 遥控器事件 |
| --- | --- | --- | --- |
| 1 | 185 s | 是 | 0（本轮日志被本机键鼠污染，作废） |
| 2 | 600 s | 是 | **0** |
| 3 | 300 s | **否**（`--no-gatt`） | **0** |
| 4 | 600 s（**解除配对 → 返回+Home 重新配对之后**） | 否 | **0** |

- 第 3 轮是专门用来排除"探针占用 GATT 把 HID 链路挤掉"的对照：不连 GATT 时依旧零上报，
  所以**不是探针抢连接导致的**。
- 采集管道自证：同一窗口内本机物理键盘（`VID_24AE&PID_1464`）与另一台蓝牙键盘
  （`VID_023554&PID_fa07`）的事件都收到了；8 秒空跑即可收到 360 条物理事件。
- 用户实机对照：按音量+ **系统音量条不弹出**，即 Windows 侧根本没收到 Consumer 上报。

> 已知盲区：`Col05`（`0xFF80`/usage 0）注册不上，"全部 HID"注册（`usUsagePage=usUsage=0`）
> 配 `RIDEV_INPUTSINK` 会直接失败，所以厂商 `0xFF80` 页的原始报告**无法被旁听**。
> 但既然连 Consumer 页和鼠标页都一条没有，结论不会因此改变。

### 重配对复测（2026-11-18 20:46）

按建议做了「删除设备 → 遥控器长按返回+Home 进配对 → 重新配对」，配对后重跑 `--no-gatt` 采集
600 秒，结果**仍是 0 条 HID 上报**。重配对确实生效（HID 实例号从 `9&3374d705` 变成
`9&1695a3ab`，5 个集合全部重新枚举），但行为没有变化。

设备栈本身是健康的（`Get-PnpDevice`）：

| 节点 | 状态 |
| --- | --- |
| `BTHLEDEVICE\{00001812…}`（HID over GATT 设备） | OK / `CM_PROB_NONE` |
| `HID\…&COL01`（HID Keyboard Device） | OK / `CM_PROB_NONE` |
| `HID\…&COL02`（HID 用户控制设备） | OK / `CM_PROB_NONE` |
| `HID\…&COL03`（HID-compliant mouse） | OK / `CM_PROB_NONE` |
| `HID\…&COL04` / `COL05`（厂商定义） | OK / `CM_PROB_NONE` |

即 Windows 侧驱动全部正常绑定，**不是驱动缺失或黄色感叹号**。

剩余假设（按可能性排序）：

1. **PVT 原型固件**：DIS 里硬件版本是 `CR PVT/MP0.0.1`、固件/软件都是 `0.0.1`，
   PVT/MP 是量产前阶段；工程样机把 HID 输入关掉或换成私有上报是完全可能的。
2. **HID 主机仍绑在 Chromecast**：遥控器可能只把按键发给电视，PC 侧只保留 GATT 会话
   （能读 DIS、能订阅，但收不到输入）。
3. **IR 分流**：遥控器规格含 IR 发射器，音量/静音/电源有可能是纯 IR 直打电视，
   这也是按音量+ 电脑毫无反应的一个解释——但它解释不了返回/主页/方向键同样为零上报。

区分方法：把这支遥控器拿到 Chromecast/电视前按几下。电视有反应 = 遥控器本身没坏，
问题在"给谁发"；电视也没反应 = 遥控器或固件本身有问题。
## ATVV 语音链路实测（2026-11-18 20:50）

探针：`examples/atvv_handshake_probe`（**只写** ATVV 的 `AB5E0002`，只用
`shengjian-domain` 的 `AtvvCommand` 编码，厂商字符一律不碰）。

| 步骤 | 写入 | 设备回应 |
| --- | --- | --- |
| 1 | `GET_CAPS` = `0A 01 00 00 03 03` | `0B 01 00 02 03 00 F7 01 00` → version `0x0100`、codecs `0x02`、interaction `0x03`、**frame 247 字节**、codec `0x02`、**16000 Hz** |
| 2 | `MIC_OPEN` = `0C 00` | `STREAM_START` = `04 00 02 00`，**立即开始推流** |
| 3 | `MIC_CLOSE` = `0D 00` | `STREAM_STOP` = `00 00` |

两次采集（8 s / 60 s）：

| 指标 | 8 秒 | 60 秒 |
| --- | --- | --- |
| AUDIO 通知 | 482 帧 / 61,696 字节 | 3,732 帧 / 477,696 字节 |
| 解码采样 | 123,006（7.69 s） | 954,902（59.68 s） |
| 峰值 / 有效值 | −27.6 / −40.2 dBFS | **0.0** / −32.1 dBFS |
| 削波 | 0 | 24 采样 |

60 秒那轮的分段电平（每秒 RMS）直接印证内容真实性：0–3 s 是拿起遥控器的摩擦声，
4–37 s 稳定在 **−58 dBFS 底噪**，**38–55 s 跳到 −20 ~ −45 dBFS**（用户大声念数字），
56–57 s 回到 −58，58–59 s 又有一段。底噪与语音差 30 dB 以上，不可能是编解码噪声。

### 三个要记住的结论

1. **不需要按键**：主机写 `MIC_OPEN` 设备就推流，没有 PTU/按键门禁。
   对声键是好事（触发可以改成键盘热键），但也是隐私上必须知道的一点——
   已配对的主机随时可以开这支麦克风。
2. **帧长 247 字节，不是小米款的 120**。`AtvvVoicePipeline` 用的是
   `capabilities.frame_size`，所以能自适应；但任何写死 120 的代码都会在这里错位。
3. **麦克风很热**：正常距离说话就顶到 0 dBFS（削波 24 采样），比 RC003 还高。
   声键的增益范围是 `0..24 dB`（只能放大、不能衰减），默认 0 dB 就会削波——
   要么让用户离远点说话，要么补一条负增益能力。
## HID 直读实测：用户态一条都读不到（2026-11-18 21:02）

探针：`examples/hid_read_probe`（`CreateFile(GENERIC_READ)` + 阻塞 `ReadFile`，
不注入、不提权）。

| 集合 | 打开结果 |
| --- | --- |
| `Col02` Consumer | OK |
| `Col04` 厂商 `0xFF01` | OK |
| `Col05` 厂商 `0xFF80` | OK |
| `Col01` 键盘 | `0x80070005`（`kbdhid` 独占） |
| `Col03` 鼠标 | `0x80070005`（`mouhid` 独占） |

句柄能开、`ReadFile` 不报错，但 **150 秒内（用户按了音量± / 返回 / 主页 / 确定）一条报告都没有**。
结合 Raw Input 同样为零，可以确定：

> **按键报告在 HOGP 的 UMDF 驱动（`WUDFHost.exe`）里就被消费掉了，从来没进入 `hidclass`
> 的用户态管道**。所以任何用户态读法（Raw Input、HID `ReadFile`、键鼠钩子、Consumer 页）
> 都拿不到，原生动作自然也不会发生。

这也解释了为什么 MoteDeck 和 vibe-mote 都选择注入 `WUDFHost.exe`：
MoteDeck hook 驱动内部的 `HogpGattAdapter::OnReportValueChanged`，
vibe-mote hook `ntdll!NtDeviceIoControlFile` 看 IOCTL `0x80018483` 的输出缓冲。
两条路都在 `hidclass` **之下**，所以能看到我们看不到的东西。

代价也很明确：注入系统服务进程需要**管理员**，而且容易被杀软拦。
声键当前是普通用户权限运行的产品，这一步是策略决定，不只是技术决定。
## 按键实测：3 字节 Consumer 报告（2026-11-18 21:16，已确认）

用户态直读不通，但**在 `WUDFHost.exe` 里挂钩子就能拿到**。本次用 Frida **只读**嗅探
（`ntdll!NtDeviceIoControlFile` + IOCTL `0x80018483`，只复制输出缓冲、不改写），
120 秒采集到 28 次命中、28 条报告。

**报告格式固定 3 字节**：`[0x02][usage_lo][usage_hi]`
——`0x02` 是 report ID（GATT 层不带，驱动补上），后两字节是 **16 位 Consumer Control usage**，
小端。松开时 usage 为 `0x0000`。

| usage | 键 | usage | 键 |
| --- | --- | --- | --- |
| `0x0041` | 确定 | `0x00E2` | 静音 |
| `0x0042` | 上 | `0x00E9` | 音量+ |
| `0x0043` | 下 | `0x00EA` | 音量− |
| `0x0044` | 左 | `0x019E` | 电源 |
| `0x0045` | 右 | `0x0189` | 信源 |
| `0x0223` | 主页 | `0x0077` | YouTube |
| `0x0224` | 返回 | `0x0078` | Netflix |

（15 个键里 14 个都在这一次采集里按到了，表里是实测值；与 MoteDeck 的
`CHROMECAST_CONSUMER_USAGES` 完全一致。）

### 这台是「B 版」硬件

同类设备有两种报文：

| 版本 | GATT 载荷 | IOCTL 缓冲 | Windows 会不会原生响应 |
| --- | --- | --- | --- |
| A | 8 字节 `[code,0,0,0,0,0,0,0]` | 8 字节 | 不会 |
| **B（本机）** | **2 字节 usage** | **3 字节（带 report ID `0x02`）** | **也不会**（所以音量条不弹） |

这就是 `motedeck` 表里 `A`/`B` 两种码表的由来。**本机是 B 版**。

### 代价：必须往系统服务进程注入

| 项 | 现状 |
| --- | --- |
| 目标 | `WUDFHost.exe`（HOGP 驱动宿主，本次 pid 42684，可从注册表 `WUDFDiagnosticInfo\HostPid` 取） |
| 权限 | **管理员**（非提权时 `frida.attach` 直接 `PermissionDenied`，已实测） |
| 依赖 | Frida 17.x（Python 3.14 有轮子；3.12 走 uv 装的解释器也可） |
| 版本脆弱性 | **低**——钩的是 `ntdll` 导出，不像 MoteDeck 那样要按模块 hash/PDB 解 RVA |
| 改写系统行为 | **没有**。只复制缓冲区；vibe-mote 那段"把 usage 写 0 抑制原生动作"本机不需要（Windows 本来就不响应） |

对声键的含义：按键可以做，但只能走「提权 helper 进程 + IPC」这条路
——仓库里已有 `helper/rc003` 的先例，架构上不是新东西，是产品策略问题（签名、杀软、安装时提权体验）。
## 对声键的三道门

| 层 | 现状 | 位置 |
| --- | --- | --- |
| 扫描 | 只放行名称白名单，`Chromecast Remote` 被 `continue` 掉，列表里根本看不到 | `crates/shengjian-windows/src/lib.rs` `scan_paired_remotes` / `is_supported_remote_name` |
| 型号 | `remote_model_from_model_number("hid_mouse")` → `None`，落到 `RemoteModel::Unknown`，没有按键布局与设备档案 | 同文件 `remote_model_from_model_number` |
| 输入 | 按键：HID 零上报（判死）；语音：**ATVV 已握手验证可用** | `examples/atvv_handshake_probe` |

前两道门在语音路线下同样要开：不放行就扫描不到、型号拿不到、也就没有设备档案。

## 建议的下一步

1. ~~重新配对~~ **已做，无效**（见上节）。HID 路线在 Windows 上可以判为不可用，
   除非能证明是"发给电视"而不是"设备不发"——把这支遥控器拿到 Chromecast/电视前
   按几下即可分流：电视有反应 ⇒ 遥控器健康，问题在 HID 主机归属；
   电视也没反应 ⇒ 遥控器/固件本身有问题。
2. **HID 修好后**再跑一次 `chromecast_probe`，用「每个键按一次 + 时间线」拿 usage 映射，
   这才是按键布局的证据来源。
3. ~~验证 ATVV 语音~~ **已验证可用**（见上节）。要把这台设备接进声键，剩下三件工程事：
   - **放行扫描与型号**：`is_supported_remote_name` 加 `Chromecast Remote`；
     `remote_model_from_model_number` 加 `hid_mouse`，并新增 `RemoteModel` 分支与
     `profile_buttons_for_model` 的空布局（它没有可用的按键）。**这一步必须在真机
     语音验收通过后再做**，而且要和"按键可用"的型号分开，避免用户以为能配宏。
   - **触发方式**：设备不给按键事件，语音只能由键盘热键或界面按钮触发。
     声键已有「自定义语音热键」，可直接复用；但要明确提示"这支遥控器的按键不可用"。
   - **负增益**：实测麦克风比 RC003 还热，0 dB 即削波。要么把增益下限放到 −12 dB，
     要么在 UI 上提示用户拉开距离。这是唯一一处需要改核心数值范围的地方。
4. 在 profile 由 `research` 转 `implemented` 前，**不要**把名字/型号加进运行时白名单。

## 接入方案：复用既有提权 helper（本仓库已有 80%）

盘点后发现，仓库里已经有一套**正好命中**的设施，不需要从零造：

| 组件 | 现状 | 对 Chromecast 的适用度 |
| --- | --- | --- |
| `helper/hid-host`（`shengjian-helper.exe`） | 提权、零第三方依赖、`CreateRemoteThread(LoadLibraryW)` 注入**定版定校验**的 Frida Gadget 到 `WUDFHost.exe` | 直接可用；目标发现、注入、租约、fail-open 都是现成的 |
| `helper/hid-host/agent/hid_agent.js` | 挂 `ntdll!NtDeviceIoControlFile` + IOCTL `0x80018483`，**同一个下刀点**；loopback TCP 回传，租约制清键 | 下刀点相同，但只认 `out[0]==0x01` 的键盘报告（RC003） |
| `crates/shengjian-windows/src/hid_bridge.rs` | 消费 helper 事件、目标集下发、ownership/代次/回退 | 协议可复用 |
| `crates/shengjian-windows/src/raw_input.rs` | `ENHANCED_CAPTURE_BUTTON_USAGES`（13 项 usage→按键白名单） | 需加一张 Consumer 页的表 |

所以工作不是"造一条路"，是**给现成的路加一个型号分支**。

### 五个切片

1. **agent 观测分支**（`hid_agent.js`）：接受 `out[0]==0x02`、长度 3 的 Consumer 报告，把
   `usage` 原样上行；**不清键**（本机 Windows 本来就不响应，清键反而危险）。
   需要遵守它已有的 `sendLine` 串行化与租约约定。
2. **helper 目标发现**（`src/main.rs`）：现在按 RC003 的 PnP 识别宿主；要能按
   `VID_18D1&PID_9450` 找到 Chromecast 的 `HostPid`。**注意**：一台机器上可能同时有
   RC003 和 Chromecast 两个 HOGP 宿主，必须做互斥/归属，否则会挂错进程。
3. **bridge 映射**（`hid_bridge.rs`）：把 Consumer usage 翻译成 `RemoteButton`。
4. **contract / UI**：见下面的岔路口。
5. **门禁**：`is_supported_remote_name` + `remote_model_from_model_number("hid_mouse")`，
   并且只对 Chromecast 走 helper 路线，Rc001/Rc003/Arn9 继续走原路径。

### 进展（2026-11-18 21:31 真机验证）

agent → helper 这一段已经在实机上跑通：

- helper 提权注入 `WUDFHost` pid 42684，agent 认到新代次 `build=2026-11-18.chromecast-consumer`；
- 90 秒内按 9 个键，助手日志逐条记下 `[CONSUMER] usage=0x00E9` … `usage=0x0000`：
  音量+ / 音量− / 返回 / 主页 / 确定 / 上 / 下 / 左 / 右，全部命中，且每按一次都有释放帧 `0x0000`；
- 同期 `target_hits=0`——RC003 的清键分支一次都没被触发，两条路互不干扰；
- `ioctl_calls` 从 2 涨到 12，与按键次数对得上。

宿主自动选择也已完成并验证（2026-11-18 21:34）：助手新增 `--hardware rc003|chromecast`
（默认 `rc003`，不传参行为不变），按 PnP 硬件串 `vid&0218d1_pid&9450` 过滤宿主成员；
不带 `--target-pid` 时自动定位到 pid 42684（`target_instances=3`、`how=attached_existing_tap`）。
识别只锁 VID/PID、不锁 REV——同 PID 不同批次（`rev&0110` / `rev&011b`）都该认。

剩下的是「`C` 行 → 映射引擎」需要主程序在环，以及**主程序把 hardware 传给助手**：
助手由计划任务拉起、参数在安装时就固定了，所以按设备区分的参数要写进主程序本来就在写的
`rc003-target.ini`（`version=1` / `address=` 旁边加一行 `hardware=`），而不是加 CLI 参数。
### 岔路口：三个键没有对应实体

`RemoteButton` 现在有 13 个变体，Chromecast 有 14 个键，多出来的是
**YouTube / Netflix / 信源**。三个选项：

| 方案 | 代价 | 后果 |
| --- | --- | --- |
| A. 新增 3 个 `RemoteButton` 变体 | 动 `ALL_BUTTONS`（门控位掩码、`ordinal`）、UI 画布、统计键、profile 按钮表、方案迁移 | 最干净，但跨 5 个模块，UI 要多画 3 个键 |
| B. 三个键暂时不接 | 零改动 | 少 3 个键，其余 11 个可用 |
| C. 硬塞进现有槽位（如 Netflix→TV） | 零改动 | 语义错乱，用户配动作时会困惑，不推荐 |

**决定点**：走 A 还是 B。A 是正经做法但要改 UI；B 能当天出可用版本。
## 端到端验收（2026-11-18 21:49，桌面应用在环）

```
13:49:16  [CONSUMER] usage=0x00E9   →  map_edges detail=VolumeUp=true   → button.edge {"button":"volume-up"}
13:49:16  [CONSUMER] usage=0x0000   →  map_edges detail=VolumeUp=false  → button.edge {"button":"volume-up"}
13:49:18  [CONSUMER] usage=0x0224   →  map_edges detail=Back=true       → button.edge {"button":"back"}
13:49:20  [CONSUMER] usage=0x0041   →  map_edges detail=Ok=true         → button.edge {"button":"ok"}
```

链路：遥控器 → HOGP 驱动 → 提权 helper 的 agent → 命名管道 `C <ms> <hex>` →
`hid_bridge::consumer_transition` → 映射引擎 → `button.edge` 核心事件。按下与松开成对到达。

接线过程中踩到并修掉的两个真实问题（都是"装了但没生效"类型，值得记）：

1. **计划任务跑的是已安装版的 helper**——`%LOCALAPPDATA%\声键\resources\hid-host\shengjian-helper.exe`，
   dev 构建改的是 `src-tauri/resources/` 与 `target/debug/resources/`，两处都要覆盖，
   否则日志里会出现"字段名对不上"的迹象（旧二进制仍打 `rc003_instances=`）。
2. **地址消歧只认 RC003 条目**——`rc003_instance_id` 只在 `is_rc003` 时填，
   Chromecast 条目为 `None`，导致四个候选宿主全部留下、助手报"请用 --target-pid 指定"。
   修法：两个硬件族都填实例 id，`host_pid_for_address` 也按"rc003 或 chromecast"筛。
## 语音端到端验收（2026-11-18 21:53）

用户按遥控器助理键，应用真的起了一次语音会话：

```
21:53:17  usage_stats action=voice_begin  source=stream_start
21:53:17  audio_stream phase=started endpoint_kind=virtual_cable
21:53:20  voice_level session=3 frames=54 samples=26676 peak=-6.7dBFS rms=-24.0dBFS clipped=0
21:53:20  voice_audio_rate result=degraded rate_hz=8987 samples=26676 duration_ms=2968 expected_hz=16000
```

两条关键结论：

1. **助理键不产生 HID 报告，但会直接发 ATVV `STREAM_START`**（`source=stream_start`）。
   所以语音不依赖按键链路，和小米 RC003 是同一条路。
2. **帧长是对的、速率偏低**：54 帧 × 494 采样 = 26,676，与 `frame_size=247` 完全吻合，
   说明应用确实用了设备声明的帧长；但 26,676 采样摊在 2.97 秒上只有 **8,987 Hz**，
   约为 16 kHz 的 56%。我们自己的探针在 59.68 秒里拿到的是**精确 16 kHz**
   （954,902 采样），所以不是设备本身的问题，更像应用侧这段时间的吞吐/丢帧。
   这一条值得单独查，但**不影响"这台设备可以用于语音输入"的结论**。
## 助理键走哪条通道（2026-11-18 21:56，只读探针确认）

探针：`examples/atvv_control_listen`（**零写入**：只订阅 ATVV CONTROL/AUDIO 的 CCCD，
不发 `GET_CAPS` / `MIC_OPEN` / `MIC_CLOSE`）。

```
t=28.975  [CONTROL] len=4  b=[04 03 02 04]   # STREAM_START
t=32.249  [CONTROL] len=2  b=[00 02]         # STREAM_STOP（松开）
结束：AUDIO 0 帧 / 0 字节
```

- 助理键**不产生 HID 报告**（所以 14 个消费页码里没有它，不是漏抓）；
- 按键时是**设备主动**发 `0x04 STREAM_START`、松开发 `0x00 STREAM_STOP`，
  不需要主机先发 `MIC_OPEN`——与 RC003 的「直接 0x04」同类；
- 但**零写入时没有音频**：设备只发控制帧、不推流。应用那次会话能拿到 54 帧，
  差别在于应用连接时会做 `GET_CAPS` 能力交换。
  ⇒ 待确认：前置条件是**能力交换**还是 `MIC_OPEN`（下一轮可用"只发 GET_CAPS"的探针判定）。

报文编码：`04 | 03 | 02 | 04` = STREAM_START / interaction `0x03` / codec `0x02` / session 4；
`00 | 02` = STREAM_STOP。
## 看门狗实测：没有触发，假设被推翻（2026-11-18 22:04）

补丁（`STREAM_START` 后 400 ms 无音频则补发 `MIC_OPEN`）上线后真机复测：

```
14:04:17  usage_stats action=voice_begin source=stream_start
14:04:20  voice_level session=6 frames=61 samples=30134 peak=-13.2dBFS rms=-27.0dBFS
14:04:20  voice_audio_rate result=degraded rate_hz=9479 expected_hz=16000
```

- **`atvv event=mic_open_retry` 没有出现**——音频在 400 ms 内就到了，看门狗没机会触发；
- 也就是说「必须先 `MIC_OPEN`」这个假设**对应用路径不成立**。当时只读探针拿不到音频，
  另有原因（探针只订阅 CCCD + 发 `GET_CAPS`，与应用的握手/顺序不同），尚未定位；
- **看门狗已撤除**（2026-11-18 22:19 用户决定）：它在本机和小米三款上都不触发，
  是一段永不执行的死代码，留着只会增加维护面。
- `atvv event=audio_progress` 诊断改为**按需开启**：默认不输出，排查采样率/丢帧时
  设 `SHENGJIAN_DEBUG=1`（`0`/`false`/`off` 视为关闭）再复现。

### 真正的问题：应用收到约 60% 的音频帧

同一台设备、同一段按键：

| 采集方 | 帧率 | 折算采样率 |
| --- | --- | --- |
| 裸探针（8 s） | 482 通知 / 8 s ≈ 60 帧/s | **16,000 Hz（精确实时）** |
| 桌面应用（3.18 s） | 61 帧 / 3.18 s ≈ 19.2 帧/s（≈38 通知/s） | **9,479 Hz** |

### `degraded` 是**测量口径**问题，不是丢帧（2026-11-18 22:13 定案）

加了 `audio_progress` 诊断（每 ~0.5 秒音频打一条「已收样本 / 已用墙钟」）后：

```
elapsed_ms  samples  cumulative_rate_hz
       266        0        0
       296      494    1,668
       806    8,398   10,419
     1,301   16,302   12,530
     1,796   24,206   13,477
     2,292   32,110   14,009
     2,786   40,014   14,362
```

**相邻两条之间的增量是精确的**：每 494–496 ms 涨 7,904 采样 →
**15,935 ~ 16,000 Hz**，也就是满速、**一帧没丢**。

累积率之所以从 0 爬到 14,362，是两个「非推流时间」被算进了分母：

1. **起步延迟**：`STREAM_START` 到第一帧约 270–300 ms；
2. **尾巴**：松手后设备停止推流，但会话还要等 `STREAM_STOP`/收尾（本次最后 1.43 s 只进了 6,916 采样）。

结论：**音频链路是健康的**，`voice_audio_rate result=degraded` 是拿「采样数 ÷ 会话墙钟」算出来的
系统性低估，短会话尤其明显。要修的是这个**度量口径**（按「首帧→末帧」算），不是音频路径。

### 已修并真机复验（2026-11-18 22:18）

`audio_span_ms(first, last, stream_ms)`：有首帧和末帧就按「末帧 − 首帧」算，
缺帧或末帧早于首帧（时钟异常）回落到会话墙钟，不产生 0 或负数分母。
顺带修了一个顺序问题：`StreamStopped` 分支原先**先清零 `last_voice_audio_at`、再算速率**，
导致永远回落到墙钟——现在先把它留在局部变量里再清零。

```
voice_audio_rate result=full rate_hz=16007 samples=42484 duration_ms=2654 stream_ms=4010 expected_hz=16000
```

`full` / 16,007 Hz（偏差 0.04%）/ 音频跨度 2,654 ms vs 会话墙钟 4,010 ms
——多出的 1,356 ms 正是「起步延迟 + 松手后的尾巴」，与推断一致。


## 最后一个断点：注入层拿不到映射（2026-11-18 23:0x，未完成）

排障到最后一环时的确切状态：

```
map_apply   model=Some("CHROMECAST") actions=10 ok=Shortcut{enter} volume_up=SystemAction   ← app 层算对了
map_resolve button=Ok table_action=Disabled                                                 ← 注入层表是空的
```

`shengjian-windows` 的注入运行时有一份**独立的映射表**（`send_input::ButtonMappings`），
由 `WindowsPlatform::set_button_mappings` 写入；全仓库除测试外**没有任何调用者**。

### 为什么"补个转换器推过去"是死路

查过两边的动作类型：契约层的 `LogicalAction` 有 `SystemAction { key }`，
而注入层的 `ButtonAction` **只有** `Disabled / Shortcut / OpenApp / FocusInput /
Scroll / MouseClick / MouseMove / RunMacro / OpenCommonPhrases`——**没有系统动作**。
把合成结果推过去，音量/静音会直接丢掉（我们 10 条里有 3 条是系统动作）。

### 正确的修法：改路由，不改转换

helper 旁路（`hid_bridge` 的 `C` 行 → `EngineMessage::GateEdge`）被送进了**旧注入器**，
而旧注入器是给 Raw Input + 增强捕获那条链路设计的（依赖 `key_gate` 武装、只认 Shortcut 类动作）。
Chromecast 的边沿应该像语音一样交给 app 层的 `ButtonService`，
由它按 `ActionSinkPort` 执行——那里支持全部动作类型。

改动点：`hid_bridge` 投递边沿时不要再发 `EngineMessage::GateEdge`，改为调用
`engine.buttons().press/release`；`key_gate` 相关的武装/吞键逻辑对这条路径不适用。
## 最终修复与端到端验收（2026-11-18 23:02）

最后一环是「合成好的映射没有同步到平台注入层」。三处改动：

1. `ActionSinkPort` 增加 `set_button_mappings`（默认空实现，纯逻辑/测试替身不受影响）；
2. 适配器新增 `contract_to_platform_mappings` 转换器：
   - `Shortcut` 复用现成的 `parse_chord`；
   - `SystemAction` 的音量三键映射成等价单键和弦（`KeyCode::VolumeUp/Down/Mute`）——
     注入层没有系统动作变体，但 `VK_VOLUME_*` 用快捷键同样能表达；
   - 其余动作一律 `Disabled`（宁可不动，也不误触发）；
3. `ButtonService::set_mappings` 里转发给注入层。

（另外修掉一个自己造成的坑：`Replace` 匹配到多处 `live()` 导致重复 `impl PlatformActions`，删掉 5 处。）

真机验收（`SHENGJIAN_DEBUG=1`）：

```
map_reconfig enabled=true buttons_configured=10
map_resolve  … table_action=Shortcut { chord: [VolumeUp] }
map_inject   result=queued last_vk=0x0D last_scan=0x1C echo_delta=2   # 回车
map_inject   result=queued last_vk=0xAF last_scan=0x00 echo_delta=2   # 音量+
```

`0x0D` = Enter、`0xAF` = VK_VOLUME_UP，`echo_delta` 是防双输入的回声抑制计数。
**按键从探测到真正注入，全链路闭环。**

## 打包坑（必读）

`pnpm build:installer` **只构建主应用、不构建 helper**，而安装包是从
`helper/hid-host/target/release/` 拷 helper 的。改了 helper 却只跑打包，
安装包里会**静默带上旧 helper**（本次实测：装上去的是 11-17 的版本）。

发布前必须：

```powershell
cd helper/hid-host && cargo build --release
cd ../../apps/shengjian-desktop && pnpm build:installer
```

或把这一步加进打包脚本 / 在 release 清单里校验 helper 的 SHA-256。
## 复现

```powershell
# 静态身份 + 服务枚举（只读）
cargo build --offline -p shengjian-windows --example gatt_probe
target\debug\examples\gatt_probe.exe enum --mac A7:60:10:14:3D:23 --out chromecast-enum.txt

# 合并探针：GATT 通知 + Raw Input（只读；只订阅，不写厂商特征）
cargo build --offline -p shengjian-windows --example chromecast_probe
target\debug\examples\chromecast_probe.exe --mac A7:60:10:14:3D:23 --seconds 300 --out probe.txt

# 对照：不建立 GATT 会话，只抓 Raw Input
target\debug\examples\chromecast_probe.exe --mac A7:60:10:14:3D:23 --seconds 300 --no-gatt --out probe-nogatt.txt
```

探针实现注意（踩过的坑，改之前先读）：

- `RAWINPUT` 要求 8 字节对齐，`[u8; N]` 栈缓冲在 debug 版会以
  `misaligned pointer dereference` panic，必须用 `[u64; N]` 当存储；
- `RegisterRawInputDevices` 是**整批原子**操作：混一个非法项（如 usage=0）整批失败；
  但分多次调用会互相顶掉（只剩最后一页生效），所以正确做法是过滤非法项后**一次批量注册**；
- Windows 绑定层不允许 panic 穿过 FFI 边界，`wnd_proc` 里要 `catch_unwind` 兜住。

## 参考

- `profiles/google-chromecast.json`（本机探测得到的设备事实，`research`）
- `crates/shengjian-windows/examples/chromecast_probe.rs`、`examples/gatt_probe.rs`
- `docs/device-identity.md`（小米三款的对照基线）

## 排障实录：遇到的问题与处理

按「现象 → 根因 → 处理」记录，供以后再接新设备时对照。

### 一、探测阶段

| 现象 | 根因 | 处理 |
| --- | --- | --- |
| Raw Input 收不到任何按键；用户态 `CreateFile`+`ReadFile` 打开 Col02/Col04/Col05 也读到 0 字节 | 该机的 HID 报告**只存在于 HOGP 驱动进程**（`WUDFHost`）内部，不进 `hidclass`，所以常规用户态路径全空 | 用只读 Frida 探针注入 `WUDFHost`，在 `ntdll!NtDeviceIoControlFile` 上挂钩 `IOCTL 0x80018483` 读走报告；据此确认报告格式 |
| 探针一跑就 `0xC0000409` | `RAWINPUT` 要求 8 字节对齐，`[u8; 512]` 强转导致 misaligned pointer dereference | 改用 `[u64; 128]` 存放 + `catch_unwind` 兜住窗口过程 |
| 探针收到大量按键，但不是遥控器的 | 物理键鼠也在同一个 Raw Input 队列里 | 用 `GetRawInputDeviceInfoW` 读设备路径并缓存，只保留 `VID_18D1&PID_9450` |
| 只注册 consumer 页有效，多页都注册后只有最后一页生效 | `RegisterRawInputDevices` 是**按调用整体生效**：一次调用里有非法项（usage 0）整批失败，多次调用互相覆盖 | 每个 page 单独判断 `usage != 0` 后**一次性批量**提交 |
| 注入 `WUDFHost` 时被拒 | 目标进程是 PPL/受保护，普通权限无法 `CreateRemoteThread` | 提权运行脚本；PowerShell 5.1 要求脚本存为 **UTF-8 with BOM**，否则中文报错静默失败 |

### 二、接入阶段

| 现象 | 根因 | 处理 |
| --- | --- | --- |
| 应用日志显示 `rc003_instances=`（旧字段名），新功能不生效 | 计划任务 `Shengjian HID Host` 执行的是 `%LOCALAPPDATA%\声键\resources\hid-host\shengjian-helper.exe`，是 11-17 的旧构建 | 覆盖安装目录 + `src-tauri/resources` + `target/debug/resources` 三处；**用日志字段名判断 helper 新旧** |
| helper `STOP`：定位到多个候选宿主 `{5216, 5696, 22404, 42684}` | `host_entry_matches_address` 只匹配 `rc003_instance_id` 非空的条目，而这个字段只对 RC003 填充 | 对 `is_rc003 || is_chromecast` 都填 `rc003_instance_id`，并放宽 `host_pid_for_address` |
| helper 挂到了错误的宿主（先挂 22404，1.5 分钟后才挂到 42684） | 应用连上后才把地址写进 `rc003-target.ini`，helper 在连接建立前用的是上一次的地址 | 属于启动时序，连接后自动纠正；排障时先确认 `[HELLO] pid=` 是当前设备宿主再按键 |

### 三、语音阶段

| 现象 | 根因 | 处理 |
| --- | --- | --- |
| 只读探针收不到音频，怀疑「必须先发 `MIC_OPEN`」 | 该假设对应用路径**不成立** | 后来实测：应用侧音频正常；保留的 400 ms 看门狗从不触发，已删除（死代码） |
| `voice_audio_rate result=degraded rate_hz≈9000` | 分母用的是**会话墙钟**，把「STREAM_START→首帧」的延迟和「松手→STREAM_STOP」的尾巴都算了进去 | 加 `audio_progress` 诊断证明瞬时速率稳定在 16 kHz；把分母改成**首帧→末帧** |
| 改完仍报 `duration_ms == stream_ms` | `StreamStopped` 分支**先把 `last_voice_audio_at` 清零再算速率**，永远回落墙钟 | 先把末帧时刻存进局部变量再清零 |

### 四、按键映射阶段（最费时的一段）

初始现象：按键边沿都到了（`button.edge` 正常），但没有任何输入产生。

排查方法是把链路切成三段，逐段取证：

| 段 | 证据 | 结论 |
| --- | --- | --- |
| 传输 | `[CONSUMER] usage=0x0041` | 通 |
| 映射 | `map_edges` / `button.edge` / `map_apply` | 通 |
| 执行 | `map_inject` / `inject_tap` | 断 |

逐层修掉的问题：

1. **预设的 `autoApply` 不按型号生效**——它匹配的是前台应用（`matches_executable`），
   `appTargets` 为空就永远不会自动套用。改为把默认映射写进**型号默认方案**
   （`ButtonProfile::default_for(model)` 的 `bindings`）。
2. **合成只遍历小米 12 键**——`compose_profile_mappings` 写死 `PROFILE_BUTTONS`，
   型号布局里的键（如 Chromecast 的静音）绑了也合成不出来。改为按
   `profile_buttons_for_model(profile.device_model)` 遍历。
3. **型号变化不触发重算**——`sync_foreground_if_changed` 的判重键只有「活动方案 id」，
   连上 Chromecast 时型号从 `None → CHROMECAST` 而 `active` 一直是 `None`，
   判重认为"没变化"，合成好的映射永远推不下去。判重键改为 `(型号, 活动方案 id)`。
4. **同一功能两条实现**——轮询路径内联 `set_mappings`，与 `apply_effective_mappings`
   分叉，行为不一致且不可观测。合并为一条。
5. **平台注入层有独立的一张表，没人写**——`WindowsPlatform::set_button_mappings`
   除测试外没有任何调用者。新增 `ActionSinkPort::set_button_mappings` +
   `contract_to_platform_mappings` 转换器（`Shortcut` 复用 `parse_chord`，
   `SystemAction` 的音量三键映射成等价单键和弦），由 app 层合成后统一推送。
6. **一次按键被执行两遍**——`engine.wire_button_source` 把边沿同时喂给 app 层
   `ButtonService`，平台注入器自己也会执行。静音是切换语义，做两遍正好抵消
   （"按一下静音，又立刻取消"），回车则会打两个。
   去掉 app 层那条 wiring，**注入器为唯一执行者**。

### 五、打包阶段

| 现象 | 根因 | 处理 |
| --- | --- | --- |
| 安装包里带的是 11-17 的旧 helper | `pnpm build:installer` **只构建主应用**，helper 从 `helper/hid-host/target/release/` 拷，那个产物没重建 | `build:installer` 前置 `cargo build --release --manifest-path ../../helper/hid-host/Cargo.toml`；发布前校验 helper 的 SHA-256 |

### 六、过程中的误判（留作教训）

- **把静音"开了又关"归因为 Windows 对 keyup 也切换**，于是把媒体键改成"只发 keydown"。
  真正的根因是**双重执行**；down-only 反而会触发系统自动重复。最终回退成标准 down+up，
  并去掉重复的执行路径。教训：先用日志数清"发生了几次"，再猜"每次做了什么"。
- **在源码里用脚本做多行替换**，多次命中重复位置：`Replace` 命中了 5 处 `live()`
  导致重复 `impl`；替换注释行时覆盖掉数组首项 `Back` 导致数组长度不符。
  教训：改代码用带唯一锚点的编辑，改完立刻编译验证。

### 七、可复用的排查手法

1. **三段取证**：传输（`[CONSUMER]`）→ 映射（`map_edges`/`map_resolve`）→ 执行（`map_inject`/`inject_tap`），
   每段都要有独立日志，否则无法定位。
2. **一按键一行日志**，并且能用 `SHENGJIAN_DEBUG=1` 开关控制；默认关闭，排查时打开。
3. **app 层也要能打日志**：为此加了 `shengjian_app::set_log_hook`，由宿主注册成 `gatt_note`。
4. **配置类问题优先用"改配置+重启"验证**，不必每次都重新打包安装。

## 界面：按键方案页的产品图与热区（2026-11-18）

`ProfileWorkbench.vue` 增加了 `CHROMECAST_HOTSPOTS`（14 个热区）和产品图
`google-chromecast.png`（335 x 1081，与小米三款同一画布）。

坐标是量的，不是目测的：

- **环形方向键**：线稿外圆 bbox `x=27..302 y=63..337`（圆心 49.0%/18.5%，外半径 137.5px），
  内圆（OK）bbox `x=113..215 y=148..250`（直径 103px）。四个方向取环带中径
  `(137.5+51.5)/2 = 94.5px` 处的四个正方向点，热区直径 80px（环带厚 86px，不顶到内外边界）
- **其余按键**：产品图暗连通域的 bbox 直接给圆心与直径——返回 100px、主页 100px、
  静音 97px、YouTube/Netflix 102px、电源 58px、信源 59px
- **底部胶囊**：中间那段是音量摇杆，左半 41.5% = 音量−、右半 55.2% = 音量+
- **助理键**（右侧黑色圆）是硬件语音键，不进方案，不标热区

同时补齐了前端的键定义：`LogicalButton` 增加 `volume-mute / youtube / netflix / input-source`，
`lib/buttons.ts` 新增 `CHROMECAST_BUTTONS`（14 项）与 `buttonLabel()` 反查，
`profileButtonsForModel("CHROMECAST")` 返回这套布局（方案页显示 14 键 × 3 手势 = 42 个触发位）。

配对指南（`DevicePairingHelp.vue`）新增 Google 机型：

- 配对手法：**同时按住「返回」和「主页」约 3 秒**（不是「主页 + 菜单」，那是小米的做法）；
- 蓝牙列表里的名字：`Chromecast Remote`；
- 产品图是**透明背景** PNG，深色舞台会直接透出来，因此只对 Google 机型铺浅色底
  （`.remote-stage.photo-light`）。

表格视图（`ProfileWorkbench.vue` 的 matrix 面板）补齐了四个新键的键帽字形：
静音 `⊘`、YouTube `▶`、Netflix `N`、信源 `⇥`；按钮行只保留键帽 + 名称 + 三个手势列。
真机验证：连接页与方案页都用 Chromecast 产品图；点击环带顶部选中「上」、
点击摇杆右半选中「音量+」，金色描边落在按键上。

## Review 记录（2026-11-19）

全量 review 的结论与当场修掉的问题。

### 验证矩阵

| 项 | 结果 |
| --- | --- |
| Rust 测试 | contract 29 + windows 570 + app 1 + adapter 27 = 627；helper 89；**0 failed** |
| clippy（workspace, all-targets） | **0 error**；190 warning 均为既有风格类（`too_many_arguments` / `while_let` / `is_multiple_of`），无新增 `dead_code` |
| 前端类型 | `vue-tsc --noEmit` 0 error |
| 前端测试 | 39 文件 / 242 测试全过 |
| 打包安装 | NSIS 0.9.4 构建并安装，运行日志 `ver=0.9.4` |
| 真机 | 连接 / 语音 / 14 键边沿 / 映射注入 / 静音 / 音量± / 回车 / 配对页 / 方案页 |

### P0：转换器覆盖不足（已修）

注入层是边沿的**唯一执行者**（app 层 `wire_button_source` 已停用），而 `to_platform_action`
第一版只映射 `Disabled / Shortcut / SystemAction`——注入层实际支持 8 种动作，
于是**滚动、鼠标移动/点击、运行宏、打开应用、常用语、聚焦输入框**六类动作会被静默丢成 `Disabled`。

- 修法：补全全部变体（`OpenApp / FocusInput / Scroll / MouseClick / MouseMove / RunMacro / OpenCommonPhrases`），
  `Mouse` 按 `kind + direction` 拆成滚动/移动/点击；
- 守卫测试：`every_action_kind_survives_conversion`（adapter），逐一断言每种动作转换后仍是原类型；
- 教训：**写转换器时用穷举 match，不要用 `_ => Disabled` 兜底**——兜底会把"没实现"伪装成"故意不动"。

### P2：其他当场修掉的问题

| 问题 | 处理 |
| --- | --- |
| 诊断开关叫 `audio_progress_enabled`，但映射诊断也在用 | 改名 `debug_log_enabled`（3 个文件 4 处） |
| `config/button-profiles.json` 里 `preset.chromecast-remote` 的 `autoApply: true` | 该字段按**前台应用**匹配，`appTargets` 为空永远不触发；改回 `false` |
| 版本号不一致：界面 0.9.4、日志 `ver=0.9.3` | 日志取自 desktop crate 的 `CARGO_PKG_VERSION`，漏改 `src-tauri/Cargo.toml`；已补 |

### 遗留（未处理，记录在案）

1. `wire_button_source` 现为注释保留的死代码，`WindowsButtonSource` 无使用者 —— 建议后续删除或在 trait 上做 feature 开关。
2. 诊断代码散在 `ble.rs` / `button_mapping.rs` / `send_input_windows.rs` 三处，建议收敛成一个 `debug` 模块。
3. `config/button-profiles.json` 的 Chromecast 推荐方案与代码里的型号默认方案内容重复（11 条手工同步），存在漂移风险；建议二选一。
4. 行尾不一致：部分 `.vue/.ts` 是 LF、`.rs` 是 CRLF，git 每次都会警告；建议加 `.gitattributes`。
5. 探针示例（`chromecast_probe` / `hid_read_probe` / `atvv_control_listen`）目前留在 `crates/shengjian-windows/examples/`，属于排障工具而非产品代码，建议移到 `Testing/`。
