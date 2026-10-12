# ER Torrent Skin Slots

[简体中文](README.md) | [English](README_EN.md)

一个面向《ELDEN RING》1.17 的灵马外观扩展槽位 DLL。它绕过游戏原本只能保存
`0..3` 四个外观选择值的限制，并通过 INI 目录把额外的选择器映射到事件旗标和
`NpcParam` 外观行。

> 当前支持：WW `eldenring.exe` 产品版本 `2.7.0.0` / `2.7.1.0`（游戏版本 1.17 / 1.17.1）。
>
> 本项目是非官方 Mod 工具，与 FromSoftware、Bandai Namco 或 Mod Engine 2
> 无关联。请关闭 Easy Anti-Cheat 并仅在离线或明确允许 Mod 的环境中使用；不要
> 用它连接官方在线服务。

## 功能与边界

DLL 负责的事情：

- 从 `er_torrent_skin_slots.ini` 读取可扩展的灵马槽位目录；
- 根据每个槽位对应的事件旗标确定本地选择器；
- 在灵马第一次创建并解析外观时，把选择器映射为指定的 `NpcParam` ID；
- 按 INI 目录扩展原生坐骑角色识别与物理初始化识别，
  让新增坐骑参与原版骑乘动画、清理与坐骑物理分支；
- 在加载时核对 EXE 版本和关键代码签名；不匹配时不安装 Hook；
- 把加载、选择和外观解析结果写入同目录日志。

DLL **不会**替使用者完成以下内容：

- 修改 `m00_00_00_00.talkesdbnd.dcx` 或创建灵马选择菜单；
- 初始化、清除或保存新增的事件旗标；
- 合并 `regulation.bin`，或者创建 `NpcParam` / `RideParam` 行；
- 安装角色模型、纹理、动画、音频、FMG 文本等资源；
- 自动处理与其他 Mod 的 Param、Talk、Event 或文件覆盖冲突。

这样的职责划分使 DLL 只提供“扩建槽位”的底层能力，实际坐骑内容及菜单完全由
Mod 作者自行组织。

## 已验证状态

- v0.1.2 修复新增坐骑未进入原生坐骑识别及物理初始化分支的问题。
  项目所有者于 2026-10-13 确认：在本地 1.17.1 Mod、未启用 Seamless Co-op 的
  环境中，c8008 跨小台阶异常，以及下落时退回标题、读档后跳跃导致马消失和
  玩家卡在骑乘状态的问题，均已修复。该人工验收针对 `0.1.2-test.1` 的同一修补
  逻辑；v0.1.2 仅移除测试版版本后缀，并更新文档，不改变修补行为。
  自动测试使用 1.17.1 EXE 的实际机器码验证识别、动画选择、物理初始化和撤销
  Hook 的前后结果。1.17 旧 EXE 本轮未做同等回归验证，也不据此宣称联机已验证。
  INI 格式不变；识别来源为已配置的扩展槽位，而不是固定新增三个角色。
  `log_enabled = true` 时增加最多 64 条变化后的恢复边界状态记录；只读取原版
  骑乘状态，不改写坐标、HP、状态旗标或挂接指针。

- v0.1.1 增加 WW 1.17.1（EXE 2.7.1.0）适配：更新外观解析地址与选择刷新特征码，
  并固定到支持新版的 fsrs fork 提交，通过新 EXE 静态检查、安全门破坏测试和编译。
  保留 1.17 的独立版本配置；本次新增坐骑行为修补见 v0.1.2。

- 本地使用：已用默认目录中的三个新增槽位（selector `4`、`5`、`6`）完成游戏内
  人工测试。
- 官方槽位：selector `0..3` 继续沿用游戏原有 `NpcParam` 映射。
- Seamless Co-op 自定义同步：仍是实验功能。`probe` 模式只发送、接收和记录目录
  状态，不会把远端身份路由到外观创建流程；`on` 在 `0.1.0` 中保留但拒绝启用。

若只需要本地玩家使用扩展坐骑，请保持 `network_mode = off`。

## 默认槽位目录

仓库附带的 INI 包含四个官方槽位和三个经过本地测试的扩展槽位：

| selector | 事件旗标 | appearance `NpcParam` | 说明 |
|---:|---:|---:|---|
| 0 | 6700 | 80020000 | 官方槽位 |
| 1 | 6701 | 80030000 | 官方槽位 |
| 2 | 6702 | 80040000 | 官方槽位 |
| 3 | 6703 | 80050000 | 官方槽位 |
| 4 | 6704 | 80060000 | 扩展槽位示例 |
| 5 | 6705 | 80070000 | 扩展槽位示例 |
| 6 | 6706 | 80080000 | 扩展槽位示例 |

这里的第三列必须填写完整的 `NpcParam` 行 ID。DLL 只返回这个 ID；该行引用哪个
`RideParam`、最终加载哪个角色资源，取决于使用者自己的 `regulation.bin` 和资源
目录。

## 安装

### 1. 准备 DLL 和 INI

