//! 经典 S7 绝对地址通信：DB、M、I、Q 区的字节及单个位读写。
//!
//! 仅使用 Rust 标准库；每次操作最多 200 字节，不自动重试写入。
//! 使用 RFC 1006 TPKT、COTP Class 0 和经典 S7 Read/Write Var 报文。
//! 仅支持绝对地址；优化 DB、S7CommPlus 和 COTP 分段重组不在范围内。

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const REQUESTED_PDU: usize = 480;
const MAX_TRANSFER: usize = 200;
const MAX_BYTE_OFFSET: u32 = 0x1f_ffff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Inputs,
    Outputs,
    Markers,
    DataBlock,
}

impl Area {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inputs => "I 输入区",
            Self::Outputs => "Q 输出区",
            Self::Markers => "M 标志区",
            Self::DataBlock => "DB 数据块",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Inputs => 0x81,
            Self::Outputs => 0x82,
            Self::Markers => 0x83,
            Self::DataBlock => 0x84,
        }
    }
}

pub struct Config {
    pub endpoint: SocketAddr,
    pub local_tsap: u16,
    pub remote_tsap: u16,
    pub timeout: Duration,
}

pub struct Client {
    stream: Option<TcpStream>,
    timeout: Duration,
    pdu_length: usize,
    reference: u16,
}

impl Client {
    pub fn connect(config: Config) -> Result<Self, String> {
        if config.endpoint.port() == 0 || config.timeout.is_zero() {
            return Err("S7 端口和超时时间必须大于零".into());
        }
        // TCP 连接、COTP 确认和 S7 协商共用一次连接截止时间。
        let deadline = new_deadline(config.timeout)?;
        let mut stream = TcpStream::connect_timeout(&config.endpoint, config.timeout)
            .map_err(|e| network_error("S7 TCP 连接", &e))?;
        stream
            .set_nodelay(true)
            .map_err(|e| network_error("设置 S7 连接", &e))?;
        let mut cotp = vec![3, 0, 0, 22, 17, 0xe0, 0, 0, 0, 1, 0, 0xc0, 1, 10, 0xc1, 2];
        cotp.extend_from_slice(&config.local_tsap.to_be_bytes());
        cotp.extend_from_slice(&[0xc2, 2]);
        cotp.extend_from_slice(&config.remote_tsap.to_be_bytes());
        write_before(&mut stream, &cotp, deadline)?;
        validate_cotp_confirm(&read_tpkt(&mut stream, deadline)?)?;

        let mut client = Self {
            stream: Some(stream),
            timeout: config.timeout,
            pdu_length: REQUESTED_PDU,
            reference: 0,
        };
        let (parameters, data) = client.exchange(&[0xf0, 0, 0, 1, 0, 1, 1, 0xe0], &[], deadline)?;
        if parameters.len() != 8
            || parameters[..2] != [0xf0, 0]
            || !data.is_empty()
            || u16::from_be_bytes([parameters[2], parameters[3]]) == 0
            || u16::from_be_bytes([parameters[4], parameters[5]]) == 0
        {
            return Err("S7 Setup Communication 协商响应无效".into());
        }
        let pdu = u16::from_be_bytes([parameters[6], parameters[7]]) as usize;
        // 至少容纳单个字节写入；不接受超过本机请求值的 PDU。
        if !(32..=REQUESTED_PDU).contains(&pdu) {
            return Err(format!("S7 协商 PDU 长度无效：{pdu}"));
        }
        client.pdu_length = pdu;
        Ok(client)
    }

    pub fn read_bytes(
        &mut self,
        area: Area,
        db: u16,
        offset: u32,
        len: u16,
    ) -> Result<Vec<u8>, String> {
        self.read(area, db, offset, len as usize, None)
    }

    pub fn write_bytes(
        &mut self,
        area: Area,
        db: u16,
        offset: u32,
        data: &[u8],
    ) -> Result<(), String> {
        self.write(area, db, offset, data, None)
    }

    pub fn read_bit(&mut self, area: Area, db: u16, offset: u32, bit: u8) -> Result<bool, String> {
        Ok(self.read(area, db, offset, 1, Some(bit))?[0] != 0)
    }

    pub fn write_bit(
        &mut self,
        area: Area,
        db: u16,
        offset: u32,
        bit: u8,
        value: bool,
    ) -> Result<(), String> {
        // S7 的原子位写，不执行可能覆盖相邻位的字节读改写。
        self.write(area, db, offset, &[u8::from(value)], Some(bit))
    }

