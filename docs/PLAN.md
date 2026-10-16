# 工作工具箱 · Windows 版（Rust 重写）— 项目计划

> 创建：2026-10-15 · 更新：2026-10-16 凌晨（首个 exe 交付） · 工作目录：`/home/li/worktoolbox-win/`
> 目标：把安卓版「工作工具箱」(cn.personal.worktoolbox) 重写为 Windows 原生工具：**先复刻原有功能，再加「电线线缆选型计算器」（多标准）**。

## 0. 背景

- 原 App：安卓，Kotlin + Jetpack Compose + SQLite；v0.1.13 调试签名（个人开发）。功能：报警码大全（汇川/西门子/高创，13754 条）、电气元件 3D 图库、单位换算、通信参考、电磁阀动画等。
- 用户需求：Windows 电脑上使用（上班场景）；先复刻原功能，再加「电线线缆选型计算器」类功能。

## 1. 技术方案（已确认 ✅）

- **Rust + egui/eframe** —— **eframe/egui/egui_extras 0.36.2**（注意：0.36 默认渲染器是 **wgpu**，不是 glow）
- 目标平台：64 位 Win10/11；交付：exe（+ 后续 data 文件夹），zip 打包微信交付
- 构建方式：Linux 侧交叉编译（`x86_64-pc-windows-gnu` + mingw-w64 GCC 13）
- 实现要点：
  - 中文字体：运行时加载系统字体（Windows: msyh.ttc；本机测试: Noto CJK）——已实测
  - 报警库：rusqlite（bundled）读 sqlite（下阶段加入）
  - 单位换算 / 线缆表：自建数据（公开标准），线缆表按标准分库
- **磁盘策略**：编译产物统一放根分区 `app/.cargo/config.toml → target-dir = "/var/tmp/worktoolbox-target"`

## 2. 环境与进度（2026-10-16 凌晨）

- [x] rustup + rustc/cargo **1.99.0**（rsproxy.cn 镜像）
- [x] 交叉目标 `x86_64-pc-windows-gnu`
- [x] **界面骨架完成并编译通过**（首页卡片 + 五个模块占位页 + 深色主题 + 中文系统字体）
- [x] mingw-w64 交叉编译器 + xvfb + imagemagick 已装（2026-10-15 深夜，用户授权）
- [x] **首个 Windows exe 交叉编译成功并交付**：14.2MB（release + thin-lto + strip），PE32+ x86-64 GUI；
  打包 `deliver/工作工具箱-v0.1-测试版.zip`（6.0MB）已微信发出
- [x] 界面预览截图自测：`screenshots/ui_v0.1_window.png`（Xvfb + import，中文渲染正常——**已人工目检**）
- [ ] 等用户反馈：界面样式（配色/字号/布局）

## 3. eframe 0.36 API 笔记（踩坑记录，写新代码必看）

- 主入口：`impl eframe::App { fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame) }`（**没有 update()**）
- 面板统一为 `Panel`：`egui::Panel::top(id).frame(...).show(ui, |ui| ...)`（**TopBottomPanel/SidePanel 已删除**）；
  `CentralPanel::default().show(ui, ...)` 保留
- `Frame::new()`（=旧 none）；圆角 `CornerRadius::same(u8)`；`Margin` 字段为 i8
- 字体：`FontData::from_owned(Vec<u8>)`（带 `index` 字段支持 .ttc）；`FontDefinitions.font_data: BTreeMap<String, Arc<FontData>>`
- `run_native` 的 creator 闭包返回 `Result<Box<dyn App>, DynError>`
- **本机 GUI 测试脚本要点**：终端会话保留历史 export 的环境变量——启动前必须 `unset WAYLAND_DISPLAY` 再 `DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1`，否则应用连到用户桌面会话（已踩坑）；wgpu 在 Xvfb 下走 lavapipe 软件 Vulkan，能正常出画面
- egui 0.36.2（2026 年新版，网上老教程 API 大多过时）

## 4. 功能分期（已确认 ✅：先复刻，后新功能）

### S1 复刻（当前阶段）
1. [x] 框架/界面骨架
2. [ ] **报警码查询**（当前任务：13754 条；搜索 / 品牌·类别筛选 / 详情含原因·处理·来源）
3. [ ] 电气元件图库（30 张渲染图 + photos.json；WebP 已备好）
4. [ ] 单位换算
5. [ ] 通信参考（PLC 数据类型 / 协议速查）

### S2 线缆选型计算器（多标准）
- 载流量选型 / 电压降校验 / 空开·保护配合 / 电缆型号建议
- **标准体系多选**：国标 GB（GB/T 16895 系列 ≈ IEC 60364）/ 欧标 IEC·EN（CE）/ 美标 UL·NEC（310 系列等）
- 表数据做成可编辑数据文件，按标准分库

### S3 打磨
- 图标、打包体验；低优先级：电磁阀动画

## 5. 数据资产（已归档 ✅）