从源码构建后，取得：

```text
target/x86_64-pc-windows-msvc/release/er_torrent_skin_slots.dll
```

把它与仓库根目录下的 `er_torrent_skin_slots.ini` 放在同一目录：

```text
your-mod/
├─ er_torrent_skin_slots.dll
└─ er_torrent_skin_slots.ini
```

INI 是 DLL 的 sidecar 配置，启动时只读取一次。修改后必须完全退出并重新启动游戏。

### 2. 由 DLL Loader 加载

以 Mod Engine 2 为例，在对应配置的 `[modengine]` 中加入 DLL 路径。下面假设 DLL
位于启动目录：

```toml
[modengine]
external_dlls = [
    "er_torrent_skin_slots.dll"
]
```

如果 DLL 放在子目录，请按你的 Mod Engine 2 启动目录和配置方式填写正确的相对或
绝对路径。无论 Loader 从哪里加载，INI 都必须紧邻实际加载的 DLL 文件。

### 3. 安装你的内容 Mod

使用者仍需通过 Mod Engine 2 或自己的打包方案安装：

- 修改后的 Talk 菜单；
- Event 初始化与旗标逻辑；
- 包含新增 `NpcParam` / `RideParam` 的 `regulation.bin`；
- 新坐骑所需的模型、材质、纹理、动画及其他资源；
- 对应的菜单文本。

仓库不会分发任何从游戏中提取的文件。

## INI 配置参考

```ini
# ER Torrent Skin Slots - WW Elden Ring 1.17 / executable 2.7.0.0
enabled = true
log_enabled = true

network_mode = off
packet_type = 238
heartbeat_frames = 120

# selector, event flag, appearance NpcParam ID
slot = 0, 6700, 80020000
slot = 1, 6701, 80030000
slot = 2, 6702, 80040000
slot = 3, 6703, 80050000
slot = 4, 6704, 80060000
slot = 5, 6705, 80070000
slot = 6, 6706, 80080000
```

### 通用选项

| 键 | 可用值 | 作用 |
|---|---|---|
| `enabled` | `true` / `false` | 为 `false` 时不安装任何 Hook |
| `log_enabled` | `true` / `false` | 是否写入 `er_torrent_skin_slots.log` |
| `network_mode` | `off` / `probe` / `on` | 网络实验模式；正常使用请选择 `off` |
| `packet_type` | `0..255` | `probe` 模式的候选自定义包类型 |
| `heartbeat_frames` | 正整数 | `probe` 模式发送心跳的帧间隔；最小按 1 处理 |
| `slot` | 三个十进制整数 | `selector, event_flag, appearance_npc_param` |

布尔值也接受 `1/0`、`yes/no` 和 `on/off`。键名区分大小写；未知键、非法数字或
非法目录会使整份配置被拒绝，此时 DLL 不安装 Hook，并在日志中写明原因。

### `slot` 规则

每一行：

```ini
slot = <selector>, <event_flag>, <appearance_npc_param>
```

必须满足：

1. `selector` 是 `0..255` 的唯一整数；因此目录最多 256 项，其中最多 252 项是
   selector `4..255` 的扩展槽位。
2. 每个 `event_flag` 必须唯一。请选择不会与游戏或其他 Mod 冲突的旗标。
3. `appearance_npc_param` 必须是正整数，并且对应实际存在的 `NpcParam` 行。
4. selector `0..3` 必须全部存在，其外观 ID 必须分别保持为 `80020000`、
   `80030000`、`80040000`、`80050000`。建议也保留默认旗标 `6700..6703`。
5. INI 中只要出现任意一行 `slot`，这些行就会**完整替换**内建目录，并不是在
   默认目录后增量追加。因此新增一项时仍要保留所有既有行。
6. 如果多个槽位旗标同时为 ON，`selector` 数字最小的槽位获胜；正确的菜单逻辑
   应保证任何时刻只有一个选择旗标开启。

如果 INI 文件不存在，或者配置中完全没有 `slot` 行，DLL 会使用内建的七项默认
目录。

## 一直增加新坐骑

不需要重新编译 DLL。以下示例在默认七项之后加入第八项：

```ini
slot = 0, 6700, 80020000
slot = 1, 6701, 80030000
slot = 2, 6702, 80040000
slot = 3, 6703, 80050000
slot = 4, 6704, 80060000
slot = 5, 6705, 80070000
slot = 6, 6706, 80080000
slot = 7, 6707, 80090000
```

然后由内容 Mod 完成下面的接线：

1. 在 `regulation.bin` 中创建 `NpcParam` `80090000`，并正确配置它所需的
   `RideParam` 和其他引用。
2. 安装该坐骑实际使用的角色资源。
3. 在 Talk 菜单中增加一项。选中后先关闭目录内所有槽位旗标，再只打开 `6707`。
4. 在 Event 中处理新旗标的默认值、初始化和需要的持久化逻辑。
5. 增加相应的 FMG 文本，并检查 Talk、Event、Param 与其他 Mod 的合并冲突。
6. 重启游戏，查看日志确认出现 `selector=7` 与 `appearance=80090000`。

