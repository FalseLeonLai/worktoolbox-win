//! 网络、串口及工业协议调试页；连接与收发由 Worker 后台执行。

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::debug::codec::{self, ByteOrder, ValueKind};
use crate::debug::worker::{Config, Event, Log, Mode, Operation, Reply, State, Worker};
use crate::debug::{modbus, s7};

const MAX_LOGS: usize = 1000;
const MAX_LOG_BYTES: usize = 2 * 1024 * 1024;
const PREVIEW_BYTES: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ending {
    None,
    Cr,
    Lf,
    CrLf,
}

struct Record {
    elapsed: Duration,
    log: Log,
}

pub struct DebugScreen {
    config: Config,
    worker: Option<Worker>,
    state: State,
    status: String,
    error: Option<String>,
    started: Instant,
    tx_bytes: u64,
    rx_bytes: u64,
    worker_dropped: u64,
    repeating: bool,
    period_ms: u64,
    raw_kind: ValueKind,
    ending: Ending,
    input: String,
    kind: ValueKind,
    order: ByteOrder,
    unit: u8,
    mb_area: modbus::Area,
    address: u16,
    quantity: u16,
    s7_area: s7::Area,
    db: u16,
    offset: u32,
    length: u16,
    bit_mode: bool,
    bit: u8,
    bit_value: bool,
    result: Option<Reply>,
    logs: VecDeque<Record>,
    log_bytes: usize,
    discarded: u64,
    paused: bool,
    log_text: bool,
    ports: Vec<String>,
    port_scan: Option<mpsc::Receiver<Result<Vec<String>, String>>>,
}

