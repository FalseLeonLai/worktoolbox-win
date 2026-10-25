# 任务：进制转换计算器 + 报警码序列搜索筛选

**状态**：完成
**创建时间**：2026-10-23
**最后更新**：2026-10-23

## 背景与目标

工作工具箱（`worktoolbox-win`，Rust + eframe/egui 0.36.2）新增两个功能：

1. **进制转换计算器**：f32/f64 IEEE754 浮点与十进制/十六进制/二进制互转；u16/i16/u32/i32 整数与二/八/十/十六进制互转；支持大端/小端及 Modbus 字交换。
2. **报警码查询 - 序列搜索**：序列下拉框改为可输入文本搜索筛选的选择器，匹配序列名称、型号、来源路径，不区分大小写。

已确认：单输入框 + 格式切换 + 只读结果；不显示 IEEE754 符号/指数/尾数分解；整数支持八进制；字节序/字序可切换；序列匹配"名称 + 型号 + 来源路径"。

## 探索结论

- 页面登记：`src/app.rs` 的 `Screen` 枚举、`MENU` 定长数组、`App` 字段、`App::new`、`WTB_START_SCREEN` 分支、主内容 `match`；模块在 `src/data/mod.rs`、`src/screens/mod.rs` 注册。
- UI：eframe/egui 0.36.2；纯逻辑放 `src/data/`，页面放 `src/screens/`；可复用 `src/ui.rs` 的 `card/section/page_head/segmented` 与 `src/theme.rs` 颜色 token；复制用 `ui.ctx().copy_text`。
- 无新增依赖；标准库足够。
- 报警序列：原 `src/screens/alarm.rs:166-189` 普通 ComboBox；129 个序列；选择写入 `AlarmScreen.series: Option<String>`；后续 `db.search` 精确匹配，链路未改。
- egui 无可搜索 ComboBox；采用自定义 `egui::Popup`（固定搜索行 + ScrollArea 列表，`CloseOnClickOutside`，选中后 `ui.close()`）。
- 无持久化；`WTB_SERIES` 调试预填保持。

## 关键入口文件（信息透明）

- `src/app.rs`（路由/菜单）
- `src/screens/mod.rs`、`src/data/mod.rs`
- `src/screens/about.rs`（功能清单文案）
- `README.md`（功能列表与 `WTB_START_SCREEN`）

## 任务拆分

- [x] T0：创建审计计划文件与特性分支 `feat/radix-series-search`
- [x] T1：数据层·进制转换（新增 `src/data/radix.rs`、`src/data/mod.rs`；16 个测试）
- [x] T2：数据层·报警系列匹配（`src/data/alarm.rs`；4 个测试，真实数据回归）
- [x] T3：页面·进制转换 + 接线（新增 `src/screens/radix.rs`；改 `src/screens/mod.rs`、`src/app.rs`）
- [x] T4：页面·报警序列搜索选择器（`src/screens/alarm.rs`）
- [x] T5：文档同步（`README.md`、`src/screens/about.rs`）
- [x] T5b：文档精度校正（浮点不支持八进制；README `data/` 架构行；页面副标题）
- [x] T6：验收测试（`cargo test --offline --locked` 114 passed；`cargo build` 通过）
- [x] T6b：启动冒烟（`WTB_START_SCREEN=radix` 与 `alarm` 均正常初始化 WGPU/DX12，无 panic）
- [x] T7：变更摘要（7 个跟踪文件修改、2 个新文件，无无关变更）

## 审批项（一次性确认）

- 删除文件：无
- 修改配置文件：无（`Cargo.toml` 不动，无新依赖）
- 新增依赖：无
- 项目结构调整：无（仅新增两个模块文件）
- 关键入口文件：`src/app.rs`（信息透明，非审批门槛）

## 风险与回滚

- Popup 首帧焦点：已实现"打开期间未获焦则补请求"；交互行为（焦点锁定、Esc、点外部、中文 IME）需真机确认。
- 借用冲突：字段级捕获，编译通过。
- 缓存顺序：先重建缓存再校验 `series`，已按此实现。
- 字节序：2 字节仅 Abcd/Dcba；8 字节 Cdab = GHEFCDAB；golden 测试锁定。
- 浮点格式化：最短往返 + 科学计数法阈值；测试覆盖 NaN/Inf/非规范数/-0。
- 报警链路：`last_key`/`db.search` 未改；真实数据回归通过。
- 已知 1 条 `series_list` 未使用警告（仅生产构建；测试仍使用），不影响构建。
- 回滚：特性分支 `feat/radix-series-search`；未提交、未推送；回滚用 `git restore`/切换回 `main`，不使用 `reset --hard`/`clean -fd`。

## 验证步骤

1. `cargo test --offline --locked` 全绿（114 passed / 0 failed）。
2. `cargo build --offline --locked` 成功（仅 1 条预期 dead_code 警告）。
3. 进制转换启动冒烟：`WTB_START_SCREEN=radix` 正常启动，WGPU/DX12 初始化成功，无 panic。
4. 报警页启动冒烟：`WTB_START_SCREEN=alarm WTB_SERIES=DDR系列 WTB_QUERY=234` 正常启动，无 panic。
5. 交互项（弹层聚焦、Esc/点外部关闭、中文 IME、复制按钮、视觉布局）待人工冒烟。

## 执行日志

- [2026-10-23] T0 完成 → 创建 `.claude/plan-radix-series-search.md` 与特性分支 `feat/radix-series-search`；未提交、未推送。
- [2026-10-23] T1 完成 → 新增 `src/data/radix.rs`（1123 行、16 个测试），`src/data/mod.rs` 注册；全仓 114 项测试通过，无新依赖。
- [2026-10-23] T2 完成 → `src/data/alarm.rs` 新增 `series_entries`/`series_matches` 与 4 个测试，`series_list` 委托新函数；真实数据回归通过。
- [2026-10-23] T4 完成 → `src/screens/alarm.rs` 自定义 Popup + 搜索框 + 缓存；构建与 114 项测试通过。
- [2026-10-23] T3 完成 → 新增 `src/screens/radix.rs`，`src/app.rs` 注册 `Screen::Radix`（MENU 8 项、`WTB_START_SCREEN=radix`）；构建与 114 项测试通过。
- [2026-10-23] T5 完成 → README 功能表与 `WTB_START_SCREEN`、about.rs 功能清单同步；构建通过。
- [2026-10-23] T6 完成 → `cargo test --offline --locked` 114 passed / 0 failed（两次串行复跑一致）；`cargo build` 成功；`Cargo.toml`/`Cargo.lock` 未变。
- [2026-10-23] T6b 完成 → 两个入口启动冒烟均正常，无 panic；诊断日志无崩溃条目。
- [2026-10-23] T7 完成 → 变更摘要确认：7 个跟踪文件修改、2 个新文件，无无关变更。
- [2026-10-23] T5b 完成 → 校正浮点/整数能力文案（浮点不支持八进制），README `data/` 架构行补齐；构建与 114 项测试通过。