继续增加时重复同样的流程，并为每项分配新的 selector、唯一旗标及有效的
`NpcParam` ID。DLL 不要求新增槽位连续编号，但连续编号通常更容易维护。

## 与 Talk / Event 的约定

推荐让菜单的每个选项使用同一套简单逻辑：

```text
关闭当前目录列出的所有灵马选择旗标
    -> 只打开本选项对应的一个旗标
    -> 结束菜单或刷新状态
```

不要让新增项同时依赖另一套混合状态或残留旗标。DLL 在刷新本地选择时只扫描 INI
目录中的旗标；它不会理解 Talk 状态机的含义，也不会自动纠正多个已开启的旗标。
如果没有任何目录旗标开启，选择器回退为 `0`。

## 网络模式

| 模式 | 当前行为 |
|---|---|
| `off` | 默认且推荐；只启用本地扩展槽位逻辑 |
| `probe` | 在检测到支持的协作 Mod DLL 时尝试发送、接收并记录自定义状态包；不把远端状态用于外观创建 |
| `on` | `0.1.0` 中保留但拒绝启用；本地 Hook 仍可工作 |

`probe` 消息包含完整槽位目录的哈希。两端目录不一致时会拒绝状态，但这不代表远端
外观同步已经可用。需要多人使用时，所有参与者仍应安装同一版本的 DLL、INI、
Talk、Event、Param 和坐骑资源；当前版本不承诺扩展坐骑在远端客户端正确显示。

## 日志和故障排查

默认日志路径与 DLL 相同：

```text
er_torrent_skin_slots.log
```

正常启动时可关注这些信息：

```text
loaded enabled=true ... slots=...
safety gate accepted WW 2.7.0.0 ...
hooks installed ...
local selector changed ... selector=7 ...
appearance resolve ... effective_selector=7 appearance=80090000
```

常见问题：

- **完全没有日志**：确认 Loader 确实加载了 DLL、INI 与 DLL 同目录，并且
  `log_enabled = true`。
- **`configuration rejected`**：按日志中的行号检查未知键、重复 selector、重复
  旗标、缺少官方槽位或非法 `NpcParam` ID。
- **`safety gate rejected executable`**：当前 EXE 版本或关键签名不受支持。DLL 会
  fail closed，不会尝试在未知地址安装 Hook。
- **菜单能选但 selector 不变**：检查 Talk 是否设置了与 INI 完全一致的旗标，并
  确认其他槽位旗标已经关闭。
- **selector 正确但模型错误或缺失**：优先检查 `NpcParam`、`RideParam` 和资源
  文件；这些不由 DLL 生成或加载。
- **与其他 DLL 同时使用时异常**：可能有多个 Mod Hook 了同一游戏逻辑。逐个停用
  其他 DLL 定位冲突，并检查各 Mod 的已支持游戏版本。

对于 selector `4` 以上的槽位，DLL 会把游戏原有的两位选择值镜像为 `0`，同时在
外观解析处使用真实扩展 selector。依赖原版两位值的其他 Mod 可能只看到 `0`，这
属于兼容性边界。

## 从源码构建

需要：

- 64 位 Windows；
- 支持 Rust 2024 edition 的 stable Rust 工具链；
- Visual Studio 2022 Build Tools 的 MSVC C++ 编译工具和 Windows SDK；
- `x86_64-pc-windows-msvc` Rust target。

```powershell
rustup update stable
rustup target add x86_64-pc-windows-msvc
cargo test -p er-torrent-skin-slots --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Release DLL 位于：

```text
target/x86_64-pc-windows-msvc/release/er_torrent_skin_slots.dll
```

FromSoftware.rs bindings 不保存在本仓库中。Cargo 会从
[`KamiyamaShiki0704/fromsoftware-rs`](https://github.com/KamiyamaShiki0704/fromsoftware-rs) 获取它们，并固定到
包含 ELDEN RING 1.17 / 1.17.1 支持的提交
[`7d4fdd66eda1a35284c170feb4b07b058c005f8f`](https://github.com/KamiyamaShiki0704/fromsoftware-rs/commit/7d4fdd66eda1a35284c170feb4b07b058c005f8f)。
首次构建需要网络连接；之后 Cargo 可使用本机缓存。`Cargo.lock` 还会记录该提交的
精确来源，因而不会随上游 `main` 分支变化。

## 仓库结构

```text
├─ src/                         DLL、配置、Hook 与实验网络代码
├─ Cargo.toml
├─ Cargo.lock
├─ er_torrent_skin_slots.ini    可直接修改的默认目录
├─ README.md                     中文使用说明
├─ README_EN.md                  English documentation
├─ LICENSE                      本项目的 MIT License
└─ THIRD_PARTY_NOTICES.md       第三方组件说明
```

## License

本项目代码以 [MIT License](LICENSE) 发布。Cargo 获取的第三方依赖保留其原始
许可，详情见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
