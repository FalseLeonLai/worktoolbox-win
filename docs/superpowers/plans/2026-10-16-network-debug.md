# 网络与串口调试实施计划

**目标：** 在现有 Rust/egui Windows 工具箱中交付网络、串口、Modbus TCP/RTU 和经典 S7 的独立调试页面，继续单文件发布。

**架构：** 页面持有表单和有界日志；后台线程独占连接，通过命令和事件传递数据。协议和数值编解码独立于 GUI，可用字节流和本机模拟端验证。

**技术栈：** Rust 2024、eframe/egui 0.36.2、std::net/std::sync、serialport 4.10.1。

## 1. 协议与编解码（独立并行）

- [x] `src/debug/codec.rs`：HEX / UTF-8、U8/I16/U16/I32/U32/F32、四种字节序。Bool 由线圈和 S7 位操作表单处理。已知字节序列和非法输入测试先红后绿。
- [x] `src/debug/modbus.rs`：纯帧构造/解析，不持有端口；功能码 01/02/03/04/05/06/0F/10，TCP MBAP 与 RTU CRC，严格匹配响应。
- [x] `src/debug/s7.rs`：纯 std 同步经典 S7 客户端；COTP/Setup、PDU reference、DB/M/I/Q 字节和位读写；使用回环模拟端验证分帧和错误。
- [x] 各独立模块先用 `rustc --test` 验证红/绿循环，再接入 Cargo 测试。

## 2. 后台连接

- [x] `src/debug/mod.rs` 注册模块；共享模式、连接参数、操作请求、事件与连接状态集中在 `worker.rs`。
- [x] `src/debug/worker.rs`：TCP 客户端/单客户端服务端、UDP、串口、工业协议请求；限时读写、停止信号、周期发送/读、线程回收。
- [x] 有界命令、事件与数据缓存，UI 非阻塞轮询。日志饱和时淘汰最旧记录并计数；状态或结果无法投递时停止通信，UI 以线程结束状态恢复。服务端操作绑定客户端代次。
- [x] 回环测试验证原始二进制收发、断开、失败和周期停止；只连接测试创建的本机端口。

## 3. 页面接入

- [x] `src/screens/debug.rs` 提供 `DebugScreen` / `show`，连接设置按模式显示，三类操作面板为原始收发、Modbus、S7。
- [x] 在 `src/app.rs` 注册导航、状态和启动入口 `WTB_START_SCREEN=debug`；每帧先处理调试事件，再渲染当前页面。
- [x] 工业写入仅由明确的「写入」按钮触发，显示实际地址与值；不自动重试。
- [x] 日志上限同时按条数和字节限制；暂停只影响显示，支持复制与清空。

## 4. 集成与交付

- [x] `cargo test --offline --locked`：94 项通过；最后界面默认值/说明调整后再次运行页面 9 项测试通过。新模块格式检查通过；静态 CRT 的 Windows release 构建通过。
- [x] 实际窗口验证深浅主题、900×600 布局、已有页面导航和 DX12 默认设置；最终发布版 TCP 二进制回显、切页保持连接、断开释放已验证。
- [x] 独立检查协议匹配、资源释放、队列容量和错误路径；已修服务端写失败、串口零进展、RTU 静默残留、跨客户端队列与日志保留问题，并补充回归测试。
- [x] README/CHANGELOG 与使用说明已更新，注明真实 COM/PLC 未测；用户的 Demo、分析包或授权资料未进入仓库和交付 EXE。

详细验收与静态检查限制见 `docs/network-debug-validation.md`。

## 分析证据

- 本机 HslCommunication.dll：13.0.2.0，5,087,744 字节，SHA-256 `5a6410714c078bcacd3a85276aafdb4bf609f8650e2e4cbbdcd2757bb59c2279`。
- 用户分析包中的 DLL：12.9.0.0，SHA-256 `7dd28a12cdd1101aa5ca6df5db2a45db8c42d610918840cf71ddf3edd94d701a`，不得把旧版数量当作当前事实。
- 当前元数据现场确认：.NET Framework 4.5.1、3,074 个类型、20,890 个方法；新旧通信基类并存。
- REA 6.4.0 的 managed metadata 解析达到 64 MiB 表示预算，返回部分结果；以本机 System.Reflection.Metadata 只读解析及 XML 交叉核对接口。
- 文件传输获用户明确授权，仅已复制核心 DLL 到指定 REA 临时目录，未执行目标代码。
- 本次交付只采用纯 Rust 方案，上述商业 DLL 不进入生产依赖或 Git。