- 原版 APK：`ref/工作工具箱_v0.1.13_原版.apk`（SHA256 bb43cf…9a6d）
- 已提取（`ref/assets/`）：`builtin/alarms.sqlite`（48MB：alarms 13754 / details 14156 / provenance 14210 / catalogs 129 / sheets 160）、`builtin/sources/*.xlsx`、`electrical/webp/*.webp`（30 张，2.8MB）、`electrical/photos.json`、`reference/plc-data-types.*`、`plc-protocols.json`
- 报警库字段：alarms(id, catalog_id, code, code_key, name, title, search_text, detail_count)；details(alarm_id, cause, verification, action, document_ref, raw_model, raw_brand, raw_category, notes)

## 6. 待办 / 悬而未决

- [ ] **当前：报警码查询模块**（数据层 + 搜索 UI + 详情页；数据在 `ref/assets/builtin/alarms.sqlite`）
- [ ] 其余复刻（元件图库 → 单位换算 → 通信参考）→ S2 多标准线缆计算器
- [ ] 等用户反馈界面样式（配色/字号/布局）

## 7. 推进策略 v2（2026-10-16 用户指示 ⚑）

- **不交付中间版本**：全部功能做完 + 美化完成后，**一次性交付成品**（中途不发 v0.1/v0.2…）。
- 数据方案变更：**弃用 rusqlite** → 自研紧凑格式（`tools/convert_alarms.py` 从 sqlite 转换 → postcard 二进制，纯 Rust 读取；免 C 依赖、可瘦身）。
- **并行内容生产线（子代理）**：units（单位换算）/ cable（标准 JSON + 计算引擎）/ comms（通信参考）/ electrical（元件资料）——接口由 stub 锁定：
  - `src/data/units.rs` · `src/data/cable_engine.rs` + `data/standards/*.json` · `src/data/comms.rs` · `src/data/electrical.rs`
- 美化重点：图标字体嵌入、卡片悬停效果、标题粗体、窗口/exe 图标、细节间距打磨。
- 交付物：`工作工具箱.exe + data/`（zip），仅此一次。

## 8. v1.0 完成（2026-10-16 00:21）✅

- 全部功能落地：报警码查询 / 电气元件 / 单位换算 / 通信参考 / **线缆选型（GB·IEC 与 UL·NEC 两套表库）** / 关于页
- 数据管线：`tools/convert_alarms.py`（sqlite→postcard 17.5MB，**同时内嵌 exe 作兜底**）；标准 JSON 由内容生产线产出（`app/data/standards/`，编译期内嵌）
- 图标：窗口图标 + exe 文件图标（`assets/icon.webp` / `assets/app.ico` + build.rs + winresource → .rsrc 已确认）
- 美化：深色主题、卡片悬停、彩色边条、粗体标题、间距打磨
- 测试：`cargo test` 21 项全过（units 13 / cable 7 / alarm 真实数据 1）
- 交付：`deliver/工作工具箱-v1.0-Windows.zip`（12.1MB；exe sha256 05bd5fac…8399a）
- 构建/截图脚本：`tools/screenshot.sh`（Xvfb 渲染截图）、`tools/package.sh`（一键打包）
- 界面均已 Xvfb 渲染目检：首页 / 报警 / 元件列表+详情 / 单位换算 / 通信两页 / 线缆计算

## 9. v1.1 UI 重构（2026-10-16 12:14）✅

- 全新暗色设计系统：令牌化色板 / 三级文字层次 / 1px 半透明描边体系 / **真字重家族**（msyhbd·Noto CJK Bold）
- 自绘组件库 `src/ui.rs`：胶囊按钮 / 分段控件 / 状态胶囊 / 卡片 / 大数字 / 软分隔线（全部 0.12s 悬停动画）
- 全页面重构：顶栏（logo+面包屑）、首页（字形方块卡片）、报警 / 单位 / 元件 / 通信 / 线缆 / 关于
- 流程示范：`feat/ui-refactor` 分支 → PR #1 → 合并 main；`docs/screenshots/` 已更新
- 新技能 `egui-ui-polish`（供后续 UI 美化任务复用）

## 10. v1.2 UI v2 重制：引入最高星设计技能（2026-10-16）✅

- 安装 GitHub 最高星设计类技能 `ui-ux-pro-max`（13.6 万星：79 风格 / 192 配色 / 119 UX 规范 / 22 技术栈）
- 按其生成的设计系统重制：**Swiss Minimalism**——浅色 藏青 #1E3A5F + 祖母绿 #047857；深色 #0F172A + 亮绿
- **明暗双主题**，顶栏一键切换；全部颜色走运行时调色板（令牌化，无硬编码）
- 字体升级：IBM Plex Sans（拉丁）+ JetBrains Mono（数值/编码）+ 系统中文（微软雅黑/Noto）
- **Lucide 图标** 16 枚（编译期嵌入 PNG + 纹理缓存 + 任意着色）
- 对比度 4.5:1 校核 / 8dp 间距节奏 / 0.12s 悬停动画；`feat/ui-v2` 分支 → PR → main