    pub fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    fn read(
        &mut self,
        area: Area,
        db: u16,
        offset: u32,
        len: usize,
        bit: Option<u8>,
    ) -> Result<Vec<u8>, String> {
        self.validate_length(len, 18)?;
        let parameters = variable_parameters(4, area, db, offset, len, bit)?;
        let (parameters, data) = self.exchange(&parameters, &[], new_deadline(self.timeout)?)?;
        if parameters != [4, 1] || data.len() < 4 {
            return self.invalid_response("S7 读取响应的功能码、项目数或长度无效");
        }
        if data[0] != 0xff {
            return Err(item_error(data[0]));
        }
        let expected_bits = if bit.is_some() { 1 } else { len as u16 * 8 };
        if data[1] != if bit.is_some() { 3 } else { 4 }
            || u16::from_be_bytes([data[2], data[3]]) != expected_bits
            || data.len() != 4 + len
        {
            return self.invalid_response("S7 读取响应的数据类型或数据长度不匹配");
        }
        Ok(data[4..].to_vec())
    }

    fn write(
        &mut self,
        area: Area,
        db: u16,
        offset: u32,
        data: &[u8],
        bit: Option<u8>,
    ) -> Result<(), String> {
        self.validate_length(data.len(), 28)?;
        let parameters = variable_parameters(5, area, db, offset, data.len(), bit)?;
        let bits = if bit.is_some() {
            1
        } else {
            data.len() as u16 * 8
        };
        let mut item = vec![0, if bit.is_some() { 3 } else { 4 }];
        item.extend_from_slice(&bits.to_be_bytes());
        item.extend_from_slice(data);
        let (parameters, response) =
            self.exchange(&parameters, &item, new_deadline(self.timeout)?)?;
        if parameters != [5, 1] || response.len() != 1 {
            return self.invalid_response("S7 写入响应的功能码、项目数或长度无效");
        }
        if response[0] != 0xff {
            return Err(item_error(response[0]));
        }
        Ok(())
    }

    fn validate_length(&self, length: usize, overhead: usize) -> Result<(), String> {
        if self.stream.is_none() {
            return Err("S7 未连接".into());
        }
        let limit = MAX_TRANSFER.min(self.pdu_length.saturating_sub(overhead));
        if length == 0 || length > limit {
            return Err(format!(
                "S7 单次操作长度必须为 1～{limit} 字节（受协商 PDU 限制，不支持自动拆分）"
            ));
        }
        Ok(())
    }

    fn invalid_response<T>(&mut self, message: &str) -> Result<T, String> {
        self.disconnect();
        Err(message.into())
    }

    fn exchange(
        &mut self,
        parameters: &[u8],
        data: &[u8],
        deadline: Instant,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.reference = self.reference.wrapping_add(1).max(1);
        let reference = self.reference;
        let mut request = vec![3, 0, 0, 0, 2, 0xf0, 0x80, 0x32, 1, 0, 0];
        request.extend_from_slice(&reference.to_be_bytes());
        request.extend_from_slice(&(parameters.len() as u16).to_be_bytes());
        request.extend_from_slice(&(data.len() as u16).to_be_bytes());
        request.extend_from_slice(parameters);
        request.extend_from_slice(data);
        let length = request.len() as u16;
        request[2..4].copy_from_slice(&length.to_be_bytes());
        let result = (|| {
            let stream = self.stream.as_mut().ok_or("S7 未连接")?;
            write_before(stream, &request, deadline)?;
            let response = read_tpkt(stream, deadline)?;
            if response.len() - 7 > self.pdu_length {
                return Err("S7 响应超过协商 PDU 长度".into());
            }
            let (parameters, data) = parse_ack(&response, reference)?;
            Ok((parameters.to_vec(), data.to_vec()))
        })();
        // 超时或帧校验失败后，旧响应可能仍在途中，必须关闭连接避免串包。
        if result.is_err() {
            self.disconnect();
        }
        result
    }
}

