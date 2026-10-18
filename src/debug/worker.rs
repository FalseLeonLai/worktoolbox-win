//! 后台线程独占连接；GUI 只发送命令并接收事件。

use std::cell::Cell;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{modbus, s7};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    TcpClient,
    TcpServer,
    Udp,
    Serial,
    ModbusTcp,
    ModbusRtu,
    S7,
}

impl Mode {
    pub const ALL: [Self; 7] = [
        Self::TcpClient,
        Self::TcpServer,
        Self::Udp,
        Self::Serial,
        Self::ModbusTcp,
        Self::ModbusRtu,
        Self::S7,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::TcpClient => "TCP 客户端",
            Self::TcpServer => "TCP 服务端",
            Self::Udp => "UDP",
            Self::Serial => "串口",
            Self::ModbusTcp => "Modbus TCP",
            Self::ModbusRtu => "Modbus RTU",
            Self::S7 => "Siemens S7",
        }
    }
    pub fn is_serial(self) -> bool {
        matches!(self, Self::Serial | Self::ModbusRtu)
    }
    pub fn is_raw(self) -> bool {
        matches!(
            self,
            Self::TcpClient | Self::TcpServer | Self::Udp | Self::Serial
        )
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub mode: Mode,
    pub remote_ip: String,
    pub remote_port: u16,
    pub local_ip: String,
    pub local_port: u16,
    pub serial_port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: u8,
    pub flow_control: u8,
    pub timeout_ms: u64,
    pub rack: u8,
    pub slot: u8,
    pub connection_type: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::TcpClient,
            remote_ip: "127.0.0.1".into(),
            remote_port: 9000,
            local_ip: "0.0.0.0".into(),
            local_port: 0,
            serial_port: String::new(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: 0,
            flow_control: 0,
            timeout_ms: 1000,
            rack: 0,
            slot: 1,
            connection_type: 1,
        }
    }
}