impl DebugScreen {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            worker: None,
            state: State::Closed,
            status: "未连接".into(),
            error: None,
            started: Instant::now(),
            tx_bytes: 0,
            rx_bytes: 0,
            worker_dropped: 0,
            repeating: false,
            period_ms: 1000,
            raw_kind: ValueKind::Hex,
            ending: Ending::None,
            input: String::new(),
            kind: ValueKind::U16,
            order: ByteOrder::Abcd,
            unit: 1,
            mb_area: modbus::Area::HoldingRegisters,
            address: 0,
            quantity: 1,
            s7_area: s7::Area::DataBlock,
            db: 1,
            offset: 0,
            length: 2,
            bit_mode: false,
            bit: 0,
            bit_value: false,
            result: None,
            logs: VecDeque::new(),
            log_bytes: 0,
            discarded: 0,
            paused: false,
            log_text: false,
            ports: Vec::new(),
            port_scan: None,
        }
    }

    /// App 在页面路由前调用，隐藏此页时也持续消费有界队列。
    pub fn poll(&mut self, ctx: &egui::Context) {
        if let Some(worker) = &self.worker {
            // 先检测结束再取队列，结束的线程不会在取完后补投事件。
            let finished = worker.is_finished();
            let events = worker.events.try_iter().take(64).collect::<Vec<_>>();
            let logs = worker.take_logs();
            self.tx_bytes = worker.tx_bytes.load(Ordering::Relaxed);
            self.rx_bytes = worker.rx_bytes.load(Ordering::Relaxed);
            self.worker_dropped = worker.dropped_logs.load(Ordering::Relaxed);
            for event in events {
                match event {
                    Event::Peer(peer) => worker.acknowledge_peer(peer),
                    Event::State(state, detail) => {
                        if self.state != State::Closing || state == State::Closed {
                            self.state = state;
                            self.status = detail;
                        }
                    }
                    Event::Result(reply) => self.result = Some(reply),
                    Event::Error(error) => self.error = Some(error),
                    Event::RepeatStopped => self.repeating = false,
                }
            }
            for log in logs {
                if self.config.mode.is_raw() && !log.sent {
                    self.result = Some(Reply::Bytes(log.bytes.clone()));
                }
                self.push_log(log);
            }
            if finished {
                if self.state != State::Closed {
                    self.status = "后台已停止".into();
                    self.error =
                        Some("后台线程已结束，结束事件未送达；结果队列可能已满，请重新连接".into());
                }
                self.state = State::Closed;
                self.repeating = false;
                // 仅在线程确认结束后 drop，Worker::drop 的 join 不会阻塞 GUI。
                self.worker.take();
            }
        }
        if let Some(scan) = &self.port_scan {
            match scan.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(ports) => {
                            self.ports = ports;
                            if self.worker.is_none()
                                && self.config.serial_port.is_empty()
                                && let Some(port) = self.ports.first()
                            {
                                self.config.serial_port = port.clone();
                            }
                        }
                        Err(error) => self.error = Some(error),
                    }
                    self.port_scan = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = Some("串口枚举线程未返回结果".into());
                    self.port_scan = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.worker.is_some() || self.port_scan.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("debug_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::ui::card(ui, |ui| self.connection_ui(ui));
                ui.add_space(10.0);
                crate::ui::card(ui, |ui| self.operation_ui(ui));
                ui.add_space(10.0);
                crate::ui::card(ui, |ui| self.records_ui(ui));
            });
    }

    fn connection_ui(&mut self, ui: &mut egui::Ui) {
        crate::ui::section(ui, "连接设置");
        ui.add_enabled_ui(self.worker.is_none(), |ui| {
            let previous = self.config.mode;
            ui.horizontal_wrapped(|ui| {
                ui.label("模式");
                egui::ComboBox::from_id_salt("debug_mode")
                    .selected_text(self.config.mode.label())
                    .show_ui(ui, |ui| {
                        for mode in Mode::ALL {
                            ui.selectable_value(&mut self.config.mode, mode, mode.label());
                        }
                    });
                ui.label("超时");
                ui.add(
                    egui::DragValue::new(&mut self.config.timeout_ms)
                        .range(100..=5000)
                        .suffix(" ms"),
                );
            });
            if self.config.mode != previous {
                self.config.remote_port = match self.config.mode {
                    Mode::ModbusTcp => 502,
                    Mode::S7 => 102,
                    _ => 9000,
                };
                if self.config.mode == Mode::S7 {
                    self.order = ByteOrder::Abcd;
                }
                if self.config.mode == Mode::TcpServer && self.config.local_port == 0 {
                    self.config.local_port = 9000;
                }
                self.result = None;
                self.error = None;
            }
            if self.config.mode.is_serial() {
                self.serial_ui(ui);
            } else {
                if self.config.mode != Mode::TcpServer {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("目标 IP");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.remote_ip)
                                .desired_width(165.0),
                        );
                        ui.label("端口");
                        ui.add(egui::DragValue::new(&mut self.config.remote_port).range(1..=65535));
                    });
                }
                if matches!(self.config.mode, Mode::TcpServer | Mode::Udp) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("本地 IP");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.local_ip)
                                .desired_width(165.0),
                        );
                        ui.label("端口");
                        ui.add(egui::DragValue::new(&mut self.config.local_port).range(0..=65535));
                        ui.small("0 表示自动分配端口");
                    });
                }
                if self.config.mode == Mode::S7 {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Rack");
                        ui.add(egui::DragValue::new(&mut self.config.rack).range(0..=7));
                        ui.label("Slot");
                        ui.add(egui::DragValue::new(&mut self.config.slot).range(0..=31));
                        ui.label("连接类型");
                        egui::ComboBox::from_id_salt("s7_connection_type")
                            .selected_text(match self.config.connection_type {
                                2 => "OP",
                                3 => "S7",
                                _ => "PG",
                            })
                            .show_ui(ui, |ui| {
                                for (value, label) in [(1, "PG"), (2, "OP"), (3, "S7")] {
                                    ui.selectable_value(
                                        &mut self.config.connection_type,
                                        value,
                                        label,
                                    );
                                }
                            });
                    });
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.worker.is_none(), egui::Button::new("连接 / 打开"))
                .clicked()
            {
                match Worker::start(self.config.clone()) {
                    Ok(worker) => {
                        self.worker = Some(worker);
                        self.state = State::Connecting;
                        self.status = "正在连接".into();
                        self.error = None;
                        self.result = None;
                        self.tx_bytes = 0;
                        self.rx_bytes = 0;
                        self.worker_dropped = 0;
                        self.repeating = false;
                        ui.ctx().request_repaint();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            if ui
                .add_enabled(
                    self.worker.is_some() && self.state != State::Closing,
                    egui::Button::new("断开"),
                )
                .clicked()
            {
                if let Some(worker) = &self.worker {
                    worker.stop();
                }
                self.state = State::Closing;
                self.status = "正在关闭，等待当前 I/O 结束".into();
                self.repeating = false;
            }
            let (label, color) = match self.state {
                State::Connected => ("已连接", crate::theme::ok_green()),
                State::Connecting => ("连接中", crate::theme::amber()),
                State::Listening => ("监听中", crate::theme::accent()),
                State::Closing => ("关闭中", crate::theme::amber()),
                State::Closed => ("未连接", crate::theme::muted()),
            };
            crate::ui::pill(ui, label, color);
            ui.small("连接期间锁定参数；切换工具页保持连接");
        });
        ui.label(egui::RichText::new(&self.status).color(crate::theme::muted()));
        if let Some(error) = &self.error {
            ui.label(egui::RichText::new(error).color(crate::theme::danger()));
        }
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("TX {} B   RX {} B", self.tx_bytes, self.rx_bytes));
            ui.small("当前 / 最近一次连接统计");
        });
    }

    fn serial_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("串口");
            ui.add(
                egui::TextEdit::singleline(&mut self.config.serial_port)
                    .desired_width(100.0)
                    .hint_text("COM3"),
            );
            egui::ComboBox::from_id_salt("debug_ports")
                .selected_text("已发现端口")
                .show_ui(ui, |ui| {
                    for port in &self.ports {
                        ui.selectable_value(&mut self.config.serial_port, port.clone(), port);
                    }
                    if self.ports.is_empty() {
                        ui.label("暂无，可刷新或手填");
                    }
                });
            if ui
                .add_enabled(self.port_scan.is_none(), egui::Button::new("刷新串口"))
                .clicked()
            {
                let (sender, receiver) = mpsc::channel();
                let ctx = ui.ctx().clone();
                match std::thread::Builder::new()
                    .name("serial-enumeration".into())
                    .spawn(move || {
                        let result = serialport::available_ports()
                            .map(|ports| {
                                let mut names = ports
                                    .into_iter()
                                    .map(|port| port.port_name)
                                    .collect::<Vec<_>>();
                                names.sort();
                                names.dedup();
                                names
                            })
                            .map_err(|error| format!("枚举串口失败：{error}"));
                        let _ = sender.send(result);
                        ctx.request_repaint();
                    }) {
                    Ok(_) => self.port_scan = Some(receiver),
                    Err(error) => self.error = Some(format!("无法创建串口枚举线程：{error}")),
                }
            }
            if self.port_scan.is_some() {
                ui.spinner();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("波特率");
            ui.add(egui::DragValue::new(&mut self.config.baud_rate).range(1..=4_000_000));
            ui.label("数据位");
            ui.add(egui::DragValue::new(&mut self.config.data_bits).range(5..=8));
            ui.label("停止位");
            ui.add(egui::DragValue::new(&mut self.config.stop_bits).range(1..=2));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("校验");
            egui::ComboBox::from_id_salt("debug_parity")
                .selected_text(match self.config.parity {
                    1 => "奇校验",
                    2 => "偶校验",
                    _ => "无",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [(0, "无"), (1, "奇校验"), (2, "偶校验")] {
                        ui.selectable_value(&mut self.config.parity, value, label);
                    }
                });
            ui.label("流控");
            egui::ComboBox::from_id_salt("debug_flow")
                .selected_text(match self.config.flow_control {
                    1 => "XON/XOFF",
                    2 => "RTS/CTS",
                    _ => "无",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [(0, "无"), (1, "XON/XOFF"), (2, "RTS/CTS")] {
                        ui.selectable_value(&mut self.config.flow_control, value, label);
                    }
                });
        });
    }

    fn operation_ui(&mut self, ui: &mut egui::Ui) {
        crate::ui::section(
            ui,
            if self.config.mode.is_raw() {
                "发送与接收"
            } else {
                "设备读写"
            },
        );
        ui.add_enabled_ui(!self.repeating, |ui| {
            if self.config.mode.is_raw() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("发送格式");
                    ui.selectable_value(&mut self.raw_kind, ValueKind::Hex, "HEX");
                    ui.selectable_value(&mut self.raw_kind, ValueKind::Text, "UTF-8");
                    ui.label("追加");
                    egui::ComboBox::from_id_salt("debug_ending")
                        .selected_text(match self.ending {
                            Ending::None => "无",
                            Ending::Cr => "CR",
                            Ending::Lf => "LF",
                            Ending::CrLf => "CRLF",
                        })
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                (Ending::None, "无"),
                                (Ending::Cr, "CR"),
                                (Ending::Lf, "LF"),
                                (Ending::CrLf, "CRLF"),
                            ] {
                                ui.selectable_value(&mut self.ending, value, label);
                            }
                        });
                });
            } else if self.config.mode == Mode::S7 {
                self.s7_fields(ui);
            } else {
                self.modbus_fields(ui);
            }
            if !(self.config.mode == Mode::S7 && self.bit_mode) {
                ui.label(if self.config.mode.is_raw() {
                    "发送内容"
                } else {
                    "写入值（空白或逗号分隔；文本保留原样）"
                });
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .char_limit(262_144),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("数值类型");
            egui::ComboBox::from_id_salt("debug_kind")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for kind in [
                        ValueKind::U8,
                        ValueKind::I16,
                        ValueKind::U16,
                        ValueKind::I32,
                        ValueKind::U32,
                        ValueKind::F32,
                        ValueKind::Text,
                        ValueKind::Hex,
                    ] {
                        ui.selectable_value(&mut self.kind, kind, kind.label());
                    }
                });
            ui.label("字节序");
            egui::ComboBox::from_id_salt("debug_order")
                .selected_text(self.order.label())
                .show_ui(ui, |ui| {
                    for order in [
                        ByteOrder::Abcd,
                        ByteOrder::Badc,
                        ByteOrder::Cdab,
                        ByteOrder::Dcba,
                    ] {
                        ui.selectable_value(&mut self.order, order, order.label());
                    }
                });
        });
        ui.small(if self.config.mode.is_raw() {
            "结果按当前数值类型与字节序解释；原始发送仅使用上方 HEX / UTF-8 格式。"
        } else {
            "读结果与写入值均使用当前数值类型和字节序；写入值必须与填写的数量或长度一致。"
        });
        let connected = self.state == State::Connected;
        let writable = self.config.mode == Mode::S7
            || matches!(
                self.mb_area,
                modbus::Area::Coils | modbus::Area::HoldingRegisters
            );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    connected && !self.repeating,
                    egui::Button::new(if self.config.mode.is_raw() {
                        "发送一次"
                    } else {
                        "读取一次"
                    }),
                )
                .clicked()
            {
                self.dispatch(false, false);
            }
            if !self.config.mode.is_raw()
                && ui
                    .add_enabled(
                        connected && !self.repeating && writable,
                        egui::Button::new("写入设备"),
                    )
                    .clicked()
            {
                self.dispatch(true, false);
            }
            ui.label("周期");
            ui.add_enabled(
                !self.repeating,
                egui::DragValue::new(&mut self.period_ms)
                    .range(100..=3_600_000)
                    .suffix(" ms"),
            );
            if ui
                .add_enabled(
                    connected && !self.repeating,
                    egui::Button::new(if self.config.mode.is_raw() {
                        "开始周期发送"
                    } else {
                        "开始周期读取"
                    }),
                )
                .clicked()
            {
                self.dispatch(false, true);
            }
            if ui
                .add_enabled(self.repeating, egui::Button::new("停止周期"))
                .clicked()
                && let Some(worker) = &self.worker
            {
                match worker.stop_repeat() {
                    Ok(()) => self.status = "正在停止周期任务".into(),
                    Err(error) => self.error = Some(error),
                }
            }
        });
        if self.repeating {
            ui.colored_label(
                crate::theme::accent(),
                "周期任务运行中；使用启动时参数，停止后可编辑操作。",
            );
        }
        if !self.config.mode.is_raw() {
            ui.small("写入仅手动触发一次；不周期写入，不自动重试。");
        }
        self.result_ui(ui);
    }

    fn modbus_fields(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("区域");
            egui::ComboBox::from_id_salt("modbus_area")
                .selected_text(self.mb_area.label())
                .show_ui(ui, |ui| {
                    for area in [
                        modbus::Area::Coils,
                        modbus::Area::DiscreteInputs,
                        modbus::Area::HoldingRegisters,
                        modbus::Area::InputRegisters,
                    ] {
                        ui.selectable_value(&mut self.mb_area, area, area.label());
                    }
                });
            ui.label("站号");
            let range = if self.config.mode == Mode::ModbusRtu {
                1..=247
            } else {
                0..=255
            };
            ui.add(egui::DragValue::new(&mut self.unit).range(range));
        });
        let bits = matches!(
            self.mb_area,
            modbus::Area::Coils | modbus::Area::DiscreteInputs
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("起始地址（从 0 开始）");
            ui.add(egui::DragValue::new(&mut self.address).range(0..=65535));
            ui.label(if bits {
                "数量（位）"
            } else {
                "数量（16 位寄存器）"
            });
            ui.add(
                egui::DragValue::new(&mut self.quantity).range(1..=if bits { 2000 } else { 125 }),
            );
        });
        ui.small(if bits {
            "读取最多 2000 位；线圈写入最多 1968 位，值仅支持 0 / 1。"
        } else {
            "读取最多 125 个寄存器；写入最多 123 个。一个 F32 占 2 个寄存器，数量需填 2。"
        });
        ui.small("写入值须与数量一致；单个 / 多个写入功能码自动选择。输入区只读。");
    }

    fn s7_fields(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("区域");
            egui::ComboBox::from_id_salt("s7_area")
                .selected_text(self.s7_area.label())
                .show_ui(ui, |ui| {
                    for area in [
                        s7::Area::DataBlock,
                        s7::Area::Markers,
                        s7::Area::Inputs,
                        s7::Area::Outputs,
                    ] {
                        ui.selectable_value(&mut self.s7_area, area, area.label());
                    }
                });
            ui.label("DB 号");
            ui.add_enabled(
                self.s7_area == s7::Area::DataBlock,
                egui::DragValue::new(&mut self.db).range(1..=65535),
            );
            ui.checkbox(&mut self.bit_mode, "单个位");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("字节偏移");
            ui.add(egui::DragValue::new(&mut self.offset).range(0..=0x1f_ffff));
            if self.bit_mode {
                ui.label("位号");
                ui.add(egui::DragValue::new(&mut self.bit).range(0..=7));
                ui.checkbox(&mut self.bit_value, "写入 1（未勾选写入 0）");
            } else {
                ui.label("长度（字节）");
                ui.add(egui::DragValue::new(&mut self.length).range(1..=200));
            }
        });
        ui.small("经典 S7 绝对地址；DB 需使用可访问的非优化数据块，并在设备中允许所需访问。");
        ui.small("单次最多 200 字节，实际还受协商 PDU 限制；S7 日志仅记录有效载荷。");
    }

    fn dispatch(&mut self, write: bool, repeat: bool) {
        let operation = if self.config.mode.is_raw() {
            self.raw_operation()
        } else if self.config.mode == Mode::S7 {
            self.s7_operation(write)
        } else {
            self.modbus_operation(write)
        };
        let result = operation.and_then(|operation| {
            let worker = self
                .worker
                .as_ref()
                .ok_or_else(|| "请先连接设备".to_owned())?;
            if repeat {
                worker.repeat(operation, Duration::from_millis(self.period_ms))
            } else {
                worker.execute(operation)
            }
        });
        match result {
            Ok(()) => {
                self.error = None;
                self.status = "请求已加入后台队列".into();
                if repeat {
                    self.repeating = true;
                }
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn result_ui(&self, ui: &mut egui::Ui) {
        let Some(reply) = &self.result else {
            return;
        };
        ui.add_space(6.0);
        crate::ui::section(ui, "最新结果");
        let result = match reply {
            Reply::Bytes(bytes) => codec::decode_values(bytes, self.kind, self.order),
            Reply::Bits(bits) => Ok(bits
                .iter()
                .map(|bit| if *bit { "1" } else { "0" })
                .collect::<Vec<_>>()
                .join(" ")),
            Reply::Written(count) => Ok(format!(
                "写入成功，数量 {count}（Modbus 为位 / 寄存器数，S7 为字节数或单个位）"
            )),
        };
        egui::ScrollArea::vertical()
            .id_salt("debug_result")
            .max_height(110.0)
            .show(ui, |ui| match result {
                Ok(value) => {
                    ui.label(egui::RichText::new(value).monospace());
                }
                Err(error) => {
                    ui.colored_label(crate::theme::danger(), format!("解析失败：{error}"));
                }
            });
        if self.config.mode.is_raw() {
            ui.small("原始模式展示最近一次接收块；TCP 接收块不等于完整应用报文。");
        }
    }

    fn records_ui(&mut self, ui: &mut egui::Ui) {
        crate::ui::section(ui, "通信记录");
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.log_text, "UTF-8 预览");
            ui.checkbox(&mut self.paused, "暂停显示");
            if ui.button("清空记录").clicked() {
                self.logs.clear();
                self.log_bytes = 0;
                self.discarded = 0;
            }
            if ui
                .add_enabled(
                    !self.logs.is_empty(),
                    egui::Button::new("复制完整记录（HEX）"),
                )
                .clicked()
            {
                ui.ctx().copy_text(self.copy_logs());
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.small(format!(
                "{} / {MAX_LOGS} 条 · {:.1} / 2048 KiB",
                self.logs.len(),
                self.log_bytes as f64 / 1024.0
            ));
            if self.discarded > 0 || self.worker_dropped > 0 {
                ui.colored_label(
                    crate::theme::amber(),
                    format!(
                        "容量淘汰 {} 条 · 后台丢弃 {} 条",
                        self.discarded, self.worker_dropped
                    ),
                );
            }
        });
        ui.small(
            "相对工具启动时间；每条预览最多 512 字节。S7 记录是有效载荷，其他模式记录收发字节。",
        );
        if self.log_text {
            ui.small("文本预览使用 UTF-8；无效字节以替代字符显示，复制仍保留原字节。");
        }
        if self.paused {
            ui.label("显示已暂停；后台继续收发并保留最新记录，取消勾选后显示。");
            return;
        }
        if self.logs.is_empty() {
            ui.label(egui::RichText::new("等待通信记录…").color(crate::theme::muted()));
        }
        let height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
        egui::ScrollArea::vertical()
            .id_salt("debug_log_rows")
            .max_height(260.0)
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show_rows(ui, height, self.logs.len(), |ui, rows| {
                for index in rows {
                    let record = &self.logs[index];
                    let payload = preview(&record.log.bytes, self.log_text);
                    let detail = record
                        .log
                        .detail
                        .chars()
                        .take(80)
                        .collect::<String>()
                        .replace(['\r', '\n', '\t'], " ");
                    let line = format!(
                        "{:>6}.{:03} {} {:>5} B {} | {}",
                        record.elapsed.as_secs(),
                        record.elapsed.subsec_millis(),
                        if record.log.sent { "TX" } else { "RX" },
                        record.log.bytes.len(),
                        detail,
                        payload
                    );
                    let color = if record.log.sent {
                        crate::theme::accent()
                    } else {
                        crate::theme::text()
                    };
                    ui.add_sized(
                        [ui.available_width(), height],
                        egui::Label::new(egui::RichText::new(line).monospace().color(color))
                            .truncate(),
                    )
                    .on_hover_text(payload);
                }
            });
    }

    fn raw_operation(&self) -> Result<Operation, String> {
        let mut bytes = codec::encode_values(&self.input, self.raw_kind, ByteOrder::Abcd)?;
        bytes.extend_from_slice(match self.ending {
            Ending::None => &[],
            Ending::Cr => b"\r",
            Ending::Lf => b"\n",
            Ending::CrLf => b"\r\n",
        });
        if bytes.is_empty() {
            return Err("发送内容不能为空".into());
        }
        if bytes.len() > 65_507 {
            return Err("单次发送最多 65507 字节".into());
        }
        Ok(Operation::Raw(bytes))
    }

    fn modbus_operation(&self, write: bool) -> Result<Operation, String> {
        if self.config.mode == Mode::ModbusRtu && !(1..=247).contains(&self.unit) {
            return Err("Modbus RTU 站号必须为 1～247".into());
        }
        let request = if !write {
            modbus::Request::read(self.unit, self.mb_area, self.address, self.quantity)?
        } else {
            match self.mb_area {
                modbus::Area::Coils => {
                    let values = self
                        .input
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter(|token| !token.is_empty())
                        .map(|token| match token {
                            "0" => Ok(false),
                            "1" => Ok(true),
                            _ => Err("线圈值只能为 0 或 1".to_owned()),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if values.len() != usize::from(self.quantity) {
                        return Err("输入位数与数量不一致".into());
                    }
                    modbus::Request::write_coils(self.unit, self.address, &values)?
                }
                modbus::Area::HoldingRegisters => {
                    let bytes = codec::encode_values(&self.input, self.kind, self.order)?;
                    if bytes.len() % 2 != 0 {
                        return Err("寄存器写入字节数必须为偶数".into());
                    }
                    if bytes.len() / 2 != usize::from(self.quantity) {
                        return Err("输入数据对应的寄存器数与数量不一致".into());
                    }
                    let values = bytes
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|b| u16::from_be_bytes(*b))
                        .collect::<Vec<_>>();
                    modbus::Request::write_registers(self.unit, self.address, &values)?
                }
                _ => return Err("离散输入与输入寄存器仅支持读取".into()),
            }
        };
        Ok(Operation::Modbus(request))
    }

    fn s7_operation(&self, write: bool) -> Result<Operation, String> {
        let db = if self.s7_area == s7::Area::DataBlock {
            self.db
        } else {
            0
        };
        if self.s7_area == s7::Area::DataBlock && db == 0 {
            return Err("DB 号必须为 1～65535".into());
        }
        let length = if self.bit_mode { 1 } else { self.length };
        if !(1..=200).contains(&length) {
            return Err("S7 单次长度应为 1～200 字节".into());
        }
        if self.offset > 0x1f_ffff || u64::from(self.offset) + u64::from(length) > 0x20_0000 {
            return Err("S7 字节地址超出 24 位位地址范围".into());
        }
        if self.bit_mode && self.bit > 7 {
            return Err("位号必须为 0～7".into());
        }
        let bit = self.bit_mode.then_some(self.bit);
        if write {
            let data = if self.bit_mode {
                vec![u8::from(self.bit_value)]
            } else {
                codec::encode_values(&self.input, self.kind, self.order)?
            };
            if data.len() != usize::from(length) {
                return Err("输入数据字节数与长度不一致".into());
            }
            Ok(Operation::S7Write {
                area: self.s7_area,
                db,
                offset: self.offset,
                data,
                bit,
            })
        } else {
            Ok(Operation::S7Read {
                area: self.s7_area,
                db,
                offset: self.offset,
                length,
                bit,
            })
        }
    }

    fn push_log(&mut self, log: Log) {
        let size = log.bytes.len() + log.detail.len();
        if size > MAX_LOG_BYTES {
            self.discarded += 1;
            return;
        }
        while self.logs.len() >= MAX_LOGS || self.log_bytes + size > MAX_LOG_BYTES {
            if let Some(old) = self.logs.pop_front() {
                self.log_bytes -= old.log.bytes.len() + old.log.detail.len();
                self.discarded += 1;
            }
        }
        self.log_bytes += size;
        self.logs.push_back(Record {
            elapsed: self.started.elapsed(),
            log,
        });
    }

    fn copy_logs(&self) -> String {
        use std::fmt::Write;
        let mut output = String::from("相对时间(秒)\t方向\t字节数\t说明\tHEX\n");
        for record in &self.logs {
            let _ = writeln!(
                output,
                "{}.{:03}\t{}\t{}\t{}\t{}",
                record.elapsed.as_secs(),
                record.elapsed.subsec_millis(),
                if record.log.sent { "TX" } else { "RX" },
                record.log.bytes.len(),
                record.log.detail.replace(['\r', '\n', '\t'], " "),
                codec::format_hex(&record.log.bytes)
            );
        }
        output
    }
}

fn preview(bytes: &[u8], text: bool) -> String {
    let displayed = &bytes[..bytes.len().min(PREVIEW_BYTES)];
    let mut output = if text {
        String::from_utf8_lossy(displayed)
            .replace('\r', "\\r")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    } else {
        codec::format_hex(displayed)
    };
    if bytes.len() > PREVIEW_BYTES {
        output.push_str(" …（预览已截断，复制保留完整数据）");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_page_poll_consumes_data_and_finishes_disconnect() {
        use std::net::UdpSocket;
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut screen = DebugScreen::new();
        screen.config.mode = Mode::Udp;
        screen.config.remote_port = server.local_addr().unwrap().port();
        screen.worker = Some(Worker::start(screen.config.clone()).unwrap());
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(2);
        while screen.state != State::Connected {
            screen.poll(&ctx);
            assert!(Instant::now() < deadline, "连接未完成：{:?}", screen.error);
            std::thread::sleep(Duration::from_millis(5));
        }
        screen
            .worker
            .as_ref()
            .unwrap()
            .execute(Operation::Raw(vec![1, 2]))
            .unwrap();
        let (_, peer) = server.recv_from(&mut [0; 8]).unwrap();
        server.send_to(&[3, 4, 5], peer).unwrap();
        // 不调用 show，模拟停留在其他工具页；暂停显示也必须继续消费并保留记录。
        screen.paused = true;
        while screen.rx_bytes < 3 {
            screen.poll(&ctx);
            assert!(Instant::now() < deadline, "隐藏页面未消费接收队列");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(screen.tx_bytes, 2);
        assert!(
            screen
                .logs
                .iter()
                .any(|record| !record.log.sent && record.log.bytes == [3, 4, 5])
        );
        screen.worker.as_ref().unwrap().stop();
        screen.state = State::Closing;
        while screen.worker.is_some() {
            screen.poll(&ctx);
            assert!(Instant::now() < deadline, "断开后未回收通信线程");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(screen.state, State::Closed);
    }

    #[test]
    fn raw_hex_and_utf8_append_exact_line_endings() {
        let mut screen = DebugScreen::new();
        screen.input = "00 ff".into();
        screen.ending = Ending::CrLf;
        let Operation::Raw(bytes) = screen.raw_operation().unwrap() else {
            panic!("应为原始发送")
        };
        assert_eq!(bytes, [0, 255, 13, 10]);
        screen.raw_kind = ValueKind::Text;
        screen.input = "中".into();
        screen.ending = Ending::Lf;
        let Operation::Raw(bytes) = screen.raw_operation().unwrap() else {
            panic!("应为原始发送")
        };
        assert_eq!(bytes, [0xe4, 0xb8, 0xad, 10]);
    }

    #[test]
    fn modbus_read_quantity_is_register_count_independent_of_value_kind() {
        let mut screen = DebugScreen::new();
        screen.kind = ValueKind::F32;
        screen.quantity = 2;
        let Operation::Modbus(request) = screen.modbus_operation(false).unwrap() else {
            panic!("应为 Modbus")
        };
        assert_eq!(&request.encode_tcp(1)[7..], &[3, 0, 0, 0, 2]);
    }

    #[test]
    fn modbus_float_write_uses_two_registers_and_refuses_odd_bytes() {
        let mut screen = DebugScreen::new();
        screen.kind = ValueKind::F32;
        screen.quantity = 2;
        screen.input = "1.0".into();
        let Operation::Modbus(request) = screen.modbus_operation(true).unwrap() else {
            panic!("应为 Modbus")
        };
        assert_eq!(
            &request.encode_tcp(1)[7..],
            &[16, 0, 0, 0, 2, 4, 0x3f, 0x80, 0, 0]
        );
        screen.kind = ValueKind::Hex;
        screen.input = "01 02 03".into();
        assert!(screen.modbus_operation(true).unwrap_err().contains("偶数"));
    }

    #[test]
    fn modbus_writes_validate_count_access_and_rtu_station() {
        let mut screen = DebugScreen::new();
        screen.mb_area = modbus::Area::Coils;
        screen.input = "1,0 1".into();
        screen.quantity = 3;
        let Operation::Modbus(request) = screen.modbus_operation(true).unwrap() else {
            panic!("应为 Modbus")
        };
        assert_eq!(&request.encode_tcp(1)[7..], &[15, 0, 0, 0, 3, 1, 5]);
        screen.quantity = 2;
        assert!(screen.modbus_operation(true).is_err());
        screen.mb_area = modbus::Area::InputRegisters;
        assert!(screen.modbus_operation(true).is_err());
        screen.config.mode = Mode::ModbusRtu;
        for unit in [0, 248, 255] {
            screen.unit = unit;
            assert!(screen.modbus_operation(false).is_err());
        }
    }

    #[test]
    fn s7_bit_and_byte_operations_keep_address_units() {
        let mut screen = DebugScreen::new();
        screen.s7_area = s7::Area::Markers;
        screen.offset = 12;
        screen.bit_mode = true;
        screen.bit = 7;
        screen.bit_value = true;
        let Operation::S7Write {
            area,
            db,
            offset,
            data,
            bit,
        } = screen.s7_operation(true).unwrap()
        else {
            panic!("应为 S7 写入")
        };
        assert_eq!(area, s7::Area::Markers);
        assert_eq!((db, offset, bit), (0, 12, Some(7)));
        assert_eq!(data, [1]);
        screen.bit_mode = false;
        screen.length = 4;
        let Operation::S7Read {
            offset,
            length,
            bit,
            ..
        } = screen.s7_operation(false).unwrap()
        else {
            panic!("应为 S7 读取")
        };
        assert_eq!((offset, length, bit), (12, 4, None));
        screen.kind = ValueKind::F32;
        screen.input = "1".into();
        assert!(screen.s7_operation(true).is_ok());
        screen.length = 3;
        assert!(screen.s7_operation(true).is_err());
    }

    #[test]
    fn s7_rejects_invalid_address_and_oversized_payload() {
        let mut screen = DebugScreen::new();
        screen.db = 0;
        assert!(screen.s7_operation(false).is_err());
        screen.db = 1;
        screen.length = 201;
        assert!(screen.s7_operation(false).is_err());
        screen.length = 2;
        screen.offset = 0x1f_ffff;
        assert!(screen.s7_operation(false).is_err());
        screen.offset = 0;
        screen.bit_mode = true;
        screen.bit = 8;
        assert!(screen.s7_operation(false).is_err());
    }

    #[test]
    fn log_buffer_retains_latest_entries_with_both_limits() {
        let mut screen = DebugScreen::new();
        for index in 0..=MAX_LOGS {
            screen.push_log(Log {
                sent: false,
                bytes: vec![1],
                detail: index.to_string(),
            });
        }
        assert_eq!(screen.logs.len(), MAX_LOGS);
        assert_eq!(screen.logs.front().unwrap().log.detail, "1");
        assert_eq!(screen.discarded, 1);
        for _ in 0..40 {
            screen.push_log(Log {
                sent: true,
                bytes: vec![2; 65_536],
                detail: "发送".into(),
            });
        }
        assert!(screen.log_bytes <= MAX_LOG_BYTES);
        assert!(screen.logs.len() < MAX_LOGS);
        assert_eq!(
            screen.log_bytes,
            screen
                .logs
                .iter()
                .map(|record| record.log.bytes.len() + record.log.detail.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn oversized_log_is_dropped_and_copy_keeps_full_retained_payload() {
        let mut screen = DebugScreen::new();
        screen.push_log(Log {
            sent: false,
            bytes: vec![0; MAX_LOG_BYTES + 1],
            detail: String::new(),
        });
        assert!(screen.logs.is_empty());
        assert_eq!(screen.discarded, 1);
        let mut bytes = vec![0; PREVIEW_BYTES];
        bytes.extend_from_slice(&[0xab, 0xcd]);
        screen.push_log(Log {
            sent: true,
            bytes,
            detail: "完整复制".into(),
        });
        assert!(screen.copy_logs().contains("AB CD"));
        assert!(screen.copy_logs().contains("完整复制"));
        assert!(!preview(&screen.logs[0].log.bytes, false).contains("AB CD"));
        assert!(preview(&screen.logs[0].log.bytes, false).contains("截断"));
    }
}