fn variable_parameters(
    function: u8,
    area: Area,
    db: u16,
    offset: u32,
    len: usize,
    bit: Option<u8>,
) -> Result<Vec<u8>, String> {
    if area == Area::DataBlock && db == 0 {
        return Err("S7 DB 编号必须为 1～65535".into());
    }
    if bit.is_some_and(|v| v > 7) {
        return Err("S7 位索引必须为 0～7".into());
    }
    if offset > MAX_BYTE_OFFSET || offset.saturating_add(len as u32 - 1) > MAX_BYTE_OFFSET {
        return Err("S7 地址超出 24 位位地址范围".into());
    }
    let address = offset * 8 + u32::from(bit.unwrap_or(0));
    let mut parameters = vec![
        function,
        1,
        0x12,
        0x0a,
        0x10,
        if bit.is_some() { 1 } else { 2 },
    ];
    parameters.extend_from_slice(&(len as u16).to_be_bytes());
    parameters.extend_from_slice(&(if area == Area::DataBlock { db } else { 0 }).to_be_bytes());
    parameters.push(area.code());
    parameters.extend_from_slice(&address.to_be_bytes()[1..]);
    Ok(parameters)
}

fn validate_cotp_confirm(frame: &[u8]) -> Result<(), String> {
    if frame.len() < 11
        || frame[4] as usize + 5 != frame.len()
        || frame[5] != 0xd0
        || frame[6..8] != [0, 1]
        || frame[10] != 0
    {
        return Err("COTP 连接确认的类型、引用号或长度无效".into());
    }
    let mut cursor = 11;
    while cursor < frame.len() {
        if cursor + 2 > frame.len() {
            return Err("COTP 连接参数不完整".into());
        }
        let tag = frame[cursor];
        let length = frame[cursor + 1] as usize;
        if cursor + 2 + length > frame.len()
            || (tag == 0xc0 && length != 1)
            || ((tag == 0xc1 || tag == 0xc2) && length != 2)
        {
            return Err("COTP 连接参数长度无效".into());
        }
        cursor += 2 + length;
    }
    Ok(())
}

fn parse_ack(frame: &[u8], reference: u16) -> Result<(&[u8], &[u8]), String> {
    if frame.len() < 19 || frame[4..11] != [2, 0xf0, 0x80, 0x32, 3, 0, 0] {
        return Err("S7 响应头无效或包含不支持的 COTP 分段".into());
    }
    if u16::from_be_bytes([frame[11], frame[12]]) != reference {
        return Err("S7 响应关联号不匹配".into());
    }
    let parameters = u16::from_be_bytes([frame[13], frame[14]]) as usize;
    let data = u16::from_be_bytes([frame[15], frame[16]]) as usize;
    if frame.len() != 19 + parameters + data {
        return Err("S7 响应参数或数据长度无效".into());
    }
    if frame[17] != 0 || frame[18] != 0 {
        return Err(format!(
            "PLC 拒绝 S7 请求（错误码 {:02X}{:02X}）",
            frame[17], frame[18]
        ));
    }
    Ok((&frame[19..19 + parameters], &frame[19 + parameters..]))
}

fn item_error(code: u8) -> String {
    let reason = match code {
        1 => "硬件故障",
        3 => "访问被拒绝",
        5 => "地址无效或超出范围",
        6 => "数据类型不支持",
        7 => "数据类型不一致",
        10 => "目标对象不存在",
        _ => "PLC 拒绝数据访问",
    };
    format!("S7 {reason}（返回码 0x{code:02X}）")
}

fn new_deadline(timeout: Duration) -> Result<Instant, String> {
    Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| "S7 超时时间过大".into())
}

fn remaining(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| "S7 操作超时".into())
}

fn network_error(operation: &str, error: &io::Error) -> String {
    let reason = match error.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "超时",
        io::ErrorKind::ConnectionRefused => "被对端拒绝",
        io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted => "连接已中断",
        io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof | io::ErrorKind::NotConnected => {
            "连接已关闭"
        }
        io::ErrorKind::AddrNotAvailable => "地址不可用",
        io::ErrorKind::PermissionDenied => "权限不足",
        _ => "网络操作失败",
    };
    match error.raw_os_error() {
        Some(code) => format!("{operation}{reason}（系统错误码 {code}）"),
        None => format!("{operation}{reason}"),
    }
}

fn write_before(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> Result<(), String> {
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(|e| network_error("设置发送超时", &e))?;
        match stream.write(bytes) {
            Ok(0) => return Err("S7 发送时连接已关闭".into()),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(network_error("S7 发送", &e)),
        }
    }
    remaining(deadline)?;
    Ok(())
}

fn read_before(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(|e| network_error("设置接收超时", &e))?;
        match stream.read(bytes) {
            Ok(0) => return Err("S7 连接已被对端关闭".into()),
            Ok(n) => bytes = &mut bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(network_error("S7 接收", &e)),
        }
    }
    remaining(deadline)?;
    Ok(())
}

