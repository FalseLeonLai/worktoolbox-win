# 工作工具箱 · Windows 版

用 **Rust + [egui/eframe](https://github.com/emilk/egui)** 重写的「工作工具箱」Windows 桌面版：
**单文件、免安装、完全离线**，面向电气 / 工控现场的日常速查与选型计算。

![首页](docs/screenshots/home.png)
![报警码查询](docs/screenshots/alarm.png)
![电气元件图库](docs/screenshots/electrical.png)
![线缆选型计算](docs/screenshots/cable.png)

（其余界面截图见 `docs/screenshots/`）

## 功能一览

| 模块 | 说明 |
| --- | --- |
| 🔍 报警码查询 | 13754 条（汇川 / 西门子 / 高创 · 伺服 / 变频器）。关键词搜索 + 品牌·类别筛选；详情含**可能原因 / 确认方法 / 处理措施 / 适用型号 / 原始表格来源** |
| ⚡ 电气元件 | 30 个常用元件渲染图 + 用途原理 + 工程要点 |
| 📐 单位换算 | 22 个类别（含线规 AWG、压力、力矩、功率等），单位对调 + 全类别对照表 |
| 🔌 通信参考 | 26 个 PLC 数据类型 + 8 类通信协议要点（Modbus / EtherCAT / PROFINET / CANopen / OPC UA / S7 / RS-485） |
| 🧮 线缆选型计算 | 国标 GB / 欧标 IEC·EN（CE）/ 美标 UL·NEC：载流量选型、温度与并排校正、电压降校验、空开配合、电缆型号建议 |

## 构建方法

### ① Windows 本机（推荐）
1. 安装 [Rust](https://rustup.rs/)（stable 即可）
2. `cargo build --release`
3. 产物：`target\release\worktoolbox-win.exe`（单文件，数据已内嵌）

### ② Linux 交叉编译（本仓库的原始开发方式）
```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64            # Debian / Ubuntu
cargo build --release --target x86_64-pc-windows-gnu
```

### ③ Linux 本机运行（调界面用）
```bash
cargo run
```

> 说明：运行时自动加载系统 CJK 字体（Windows 用微软雅黑 msyh.ttc，Linux 用 Noto Sans CJK）。
> 仓库不含 `.cargo/config.toml`（开发机本地配置，用于自定义构建产物目录），需要时自行创建。

## 项目结构

```
├── src/
│   ├── main.rs            # 入口（窗口/图标/加载器）
│   ├── app.rs / theme.rs / fonts.rs
│   ├── screens/           # 页面：首页 / 报警 / 元件 / 单位 / 通信 / 线缆 / 关于
│   └── data/              # 数据层：报警库 / 单位表 / 线缆引擎 / 通信表 / 元件资料
├── assets/                # 应用图标 + 元件渲染图（编译期内嵌）
├── data/
│   ├── alarms.bin         # 报警库（postcard 紧凑格式，已提交，克隆即可构建）
│   └── standards/         # 线缆标准数据 JSON（可编辑，编译期内嵌）
├── docs/                  # 开发计划 + 界面截图
├── tools/                 # 数据生成 / 打包 / 截图 辅助脚本（开发机环境）
└── build.rs               # Windows exe 图标资源（winresource）
```

## 数据说明

- `data/alarms.bin`：由原数据的 SQLite 库经 `tools/convert_alarms.py` 转换生成的 postcard 二进制（17.5MB）。
  exe 内嵌一份作兜底；程序目录放置 `data/alarms.bin` 时优先使用外置文件（便于**不重编译**更新数据）。
- `data/standards/gb_iec.json` / `ul.json`：线缆载流量与校正系数节选表
  （IEC 60364-5-52 ≈ GB/T 16895.15、NEC 310.16 等，来源见 `data/standards/README.md`）。**改完 JSON 重新构建即生效**。
- 全部数据与计算结果供工程参考；正式设计、选型与施工请以现行标准原文及厂家资料为准。

## 开发建议

- `main` 分支与交付成品保持一致；新需求建议开 `feat/xxx` 分支开发。
- 界面相关改动可先用 `tools/screenshot.sh`（Xvfb 无头截图）自测，或直接 `cargo run` 看效果，再合并。
- 关键位置调试：环境变量 `WTB_START_SCREEN=home|alarm|electrical|units|comms|cable|about` 可直接启动到指定页面。