impl Config {
    fn endpoint(&self) -> Result<SocketAddr, String> {
        let ip = self
            .remote_ip
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| "请输入有效的目标 IPv4 或 IPv6 地址".to_owned())?;
        if self.remote_port == 0 {
            return Err("目标端口必须为 1～65535".into());
        }
        Ok(SocketAddr::new(ip, self.remote_port))
    }
    fn bind_address(&self) -> Result<SocketAddr, String> {
        let ip = self
            .local_ip
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| "请输入有效的本地 IPv4 或 IPv6 地址".to_owned())?;
        Ok(SocketAddr::new(ip, self.local_port))
    }
    fn validate(&self) -> Result<(), String> {
        if !(100..=5000).contains(&self.timeout_ms) {
            return Err("超时应为 100～5000 ms".into());
        }
        if self.mode.is_serial() {
            if self.serial_port.trim().is_empty() {
                return Err("请选择或填写串口名称".into());
            }
            if self.baud_rate == 0
                || !(5..=8).contains(&self.data_bits)
                || !(1..=2).contains(&self.stop_bits)
                || self.parity > 2
                || self.flow_control > 2
            {
                return Err("串口参数无效".into());
            }
        } else if self.mode == Mode::TcpServer {
            self.bind_address()?;
        } else {
            self.endpoint()?;
            if self.mode == Mode::Udp {
                self.bind_address()?;
            }
        }
        if self.mode == Mode::S7
            && (self.rack > 7 || self.slot > 31 || !(1..=3).contains(&self.connection_type))
        {
            return Err("S7 Rack 应为 0～7、Slot 为 0～31，连接类型为 PG/OP/S7".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Operation {
    Raw(Vec<u8>),
    Modbus(modbus::Request),
    S7Read {
        area: s7::Area,
        db: u16,
        offset: u32,
        length: u16,
        bit: Option<u8>,
    },
    S7Write {
        area: s7::Area,
        db: u16,
        offset: u32,
        data: Vec<u8>,
        bit: Option<u8>,
    },
}

#[derive(Debug)]
pub enum Reply {
    Bytes(Vec<u8>),
    Bits(Vec<bool>),
    Written(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Connecting,
    Listening,
    Connected,
    Closing,
    Closed,
}

#[derive(Debug)]
pub enum Event {
    State(State, String),
    Peer(u64),
    Result(Reply),
    Error(String),
    RepeatStopped,
}

#[derive(Debug)]
pub struct Log {
    pub sent: bool,
    pub bytes: Vec<u8>,
    pub detail: String,
}

enum Command {
    Execute(Operation, u64),
    Repeat(Operation, Duration, u64),
    StopRepeat,
}

pub struct Worker {
    commands: mpsc::SyncSender<Command>,
    pub events: mpsc::Receiver<Event>,
    logs: Arc<Mutex<VecDeque<Log>>>,
    displayed_peer: AtomicU64,
    stop: Arc<AtomicBool>,
    pub tx_bytes: Arc<AtomicU64>,
    pub rx_bytes: Arc<AtomicU64>,
    pub dropped_logs: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(config: Config) -> Result<Self, String> {
        config.validate()?;
        let (commands, receiver) = mpsc::sync_channel(16);
        let (event_sender, events) = mpsc::sync_channel(64);
        let logs = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_PENDING_LOGS)));
        let stop = Arc::new(AtomicBool::new(false));
        let tx_bytes = Arc::new(AtomicU64::new(0));
        let rx_bytes = Arc::new(AtomicU64::new(0));
        let dropped_logs = Arc::new(AtomicU64::new(0));
        let context = Context {
            events: event_sender,
            logs: logs.clone(),
            actual_peer: Cell::new(0),
            stop: stop.clone(),
            tx: tx_bytes.clone(),
            rx: rx_bytes.clone(),
            dropped: dropped_logs.clone(),
        };
        let thread = std::thread::Builder::new()
            .name("communication".into())
            .spawn(move || {
                context.state(State::Connecting, "正在建立连接".into());
                if let Err(error) = run(&config, receiver, &context)
                    && !context.stopped()
                {
                    let _ = context.events.try_send(Event::Error(error));
                }
                let _ = context.events.try_send(Event::RepeatStopped);
                context.state(State::Closed, "已断开".into());
            })
            .map_err(|e| format!("无法创建通信线程：{e}"))?;
        Ok(Self {
            commands,
            events,
            logs,
            displayed_peer: AtomicU64::new(0),
            stop,
            tx_bytes,
            rx_bytes,
            dropped_logs,
            thread: Some(thread),
        })
    }
    pub fn execute(&self, op: Operation) -> Result<(), String> {
        self.commands
            .try_send(Command::Execute(
                op,
                self.displayed_peer.load(Ordering::Acquire),
            ))
            .map_err(|_| "操作队列已满或连接已关闭".into())
    }
    pub fn repeat(&self, op: Operation, period: Duration) -> Result<(), String> {
        if period < Duration::from_millis(100) {
            return Err("周期不能小于 100 ms".into());
        }
        if matches!(&op, Operation::S7Write { .. })
            || matches!(&op, Operation::Modbus(request) if !request.is_read())
        {
            return Err("不支持周期写入 PLC".into());
        }
        self.commands
            .try_send(Command::Repeat(
                op,
                period,
                self.displayed_peer.load(Ordering::Acquire),
            ))
            .map_err(|_| "操作队列已满或连接已关闭".into())
    }
    pub fn stop_repeat(&self) -> Result<(), String> {
        self.commands
            .try_send(Command::StopRepeat)
            .map_err(|_| "操作队列已满或连接已关闭，请重试停止或断开连接".into())
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub fn acknowledge_peer(&self, peer: u64) {
        self.displayed_peer.store(peer, Ordering::Release);
    }
    pub fn take_logs(&self) -> Vec<Log> {
        self.logs.lock().unwrap().drain(..).collect()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

const POLL: Duration = Duration::from_millis(20);
const MAX_PENDING_LOGS: usize = 128;

struct Context {
    events: mpsc::SyncSender<Event>,
    logs: Arc<Mutex<VecDeque<Log>>>,
    actual_peer: Cell<u64>,
    stop: Arc<AtomicBool>,
    tx: Arc<AtomicU64>,
    rx: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
}

impl Context {
    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
    fn state(&self, state: State, detail: String) {
        if self.events.try_send(Event::State(state, detail)).is_err() {
            // 界面无法得知连接变更时停止通信，不能继续使用可能过期的目标信息。
            self.stop.store(true, Ordering::Release);
        }
    }
    fn change_peer(&self) -> Result<(), String> {
        let peer = self.actual_peer.get().wrapping_add(1).max(1);
        self.actual_peer.set(peer);
        // GUI 确认这次身份变更后才能向新客户端提交操作。
        self.events
            .try_send(Event::Peer(peer))
            .map_err(|_| "客户端状态队列已满，连接已停止；请重新连接".to_owned())
    }
    fn log(&self, sent: bool, bytes: &[u8], detail: String) {
        if sent { &self.tx } else { &self.rx }.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        let log = Log {
            sent,
            bytes: bytes.to_vec(),
            detail,
        };
        let mut logs = self.logs.lock().unwrap();
        if logs.len() == MAX_PENDING_LOGS {
            logs.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        logs.push_back(log);
    }
}

enum Connection {
    Tcp(TcpStream),
    Server(TcpListener, Option<TcpStream>),
    Udp(UdpSocket, SocketAddr),
    Serial(Box<dyn serialport::SerialPort>),
    S7(s7::Client),
}

fn prepare_tcp(stream: &TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(POLL))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(POLL))
        .map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())
}

fn open(config: &Config, cx: &Context) -> Result<Connection, String> {
    let timeout = Duration::from_millis(config.timeout_ms);
    let connection = match config.mode {
        Mode::TcpClient | Mode::ModbusTcp => {
            let stream = TcpStream::connect_timeout(&config.endpoint()?, timeout)
                .map_err(|e| format!("TCP 连接失败：{e}"))?;
            prepare_tcp(&stream)?;
            Connection::Tcp(stream)
        }
        Mode::TcpServer => {
            let listener =
                TcpListener::bind(config.bind_address()?).map_err(|e| format!("监听失败：{e}"))?;
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;
            cx.state(
                State::Listening,
                format!(
                    "正在监听 {}，等待客户端",
                    listener.local_addr().map_err(|e| e.to_string())?
                ),
            );
            return Ok(Connection::Server(listener, None));
        }
        Mode::Udp => {
            let socket = UdpSocket::bind(config.bind_address()?)
                .map_err(|e| format!("UDP 绑定失败：{e}"))?;
            socket
                .set_read_timeout(Some(POLL))
                .map_err(|e| e.to_string())?;
            socket
                .set_write_timeout(Some(timeout))
                .map_err(|e| e.to_string())?;
            let local = socket.local_addr().map_err(|e| e.to_string())?;
            cx.state(
                State::Connected,
                format!("UDP 已绑定 {local}，目标 {}", config.endpoint()?),
            );
            return Ok(Connection::Udp(socket, config.endpoint()?));
        }
        Mode::Serial | Mode::ModbusRtu => {
            use serialport::{DataBits, FlowControl, Parity, StopBits};
            let port = serialport::new(config.serial_port.trim(), config.baud_rate)
                .data_bits(match config.data_bits {
                    5 => DataBits::Five,
                    6 => DataBits::Six,
                    7 => DataBits::Seven,
                    _ => DataBits::Eight,
                })
                .stop_bits(if config.stop_bits == 2 {
                    StopBits::Two
                } else {
                    StopBits::One
                })
                .parity(match config.parity {
                    1 => Parity::Odd,
                    2 => Parity::Even,
                    _ => Parity::None,
                })
                .flow_control(match config.flow_control {
                    1 => FlowControl::Software,
                    2 => FlowControl::Hardware,
                    _ => FlowControl::None,
                })
                .timeout(POLL)
                .open()
                .map_err(|e| format!("打开串口失败：{e}"))?;
            Connection::Serial(port)
        }
        Mode::S7 => Connection::S7(s7::Client::connect(s7::Config {
            endpoint: config.endpoint()?,
            local_tsap: 0x0100,
            remote_tsap: (u16::from(config.connection_type) << 8)
                | (u16::from(config.rack) << 5)
                | u16::from(config.slot),
            timeout,
        })?),
    };
    cx.state(State::Connected, format!("{} 已连接", config.mode.label()));
    Ok(connection)
}

fn retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}

fn check_deadline(deadline: Instant, stop: &AtomicBool) -> Result<(), String> {
    if stop.load(Ordering::Acquire) {
        Err("操作已取消".into())
    } else if Instant::now() >= deadline {
        Err("通信超时，操作未自动重试".into())
    } else {
        Ok(())
    }
}

fn read_exact_until<T: Read + ?Sized>(
    stream: &mut T,
    mut bytes: &mut [u8],
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<(), String> {
    while !bytes.is_empty() {
        check_deadline(deadline, stop)?;
        match stream.read(bytes) {
            Ok(0) => return Err("连接已关闭或响应中断".into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(e) if retryable(&e) => {}
            Err(e) => return Err(format!("接收失败：{e}")),
        }
    }
    Ok(())
}

fn write_until<T: Write + ?Sized>(
    stream: &mut T,
    mut bytes: &[u8],
    deadline: Instant,
    stop: &AtomicBool,
    serial: bool,
) -> Result<(), String> {
    while !bytes.is_empty() {
        check_deadline(deadline, stop)?;
        match stream.write(bytes) {
            // Windows 串口 WriteFile 超时可能返回成功且长度为 0；按总期限等待流控恢复。
            Ok(0) if serial => std::thread::sleep(
                Duration::from_millis(1).min(deadline.saturating_duration_since(Instant::now())),
            ),
            Ok(0) => return Err("连接无法继续发送".into()),
            Ok(count) => bytes = &bytes[count..],
            Err(e) if retryable(&e) => {}
            Err(e) => return Err(format!("发送失败：{e}")),
        }
    }
    Ok(())
}

fn rtu_silence(config: &Config) -> Duration {
    if config.baud_rate > 19200 {
        return Duration::from_micros(1750);
    }
    let bits = 1 + config.data_bits + u8::from(config.parity != 0) + config.stop_bits;
    Duration::from_secs_f64(f64::from(bits) * 3.5 / f64::from(config.baud_rate))
}

// 调用方串口读取超时至多 POLL；持续排空迟到数据，静默和总期限分别计时。
// USB/Windows 只能观察驱动交付批次，无法可靠验证逐字节 t1.5 UART 间隔。
fn wait_rtu_quiet<T: Read + ?Sized>(
    stream: &mut T,
    silence: Duration,
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<(), String> {
    let mut last_activity = Instant::now();
    let mut residual = [0; 256];
    loop {
        check_deadline(deadline, stop)?;
        match stream.read(&mut residual) {
            Ok(0) => {}
            Ok(_) => {
                last_activity = Instant::now();
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if retryable(&error) => {}
            Err(error) => return Err(format!("等待 RTU 总线静默失败：{error}")),
        }
        check_deadline(deadline, stop)?;
        let elapsed = last_activity.elapsed();
        if elapsed >= silence {
            return Ok(());
        }
        let pause = Duration::from_millis(1)
            .min(silence - elapsed)
            .min(deadline.saturating_duration_since(Instant::now()));
        std::thread::sleep(pause);
    }
}

fn read_modbus_tcp<T: Read + ?Sized>(
    stream: &mut T,
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let mut header = [0; 7];
    read_exact_until(stream, &mut header, deadline, stop)?;
    let len = u16::from_be_bytes([header[4], header[5]]) as usize;
    if !(2..=254).contains(&len) {
        return Err("Modbus TCP 长度超出协议范围".into());
    }
    let mut frame = vec![0; len + 6];
    frame[..7].copy_from_slice(&header);
    read_exact_until(stream, &mut frame[7..], deadline, stop)?;
    Ok(frame)
}

fn read_modbus_rtu<T: Read + ?Sized>(
    stream: &mut T,
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let mut prefix = [0; 3];
    read_exact_until(stream, &mut prefix, deadline, stop)?;
    let len = modbus::rtu_response_len(&prefix)?.ok_or("RTU 响应头不完整")?;
    if !(5..=256).contains(&len) {
        return Err("RTU 响应长度超出协议范围".into());
    }
    let mut frame = vec![0; len];
    frame[..3].copy_from_slice(&prefix);
    read_exact_until(stream, &mut frame[3..], deadline, stop)?;
    Ok(frame)
}

fn run(config: &Config, commands: mpsc::Receiver<Command>, cx: &Context) -> Result<(), String> {
    let mut connection = open(config, cx)?;
    let mut repeat: Option<(Operation, Duration, Instant)> = None;
    let mut transaction = 0u16;
    while !cx.stopped() {
        let command = commands.try_recv();
        let operation = match command {
            Ok(Command::Execute(op, peer)) if peer == cx.actual_peer.get() => Some(op),
            Ok(Command::Repeat(op, interval, peer)) if peer == cx.actual_peer.get() => {
                repeat = Some((op.clone(), interval, Instant::now() + interval));
                Some(op)
            }
            Ok(Command::Execute(_, _) | Command::Repeat(_, _, _)) => {
                repeat = None;
                let _ = cx.events.try_send(Event::Error(
                    "客户端已变更，旧客户端的操作已丢弃；请确认当前客户端后重新发送".into(),
                ));
                let _ = cx.events.try_send(Event::RepeatStopped);
                None
            }
            Ok(Command::StopRepeat) => {
                repeat = None;
                let _ = cx.events.try_send(Event::RepeatStopped);
                None
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {
                if let Some((op, interval, next)) = &mut repeat {
                    if Instant::now() >= *next {
                        *next = Instant::now() + *interval;
                        Some(op.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        if let Some(operation) = operation {
            if let Some(reply) = execute(&mut connection, config, operation, &mut transaction, cx)?
            {
                cx.events
                    .try_send(Event::Result(reply))
                    .map_err(|_| "结果队列已满，连接已停止；请重新连接".to_owned())?;
            }
            if matches!(&connection, Connection::Server(_, None)) {
                if repeat.take().is_some() {
                    let _ = cx.events.try_send(Event::RepeatStopped);
                }
            } else {
                // 周期按上次操作完成后计时，慢设备不会积累排队请求。
                if let Some((_, interval, next)) = &mut repeat {
                    *next = Instant::now() + *interval;
                }
            }
        }
        if config.mode.is_raw() {
            if poll_raw(&mut connection, cx)? {
                repeat = None;
                let _ = cx.events.try_send(Event::RepeatStopped);
            }
        } else {
            std::thread::sleep(POLL);
        }
    }
    if let Connection::S7(client) = &mut connection {
        client.disconnect();
    }
    Ok(())
}

// 返回 true 表示服务端的客户端已退出，需停止周期任务。
fn poll_raw(connection: &mut Connection, cx: &Context) -> Result<bool, String> {
    let mut bytes = [0; 65_535];
    match connection {
        Connection::Server(listener, peer) => {
            if peer.is_none() {
                match listener.accept() {
                    Ok((stream, address)) => {
                        prepare_tcp(&stream)?;
                        *peer = Some(stream);
                        cx.change_peer()?;
                        cx.state(State::Connected, format!("客户端 {address} 已接入"));
                    }
                    Err(e) if retryable(&e) => std::thread::sleep(POLL),
                    Err(e) => return Err(format!("接受连接失败：{e}")),
                }
            } else if let Some(stream) = peer {
                match stream.read(&mut bytes[..4096]) {
                    Ok(0) => {
                        *peer = None;
                        cx.change_peer()?;
                        cx.state(
                            State::Listening,
                            format!(
                                "客户端已退出，继续监听 {}",
                                listener.local_addr().map_err(|e| e.to_string())?
                            ),
                        );
                        return Ok(true);
                    }
                    Ok(count) => cx.log(false, &bytes[..count], "TCP 接收".into()),
                    Err(e) if retryable(&e) => {}
                    Err(e) => {
                        *peer = None;
                        cx.change_peer()?;
                        cx.state(State::Listening, format!("客户端断开：{e}；继续监听"));
                        return Ok(true);
                    }
                }
            }
        }
        Connection::Tcp(stream) => match stream.read(&mut bytes[..4096]) {
            Ok(0) => return Err("对端已关闭 TCP 连接".into()),
            Ok(count) => cx.log(false, &bytes[..count], "TCP 接收".into()),
            Err(e) if retryable(&e) => {}
            Err(e) => return Err(format!("TCP 接收失败：{e}")),
        },
        Connection::Serial(port) => match port.read(&mut bytes[..4096]) {
            Ok(0) => {}
            Ok(count) => cx.log(false, &bytes[..count], "串口接收".into()),
            Err(e) if retryable(&e) => {}
            Err(e) => return Err(format!("串口接收失败：{e}")),
        },
        Connection::Udp(socket, _) => match socket.recv_from(&mut bytes) {
            Ok((count, peer)) => cx.log(false, &bytes[..count], format!("UDP 来自 {peer}")),
            Err(e) if retryable(&e) => {}
            Err(e) => return Err(format!("UDP 接收失败：{e}")),
        },
        Connection::S7(_) => {}
    }
    Ok(false)
}

fn execute(
    connection: &mut Connection,
    config: &Config,
    operation: Operation,
    transaction: &mut u16,
    cx: &Context,
) -> Result<Option<Reply>, String> {
    let deadline = Instant::now() + Duration::from_millis(config.timeout_ms);
    match operation {
        Operation::Raw(bytes) => {
            if !config.mode.is_raw() {
                return Err("当前连接不支持原始发送".into());
            }
            if bytes.is_empty() || bytes.len() > 65_507 {
                return Err("每次发送应为 1～65507 字节".into());
            }
            match connection {
                Connection::Tcp(stream) => write_until(stream, &bytes, deadline, &cx.stop, false)?,
                Connection::Server(_, peer) => {
                    let result = match peer {
                        Some(stream) => write_until(stream, &bytes, deadline, &cx.stop, false),
                        None => Err("没有已连接的客户端".into()),
                    };
                    if let Err(error) = result {
                        if peer.take().is_some() {
                            cx.change_peer()?;
                        }
                        if !cx.stopped() {
                            let _ = cx.events.try_send(Event::Error(error));
                            cx.state(State::Listening, "发送未完成，继续监听并等待客户端".into());
                        }
                        return Ok(None);
                    }
                }
                Connection::Serial(port) => {
                    write_until(port.as_mut(), &bytes, deadline, &cx.stop, true)?
                }
                Connection::Udp(socket, target) => {
                    let count = socket
                        .send_to(&bytes, *target)
                        .map_err(|e| format!("UDP 发送失败：{e}"))?;
                    if count != bytes.len() {
                        return Err("UDP 数据报发送不完整".into());
                    }
                }
                _ => return Err("没有已连接的客户端".into()),
            }
            cx.log(true, &bytes, "发送".into());
            Ok(None)
        }
        Operation::Modbus(request) => {
            *transaction = transaction.wrapping_add(1);
            let reply = match (config.mode, connection) {
                (Mode::ModbusTcp, Connection::Tcp(stream)) => {
                    let sent = request.encode_tcp(*transaction);
                    write_until(stream, &sent, deadline, &cx.stop, false)?;
                    cx.log(true, &sent, "Modbus TCP 请求".into());
                    let frame = read_modbus_tcp(stream, deadline, &cx.stop)?;
                    cx.log(false, &frame, "Modbus TCP 响应".into());
                    request.decode_tcp(*transaction, &frame)?
                }
                (Mode::ModbusRtu, Connection::Serial(port)) => {
                    if !(1..=247).contains(&request.unit()) {
                        return Err("RTU 站号应为 1～247，不支持广播读写".into());
                    }
                    wait_rtu_quiet(port.as_mut(), rtu_silence(config), deadline, &cx.stop)?;
                    let sent = request.encode_rtu();
                    write_until(port.as_mut(), &sent, deadline, &cx.stop, true)?;
                    cx.log(true, &sent, "Modbus RTU 请求".into());
                    let frame = read_modbus_rtu(port.as_mut(), deadline, &cx.stop)?;
                    cx.log(false, &frame, "Modbus RTU 响应".into());
                    request.decode_rtu(&frame)?
                }
                _ => return Err("当前连接不是 Modbus 客户端".into()),
            };
            Ok(Some(match reply {
                modbus::Response::Bits(bits) => Reply::Bits(bits),
                modbus::Response::Registers(values) => {
                    Reply::Bytes(values.iter().flat_map(|v| v.to_be_bytes()).collect())
                }
                modbus::Response::Written(count) => Reply::Written(count),
            }))
        }
        Operation::S7Read {
            area,
            db,
            offset,
            length,
            bit,
        } => {
            let Connection::S7(client) = connection else {
                return Err("当前连接不是 S7 客户端".into());
            };
            let reply = if let Some(bit) = bit {
                let value = client.read_bit(area, db, offset, bit)?;
                cx.log(
                    false,
                    &[u8::from(value)],
                    format!(
                        "S7 {} DB{db} {offset}.{bit} 位值（非完整报文）",
                        area.label()
                    ),
                );
                Reply::Bits(vec![value])
            } else {
                let bytes = client.read_bytes(area, db, offset, length)?;
                cx.log(
                    false,
                    &bytes,
                    format!(
                        "S7 {} DB{db} 偏移{offset} 有效载荷（非完整报文）",
                        area.label()
                    ),
                );
                Reply::Bytes(bytes)
            };
            Ok(Some(reply))
        }
        Operation::S7Write {
            area,
            db,
            offset,
            data,
            bit,
        } => {
            let Connection::S7(client) = connection else {
                return Err("当前连接不是 S7 客户端".into());
            };
            if let Some(bit) = bit {
                if data.len() != 1 || data[0] > 1 {
                    return Err("S7 位写入只接受一个 0 或 1".into());
                }
                client.write_bit(area, db, offset, bit, data[0] != 0)?;
            } else {
                client.write_bytes(area, db, offset, &data)?;
            }
            cx.log(
                true,
                &data,
                format!(
                    "S7 {} DB{db} 偏移{offset}{} 写入已确认（有效载荷）",
                    area.label(),
                    bit.map_or(String::new(), |b| format!(".{b}"))
                ),
            );
            Ok(Some(Reply::Written(data.len() as u16)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, UdpSocket};

    fn connected(worker: &Worker) {
        loop {
            match worker.events.recv_timeout(Duration::from_secs(3)).unwrap() {
                Event::State(State::Connected, _) => return,
                Event::Peer(peer) => worker.acknowledge_peer(peer),
                Event::Error(e) => panic!("{e}"),
                _ => {}
            }
        }
    }

    fn wait_logs(worker: &Worker) -> Vec<Log> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let logs = worker.take_logs();
            if !logs.is_empty() {
                return logs;
            }
            assert!(Instant::now() < deadline, "等待后台日志超时");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn tcp_client_sends_binary_and_receives_without_ui() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut input = [0; 3];
            stream.read_exact(&mut input).unwrap();
            assert_eq!(input, [0, 0x80, 0xff]);
            stream.write_all(&input).unwrap();
        });
        let worker = Worker::start(Config {
            remote_port: endpoint.port(),
            ..Default::default()
        })
        .unwrap();
        connected(&worker);
        worker.execute(Operation::Raw(vec![0, 0x80, 0xff])).unwrap();
        let mut received = Vec::new();
        while received.len() < 3 {
            for log in wait_logs(&worker) {
                if !log.sent {
                    received.extend(log.bytes);
                }
            }
        }
        assert_eq!(received, [0, 0x80, 0xff]);
        worker.stop();
        drop(worker);
        server.join().unwrap();
    }

    #[test]
    fn udp_preserves_binary_datagram() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let worker = Worker::start(Config {
            mode: Mode::Udp,
            remote_port: server.local_addr().unwrap().port(),
            ..Default::default()
        })
        .unwrap();
        connected(&worker);
        worker.execute(Operation::Raw(vec![1, 0, 2])).unwrap();
        let mut bytes = [0; 8];
        let (len, peer) = server.recv_from(&mut bytes).unwrap();
        assert_eq!(&bytes[..len], &[1, 0, 2]);
        server.send_to(&[9, 8], peer).unwrap();
        loop {
            if let Some(log) = wait_logs(&worker).into_iter().find(|log| !log.sent) {
                assert_eq!(log.bytes, [9, 8]);
                break;
            }
        }
    }

    #[test]
    fn rejects_invalid_addresses_before_starting_thread() {
        let config = Config {
            remote_ip: "not-an-ip".into(),
            ..Default::default()
        };
        assert!(Worker::start(config).err().unwrap().contains("IP"));
    }

    #[test]
    fn server_accepts_disconnects_and_releases_listener() {
        let worker = Worker::start(Config {
            mode: Mode::TcpServer,
            local_ip: "127.0.0.1".into(),
            ..Default::default()
        })
        .unwrap();
        let endpoint = loop {
            if let Event::State(State::Listening, detail) =
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap()
            {
                break detail
                    .trim_start_matches("正在监听 ")
                    .split('，')
                    .next()
                    .unwrap()
                    .parse::<SocketAddr>()
                    .unwrap();
            }
        };
        let mut peer = TcpStream::connect(endpoint).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        connected(&worker);
        worker.execute(Operation::Raw(vec![7, 8])).unwrap();
        let mut input = [0; 2];
        peer.read_exact(&mut input).unwrap();
        assert_eq!(input, [7, 8]);
        drop(peer);
        loop {
            if matches!(
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap(),
                Event::State(State::Listening, _)
            ) {
                break;
            }
        }
        let second = TcpStream::connect(endpoint).unwrap();
        connected(&worker);
        worker.stop();
        drop(worker);
        drop(second);
        assert!(TcpListener::bind(endpoint).is_ok());
    }

    #[test]
    fn periodic_udp_stops_and_rejects_periodic_plc_writes() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let worker = Worker::start(Config {
            mode: Mode::Udp,
            remote_port: server.local_addr().unwrap().port(),
            ..Default::default()
        })
        .unwrap();
        connected(&worker);
        let write = modbus::Request::write_registers(1, 0, &[42]).unwrap();
        assert!(
            worker
                .repeat(Operation::Modbus(write), Duration::from_millis(100))
                .is_err()
        );
        worker
            .repeat(Operation::Raw(vec![99]), Duration::from_millis(100))
            .unwrap();
        let mut input = [0; 8];
        for _ in 0..2 {
            assert_eq!(server.recv_from(&mut input).unwrap().0, 1);
        }
        worker.stop_repeat().unwrap();
        loop {
            if matches!(
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap(),
                Event::RepeatStopped
            ) {
                break;
            }
        }
        // 清除停止指令执行前已经进入操作系统队列的数据报。
        server.set_nonblocking(true).unwrap();
        while server.recv_from(&mut input).is_ok() {}
        server.set_nonblocking(false).unwrap();
        assert!(server.recv_from(&mut input).is_err());
    }

    #[test]
    fn modbus_tcp_handles_fragmented_response_and_closes_on_mismatch() {
        for wrong_transaction in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 12];
                stream.read_exact(&mut request).unwrap();
                assert_eq!(&request[6..], &[1, 3, 0, 0, 0, 2]);
                let mut response = vec![
                    request[0], request[1], 0, 0, 0, 7, 1, 3, 4, 0x12, 0x34, 0xab, 0xcd,
                ];
                if wrong_transaction {
                    response[1] = response[1].wrapping_add(1);
                }
                for byte in response {
                    stream.write_all(&[byte]).unwrap();
                }
                let mut extra = [0; 1];
                assert_eq!(
                    stream.read(&mut extra).unwrap(),
                    0,
                    "错误或停止后应关闭连接，不能自动重发"
                );
            });
            let worker = Worker::start(Config {
                mode: Mode::ModbusTcp,
                remote_port: endpoint.port(),
                ..Default::default()
            })
            .unwrap();
            connected(&worker);
            worker
                .execute(Operation::Modbus(
                    modbus::Request::read(1, modbus::Area::HoldingRegisters, 0, 2).unwrap(),
                ))
                .unwrap();
            loop {
                match worker.events.recv_timeout(Duration::from_secs(2)).unwrap() {
                    Event::Result(Reply::Bytes(bytes)) => {
                        assert!(!wrong_transaction);
                        assert_eq!(bytes, [0x12, 0x34, 0xab, 0xcd]);
                        break;
                    }
                    Event::Error(message) => {
                        assert!(wrong_transaction);
                        assert!(message.contains("事务号"));
                        break;
                    }
                    _ => {}
                }
            }
            worker.stop();
            drop(worker);
            server.join().unwrap();
        }
    }

    struct Fragmented {
        bytes: std::io::Cursor<Vec<u8>>,
        timeout_next: bool,
    }
    impl Read for Fragmented {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            self.timeout_next = !self.timeout_next;
            if self.timeout_next {
                return Err(io::ErrorKind::TimedOut.into());
            }
            self.bytes.read(&mut output[..1])
        }
    }

    #[test]
    fn rtu_stream_accepts_fragmentation_and_rejects_truncation() {
        let request = modbus::Request::read(1, modbus::Area::HoldingRegisters, 0, 1).unwrap();
        let mut bytes = vec![1, 3, 2, 0x12, 0x34];
        bytes.extend_from_slice(&modbus::crc16(&bytes).to_le_bytes());
        let mut stream = Fragmented {
            bytes: std::io::Cursor::new(bytes.clone()),
            timeout_next: false,
        };
        let stop = AtomicBool::new(false);
        let frame =
            read_modbus_rtu(&mut stream, Instant::now() + Duration::from_secs(1), &stop).unwrap();
        assert_eq!(
            request.decode_rtu(&frame).unwrap(),
            modbus::Response::Registers(vec![0x1234])
        );
        bytes.pop();
        assert!(
            read_modbus_rtu(
                &mut std::io::Cursor::new(bytes),
                Instant::now() + Duration::from_secs(1),
                &stop
            )
            .unwrap_err()
            .contains("中断")
        );
        stop.store(true, Ordering::Release);
        assert!(
            read_modbus_rtu(
                &mut std::io::Cursor::new(vec![0; 8]),
                Instant::now() + Duration::from_secs(1),
                &stop
            )
            .unwrap_err()
            .contains("取消")
        );
    }

    #[test]
    fn serial_style_short_writes_complete_without_resending_prefix() {
        struct ShortWriter(Vec<u8>);
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.push(bytes[0]);
                Ok(1)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut stream = ShortWriter(Vec::new());
        write_until(
            &mut stream,
            &[1, 2, 3, 4],
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false),
            true,
        )
        .unwrap();
        assert_eq!(stream.0, [1, 2, 3, 4]);
    }

    #[test]
    fn log_queue_is_bounded_and_keeps_total_byte_counts() {
        let (events, _) = mpsc::sync_channel(1);
        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let cx = Context {
            events,
            logs: logs.clone(),
            actual_peer: Cell::new(0),
            stop: Arc::new(AtomicBool::new(false)),
            tx: Arc::new(AtomicU64::new(0)),
            rx: Arc::new(AtomicU64::new(0)),
            dropped: Arc::new(AtomicU64::new(0)),
        };
        for index in 0..130 {
            cx.log(false, &[index], "接收".into());
        }
        let retained = logs.lock().unwrap().drain(..).collect::<Vec<_>>();
        assert_eq!(retained.len(), 128);
        assert_eq!(retained.first().unwrap().bytes, [2]);
        assert_eq!(retained.last().unwrap().bytes, [129]);
        assert_eq!(cx.rx.load(Ordering::Relaxed), 130);
        assert_eq!(cx.dropped.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn undeliverable_connection_state_stops_sending() {
        let (cx, receiver) = test_context();
        for _ in 0..16 {
            cx.events.try_send(Event::RepeatStopped).unwrap();
        }
        cx.state(State::Connected, "新客户端".into());
        assert!(
            cx.stopped(),
            "不能投递连接状态时必须停止通信，避免界面仍显示旧客户端"
        );
        assert_eq!(receiver.try_iter().count(), 16);
    }

    #[test]
    fn server_rejects_commands_from_previous_displayed_peer() {
        let worker = Worker::start(Config {
            mode: Mode::TcpServer,
            local_ip: "127.0.0.1".into(),
            ..Default::default()
        })
        .unwrap();
        let endpoint = loop {
            if let Event::State(State::Listening, detail) =
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap()
            {
                break detail
                    .trim_start_matches("正在监听 ")
                    .split('，')
                    .next()
                    .unwrap()
                    .parse::<SocketAddr>()
                    .unwrap();
            }
        };
        let first = TcpStream::connect(endpoint).unwrap();
        connected(&worker);
        drop(first);
        loop {
            if matches!(
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap(),
                Event::State(State::Listening, _)
            ) {
                break;
            }
        }
        let mut second = TcpStream::connect(endpoint).unwrap();
        second
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        // 后台已换客户端，但模拟 GUI 尚未确认新的客户端身份。
        let mut next_peer = None;
        loop {
            match worker.events.recv_timeout(Duration::from_secs(2)).unwrap() {
                Event::Peer(peer) => next_peer = Some(peer),
                Event::State(State::Connected, _) => break,
                _ => {}
            }
        }
        worker.execute(Operation::Raw(vec![0x11])).unwrap();
        worker
            .repeat(Operation::Raw(vec![0x22]), Duration::from_millis(100))
            .unwrap();
        let mut input = [0; 1];
        assert!(
            matches!(second.read(&mut input), Err(error) if retryable(&error)),
            "新客户端不得收到旧客户端界面产生的单次或周期发送"
        );
        assert!(!worker.is_finished(), "淘汰旧命令不应关闭监听或新客户端");
        worker.acknowledge_peer(next_peer.expect("新客户端状态前必须送达身份"));
        worker.execute(Operation::Raw(vec![0x33])).unwrap();
        second.read_exact(&mut input).unwrap();
        assert_eq!(input, [0x33]);
        worker.stop();
    }

    fn test_context() -> (Context, mpsc::Receiver<Event>) {
        let (events, receiver) = mpsc::sync_channel(16);
        (
            Context {
                events,
                logs: Arc::new(Mutex::new(VecDeque::new())),
                actual_peer: Cell::new(0),
                stop: Arc::new(AtomicBool::new(false)),
                tx: Arc::new(AtomicU64::new(0)),
                rx: Arc::new(AtomicU64::new(0)),
                dropped: Arc::new(AtomicU64::new(0)),
            },
            receiver,
        )
    }

    #[test]
    fn server_send_without_peer_keeps_listener_and_stops_repeat() {
        let worker = Worker::start(Config {
            mode: Mode::TcpServer,
            local_ip: "127.0.0.1".into(),
            ..Default::default()
        })
        .unwrap();
        let endpoint = loop {
            if let Event::State(State::Listening, detail) =
                worker.events.recv_timeout(Duration::from_secs(2)).unwrap()
            {
                break detail
                    .trim_start_matches("正在监听 ")
                    .split('，')
                    .next()
                    .unwrap()
                    .parse::<SocketAddr>()
                    .unwrap();
            }
        };
        worker
            .repeat(Operation::Raw(vec![42]), Duration::from_millis(100))
            .unwrap();
        let mut saw_error = false;
        loop {
            match worker.events.recv_timeout(Duration::from_secs(2)).unwrap() {
                Event::Error(_) => saw_error = true,
                Event::RepeatStopped => break,
                Event::State(State::Closed, _) => panic!("未接入客户端不应关闭监听"),
                _ => {}
            }
        }
        assert!(saw_error);
        assert!(!worker.is_finished());
        let mut peer = TcpStream::connect(endpoint).unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        connected(&worker);
        let mut input = [0; 1];
        assert!(
            peer.read(&mut input).is_err(),
            "新客户端不应继承已停止的周期发送"
        );
        worker.execute(Operation::Raw(vec![7])).unwrap();
        peer.read_exact(&mut input).unwrap();
        assert_eq!(input, [7]);
        worker.stop();
    }

    #[test]
    fn server_write_failure_discards_peer_and_accepts_a_new_one() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let old_peer = TcpStream::connect(endpoint).unwrap();
        let (stream, _) = listener.accept().unwrap();
        prepare_tcp(&stream).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut connection = Connection::Server(listener, Some(stream));
        let (cx, events) = test_context();
        let config = Config {
            mode: Mode::TcpServer,
            ..Default::default()
        };
        assert!(
            execute(
                &mut connection,
                &config,
                Operation::Raw(vec![1]),
                &mut 0,
                &cx
            )
            .is_ok()
        );
        assert!(matches!(connection, Connection::Server(_, None)));
        assert!(
            events
                .try_iter()
                .any(|event| matches!(event, Event::State(State::Listening, _)))
        );
        let mut new_peer = TcpStream::connect(endpoint).unwrap();
        new_peer
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        poll_raw(&mut connection, &cx).unwrap();
        execute(
            &mut connection,
            &config,
            Operation::Raw(vec![2]),
            &mut 0,
            &cx,
        )
        .unwrap();
        let mut input = [0; 1];
        new_peer.read_exact(&mut input).unwrap();
        assert_eq!(input, [2]);
        drop(old_peer);
    }

    struct PausedWriter {
        zero_writes: usize,
        bytes: Vec<u8>,
    }
    impl Write for PausedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.zero_writes > 0 {
                self.zero_writes -= 1;
                return Ok(0);
            }
            self.bytes.push(bytes[0]);
            Ok(1)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn serial_zero_progress_retries_without_resending_written_bytes() {
        let mut writer = PausedWriter {
            zero_writes: 2,
            bytes: Vec::new(),
        };
        write_until(
            &mut writer,
            &[1, 2, 3],
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false),
            true,
        )
        .unwrap();
        assert_eq!(writer.bytes, [1, 2, 3]);
    }

    #[test]
    fn serial_zero_progress_obeys_deadline_and_cancellation() {
        let mut writer = PausedWriter {
            zero_writes: usize::MAX,
            bytes: Vec::new(),
        };
        let stop = AtomicBool::new(false);
        let error = write_until(
            &mut writer,
            &[1],
            Instant::now() + Duration::from_millis(25),
            &stop,
            true,
        )
        .unwrap_err();
        assert!(error.contains("超时"), "{error}");
        stop.store(true, Ordering::Release);
        assert!(
            write_until(
                &mut writer,
                &[1],
                Instant::now() + Duration::from_secs(1),
                &stop,
                true
            )
            .unwrap_err()
            .contains("取消")
        );
        assert!(writer.bytes.is_empty());
    }

    #[test]
    fn tcp_zero_progress_remains_an_immediate_write_error() {
        let mut writer = PausedWriter {
            zero_writes: 1,
            bytes: Vec::new(),
        };
        assert!(
            write_until(
                &mut writer,
                &[1],
                Instant::now() + Duration::from_secs(1),
                &AtomicBool::new(false),
                false
            )
            .unwrap_err()
            .contains("无法继续发送")
        );
        assert!(writer.bytes.is_empty());
    }

    #[test]
    fn rtu_silence_uses_actual_character_bits_and_high_baud_fixed_time() {
        let mut config = Config {
            baud_rate: 9600,
            data_bits: 8,
            parity: 0,
            stop_bits: 1,
            ..Default::default()
        };
        assert_eq!(rtu_silence(&config), Duration::from_secs_f64(35.0 / 9600.0));
        config.parity = 2;
        assert_eq!(rtu_silence(&config), Duration::from_secs_f64(38.5 / 9600.0));
        config.stop_bits = 2;
        assert_eq!(rtu_silence(&config), Duration::from_secs_f64(42.0 / 9600.0));
        config.baud_rate = 19200;
        assert_eq!(
            rtu_silence(&config),
            Duration::from_secs_f64(42.0 / 19200.0)
        );
        config.baud_rate = 38400;
        assert_eq!(rtu_silence(&config), Duration::from_micros(1750));
    }

    struct DelayedResidual {
        started: Instant,
        delay: Duration,
        frame: Vec<u8>,
        delivered: bool,
        last_delivery: Option<Instant>,
    }
    impl Read for DelayedResidual {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if !self.delivered && self.started.elapsed() >= self.delay {
                output[..self.frame.len()].copy_from_slice(&self.frame);
                self.delivered = true;
                self.last_delivery = Some(Instant::now());
                return Ok(self.frame.len());
            }
            std::thread::sleep(Duration::from_millis(1));
            Err(io::ErrorKind::TimedOut.into())
        }
    }

    #[test]
    fn rtu_quiet_drains_delayed_old_frame_and_restarts_silence() {
        let mut stream = DelayedResidual {
            started: Instant::now(),
            delay: Duration::from_millis(15),
            frame: vec![1, 3, 2, 0, 10, 0x38, 0x43],
            delivered: false,
            last_delivery: None,
        };
        let silence = Duration::from_millis(30);
        wait_rtu_quiet(
            &mut stream,
            silence,
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(stream.delivered, "静默等待期间到达的旧响应必须被排空");
        assert!(stream.last_delivery.unwrap().elapsed() >= silence);
    }

    #[test]
    fn rtu_quiet_obeys_deadline_even_at_very_low_baud() {
        let config = Config {
            baud_rate: 1,
            ..Default::default()
        };
        let started = Instant::now();
        let error = wait_rtu_quiet(
            &mut io::empty(),
            rtu_silence(&config),
            started + Duration::from_millis(25),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(error.contains("超时"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn rtu_quiet_can_be_cancelled_while_waiting() {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let cancel = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(15));
            thread_stop.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let error = wait_rtu_quiet(
            &mut io::empty(),
            Duration::from_secs(30),
            started + Duration::from_secs(1),
            &stop,
        )
        .unwrap_err();
        cancel.join().unwrap();
        assert!(error.contains("取消"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn rtu_quiet_continuous_input_cannot_extend_total_deadline() {
        let started = Instant::now();
        let error = wait_rtu_quiet(
            &mut io::repeat(0x55),
            Duration::from_millis(5),
            started + Duration::from_millis(25),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(error.contains("超时"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