fn read_tpkt(stream: &mut TcpStream, deadline: Instant) -> Result<Vec<u8>, String> {
    let mut header = [0; 4];
    read_before(stream, &mut header, deadline)?;
    let length = u16::from_be_bytes([header[2], header[3]]) as usize;
    if header[..2] != [3, 0] || !(7..=REQUESTED_PDU + 7).contains(&length) {
        return Err("TPKT 版本或报文长度无效".into());
    }
    let mut frame = vec![0; length];
    frame[..4].copy_from_slice(&header);
    read_before(stream, &mut frame[4..], deadline)?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};
    use std::time::Instant;

    fn receive(stream: &mut TcpStream) -> Vec<u8> {
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        assert_eq!(&header[..2], &[3, 0]);
        let len = u16::from_be_bytes([header[2], header[3]]) as usize;
        assert!((7..=487).contains(&len));
        let mut frame = vec![0; len];
        frame[..4].copy_from_slice(&header);
        stream.read_exact(&mut frame[4..]).unwrap();
        frame
    }

    fn reference(frame: &[u8]) -> [u8; 2] {
        [frame[11], frame[12]]
    }

    fn ack(request: &[u8], parameters: &[u8], data: &[u8], errors: [u8; 2]) -> Vec<u8> {
        let mut frame = vec![3, 0, 0, 0, 2, 0xf0, 0x80, 0x32, 3, 0, 0];
        frame.extend_from_slice(&reference(request));
        frame.extend_from_slice(&(parameters.len() as u16).to_be_bytes());
        frame.extend_from_slice(&(data.len() as u16).to_be_bytes());
        frame.extend_from_slice(&errors);
        frame.extend_from_slice(parameters);
        frame.extend_from_slice(data);
        let len = frame.len() as u16;
        frame[2..4].copy_from_slice(&len.to_be_bytes());
        frame
    }

    fn read_ack(request: &[u8], data: &[u8], bit: bool) -> Vec<u8> {
        let mut item = vec![0xff, if bit { 3 } else { 4 }];
        let bits = if bit { 1 } else { data.len() as u16 * 8 };
        item.extend_from_slice(&bits.to_be_bytes());
        item.extend_from_slice(data);
        ack(request, &[4, 1], &item, [0, 0])
    }

    fn send(stream: &mut TcpStream, data: &[u8], fragmented: bool) {
        if fragmented {
            for byte in data {
                stream.write_all(&[*byte]).unwrap();
                thread::sleep(Duration::from_millis(1));
            }
        } else {
            stream.write_all(data).unwrap();
        }
    }

    fn handshake(stream: &mut TcpStream, pdu: u16, fragmented: bool) {
        let request = receive(stream);
        assert_eq!(&request[4..11], &[17, 0xe0, 0, 0, 0, 1, 0]);
        assert!(request.windows(4).any(|p| p == [0xc1, 2, 1, 0]));
        assert!(request.windows(4).any(|p| p == [0xc2, 2, 1, 2]));
        let confirm = [
            3, 0, 0, 22, 17, 0xd0, 0, 1, 0, 2, 0, 0xc0, 1, 10, 0xc1, 2, 1, 0, 0xc2, 2, 1, 2,
        ];
        send(stream, &confirm, fragmented);
        let setup = receive(stream);
        assert_eq!(&setup[4..11], &[2, 0xf0, 0x80, 0x32, 1, 0, 0]);
        assert_eq!(&setup[13..], &[0, 8, 0, 0, 0xf0, 0, 0, 1, 0, 1, 1, 0xe0]);
        let mut parameters = vec![0xf0, 0, 0, 1, 0, 1];
        parameters.extend_from_slice(&pdu.to_be_bytes());
        send(stream, &ack(&setup, &parameters, &[], [0, 0]), fragmented);
    }

    fn server(handler: impl FnOnce(TcpStream) + Send + 'static) -> (Config, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let join = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream.set_nodelay(true).unwrap();
            handler(stream);
        });
        (
            Config {
                endpoint,
                local_tsap: 0x0100,
                remote_tsap: 0x0102,
                timeout: Duration::from_secs(2),
            },
            join,
        )
    }

    fn connected(
        pdu: u16,
        handler: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (Client, JoinHandle<()>) {
        let (config, join) = server(move |mut stream| {
            handshake(&mut stream, pdu, false);
            handler(stream);
        });
        (Client::connect(config).unwrap(), join)
    }

    #[test]
    fn accepts_bytewise_fragmented_handshake_and_response() {
        let (config, join) = server(|mut stream| {
            handshake(&mut stream, 480, true);
            let request = receive(&mut stream);
            assert_eq!(
                &request[17..],
                &[4, 1, 0x12, 0x0a, 0x10, 2, 0, 4, 0, 1, 0x84, 0, 0, 0]
            );
            send(
                &mut stream,
                &read_ack(&request, &[0x12, 0x34, 0xab, 0xcd], false),
                true,
            );
        });
        let mut client = Client::connect(config).unwrap();
        assert_eq!(
            client.read_bytes(Area::DataBlock, 1, 0, 4).unwrap(),
            [0x12, 0x34, 0xab, 0xcd]
        );
        client.disconnect();
        join.join().unwrap();
    }

    #[test]
    fn encodes_all_four_areas_and_increments_reference() {
        let (mut client, join) = connected(480, |mut stream| {
            let mut last_reference = 1;
            for (area, db) in [(0x81, 0), (0x82, 0), (0x83, 0), (0x84, 7)] {
                let read = receive(&mut stream);
                assert_eq!(read[27], area);
                assert_eq!(u16::from_be_bytes([read[25], read[26]]), db);
                assert_eq!(&read[28..31], &[0, 0, 16]);
                let next = u16::from_be_bytes(reference(&read));
                assert_eq!(next, last_reference + 1);
                last_reference = next;
                stream.write_all(&read_ack(&read, &[0x5a], false)).unwrap();
                let write = receive(&mut stream);
                assert_eq!(write[27], area);
                assert_eq!(u16::from_be_bytes([write[25], write[26]]), db);
                assert_eq!(&write[28..], &[0, 0, 16, 0, 4, 0, 8, 0x5a]);
                let next = u16::from_be_bytes(reference(&write));
                assert_eq!(next, last_reference + 1);
                last_reference = next;
                stream
                    .write_all(&ack(&write, &[5, 1], &[0xff], [0, 0]))
                    .unwrap();
            }
        });
        for area in [Area::Inputs, Area::Outputs, Area::Markers, Area::DataBlock] {
            assert_eq!(client.read_bytes(area, 7, 2, 1).unwrap(), [0x5a]);
            client.write_bytes(area, 7, 2, &[0x5a]).unwrap();
        }
        join.join().unwrap();
    }

    #[test]
    fn high_offset_bit_access_uses_single_atomic_bit_telegram() {
        let (mut client, join) = connected(480, |mut stream| {
            let read = receive(&mut stream);
            assert_eq!(&read[22..31], &[1, 0, 1, 0, 9, 0x84, 1, 0, 3]);
            stream.write_all(&read_ack(&read, &[1], true)).unwrap();
            let write = receive(&mut stream);
            assert_eq!(write[17], 5);
            assert_eq!(&write[22..], &[1, 0, 1, 0, 9, 0x84, 1, 0, 3, 0, 3, 0, 1, 0]);
            stream
                .write_all(&ack(&write, &[5, 1], &[0xff], [0, 0]))
                .unwrap();
        });
        assert!(client.read_bit(Area::DataBlock, 9, 8192, 3).unwrap());
        client
            .write_bit(Area::DataBlock, 9, 8192, 3, false)
            .unwrap();
        join.join().unwrap();
    }

    #[test]
    fn wrong_reference_disconnects_without_retry() {
        let (mut client, join) = connected(480, |mut stream| {
            let request = receive(&mut stream);
            let mut response = read_ack(&request, &[1], false);
            response[12] ^= 1;
            stream.write_all(&response).unwrap();
            assert_eq!(stream.read(&mut [0; 1]).unwrap(), 0);
        });
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 1)
                .unwrap_err()
                .contains("关联号")
        );
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 1)
                .unwrap_err()
                .contains("未连接")
        );
        join.join().unwrap();
    }

    #[test]
    fn reports_top_level_and_item_rejection() {
        let (mut client, join) = connected(480, |mut stream| {
            let request = receive(&mut stream);
            stream
                .write_all(&ack(&request, &[4, 1], &[], [0x81, 4]))
                .unwrap();
        });
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 1)
                .unwrap_err()
                .contains("8104")
        );
        join.join().unwrap();
        let (mut client, join) = connected(480, |mut stream| {
            let request = receive(&mut stream);
            stream
                .write_all(&ack(&request, &[4, 1], &[5, 0, 0, 0], [0, 0]))
                .unwrap();
            let write = receive(&mut stream);
            stream
                .write_all(&ack(&write, &[5, 1], &[3], [0, 0]))
                .unwrap();
        });
        assert!(
            client
                .read_bytes(Area::DataBlock, 1, 0, 1)
                .unwrap_err()
                .contains("地址")
        );
        assert!(
            client
                .write_bytes(Area::DataBlock, 1, 0, &[1])
                .unwrap_err()
                .contains("拒绝")
        );
        join.join().unwrap();
    }

    #[test]
    fn rejects_lengths_and_addresses_before_sending() {
        let (mut client, join) = connected(480, |mut stream| {
            let read = receive(&mut stream);
            assert_eq!(read[24], 200);
            stream
                .write_all(&read_ack(&read, &[0xaa; 200], false))
                .unwrap();
        });
        assert!(
            client
                .read_bytes(Area::DataBlock, 1, 0, 201)
                .unwrap_err()
                .contains("200")
        );
        assert!(
            client
                .write_bytes(Area::DataBlock, 1, 0, &[0; 201])
                .unwrap_err()
                .contains("200")
        );
        assert!(client.read_bytes(Area::DataBlock, 1, 0, 0).is_err());
        assert!(client.write_bytes(Area::DataBlock, 1, 0, &[]).is_err());
        assert!(client.read_bytes(Area::DataBlock, 0, 0, 1).is_err());
        assert!(client.read_bytes(Area::DataBlock, 1, 0x20_0000, 1).is_err());
        assert!(client.read_bytes(Area::DataBlock, 1, 0x1f_ffff, 2).is_err());
        assert!(client.write_bit(Area::Markers, 0, 0, 8, true).is_err());
        assert_eq!(
            client.read_bytes(Area::DataBlock, 1, 0, 200).unwrap(),
            [0xaa; 200]
        );
        join.join().unwrap();
    }

    #[test]
    fn respects_negotiated_pdu_limit() {
        let (mut client, join) = connected(64, |mut stream| {
            let read = receive(&mut stream);
            stream.write_all(&read_ack(&read, &[0], false)).unwrap();
        });
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 47)
                .unwrap_err()
                .contains("46")
        );
        assert!(
            client
                .write_bytes(Area::Markers, 0, 0, &[0; 37])
                .unwrap_err()
                .contains("36")
        );
        client.read_bytes(Area::Markers, 0, 0, 1).unwrap();
        join.join().unwrap();
    }

    #[test]
    fn deadline_covers_entire_slow_frame_and_disconnects() {
        let (mut config, join) = server(|mut stream| {
            handshake(&mut stream, 480, false);
            let request = receive(&mut stream);
            for byte in read_ack(&request, &[1], false) {
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
        config.timeout = Duration::from_millis(100);
        let mut client = Client::connect(config).unwrap();
        let start = Instant::now();
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 1)
                .unwrap_err()
                .contains("超时")
        );
        assert!(start.elapsed() < Duration::from_millis(500));
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 1)
                .unwrap_err()
                .contains("未连接")
        );
        join.join().unwrap();
    }

    #[test]
    fn rejects_invalid_cotp_confirm() {
        let (config, join) = server(|mut stream| {
            receive(&mut stream);
            stream
                .write_all(&[3, 0, 0, 11, 6, 0xe0, 0, 1, 0, 2, 0])
                .unwrap();
        });
        let error = Client::connect(config)
            .err()
            .expect("必须拒绝错误 COTP 类型");
        assert!(error.contains("COTP"));
        join.join().unwrap();
    }

    #[test]
    fn rejects_invalid_setup_and_short_payload() {
        let (config, join) = server(|mut stream| {
            receive(&mut stream);
            stream
                .write_all(&[3, 0, 0, 11, 6, 0xd0, 0, 1, 0, 2, 0])
                .unwrap();
            let setup = receive(&mut stream);
            stream
                .write_all(&ack(&setup, &[0xf0, 0, 0, 1, 0, 1, 0, 16], &[], [0, 0]))
                .unwrap();
        });
        let error = Client::connect(config).err().expect("必须拒绝过小 PDU");
        assert!(error.contains("PDU"));
        join.join().unwrap();
        let (mut client, join) = connected(480, |mut stream| {
            let request = receive(&mut stream);
            stream.write_all(&read_ack(&request, &[1], false)).unwrap();
        });
        assert!(
            client
                .read_bytes(Area::Markers, 0, 0, 2)
                .unwrap_err()
                .contains("长度")
        );
        join.join().unwrap();
    }
}
