//! Modbus TCP / RTU 帧编解码，不持有网络或串口连接。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Coils,
    DiscreteInputs,
    HoldingRegisters,
    InputRegisters,
}

impl Area {
    pub fn label(self) -> &'static str {
        match self {
            Self::Coils => "线圈",
            Self::DiscreteInputs => "离散输入",
            Self::HoldingRegisters => "保持寄存器",
            Self::InputRegisters => "输入寄存器",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    unit: u8,
    quantity: u16,
    pdu: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum Response {
    Bits(Vec<bool>),
    Registers(Vec<u16>),
    Written(u16),
}

impl Request {
    pub fn is_read(&self) -> bool {
        self.pdu[0] <= 4
    }

    pub fn unit(&self) -> u8 {
        self.unit
    }

    pub fn read(unit: u8, area: Area, address: u16, quantity: u16) -> Result<Self, String> {
        let (function, limit) = match area {
            Area::Coils => (1, 2000),
            Area::DiscreteInputs => (2, 2000),
            Area::HoldingRegisters => (3, 125),
            Area::InputRegisters => (4, 125),
        };
        validate_range(address, usize::from(quantity), limit)?;
        Ok(Self {
            unit,
            quantity,
            pdu: request_header(function, address, quantity),
        })
    }

    pub fn write_coils(unit: u8, address: u16, values: &[bool]) -> Result<Self, String> {
        validate_range(address, values.len(), 1968)?;
        let quantity = values.len() as u16;
        let pdu = if quantity == 1 {
            request_header(5, address, if values[0] { 0xff00 } else { 0 })
        } else {
            let mut pdu = request_header(15, address, quantity);
            pdu.push(values.len().div_ceil(8) as u8);
            for chunk in values.chunks(8) {
                let mut byte = 0;
                for (bit, value) in chunk.iter().enumerate() {
                    if *value {
                        byte |= 1 << bit;
                    }
                }
                pdu.push(byte);
            }
            pdu
        };
        Ok(Self {
            unit,
            quantity,
            pdu,
        })
    }

    pub fn write_registers(unit: u8, address: u16, values: &[u16]) -> Result<Self, String> {
        validate_range(address, values.len(), 123)?;
        let quantity = values.len() as u16;
        let pdu = if quantity == 1 {
            request_header(6, address, values[0])
        } else {
            let mut pdu = request_header(16, address, quantity);
            pdu.push((values.len() * 2) as u8);
            for value in values {
                pdu.extend_from_slice(&value.to_be_bytes());
            }
            pdu
        };
        Ok(Self {
            unit,
            quantity,
            pdu,
        })
    }

    pub fn encode_tcp(&self, transaction: u16) -> Vec<u8> {
        let mut frame = Vec::with_capacity(7 + self.pdu.len());
        frame.extend_from_slice(&transaction.to_be_bytes());
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&((self.pdu.len() + 1) as u16).to_be_bytes());
        frame.push(self.unit);
        frame.extend_from_slice(&self.pdu);
        frame
    }

    pub fn encode_rtu(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(3 + self.pdu.len());
        frame.push(self.unit);
        frame.extend_from_slice(&self.pdu);
        frame.extend_from_slice(&crc16(&frame).to_le_bytes());
        frame
    }

    pub fn decode_tcp(&self, transaction: u16, frame: &[u8]) -> Result<Response, String> {
        if frame.len() < 8 {
            return Err("TCP 响应帧过短".into());
        }
        if u16::from_be_bytes([frame[0], frame[1]]) != transaction {
            return Err("TCP 响应事务号与请求不一致".into());
        }
        if frame[2..4] != [0, 0] {
            return Err("TCP 响应协议标识必须为 0".into());
        }
        let length = usize::from(u16::from_be_bytes([frame[4], frame[5]]));
        if !(2..=254).contains(&length) || frame.len() != 6 + length {
            return Err("TCP 响应 MBAP 长度与完整帧不一致或超出协议范围".into());
        }
        if frame[6] != self.unit {
            return Err("TCP 响应站号与请求不一致".into());
        }
        self.decode_pdu(&frame[7..])
    }

    pub fn decode_rtu(&self, frame: &[u8]) -> Result<Response, String> {
        if !(5..=256).contains(&frame.len()) {
            return Err("RTU 响应帧长度超出协议范围".into());
        }
        let crc_offset = frame.len() - 2;
        let received_crc = u16::from_le_bytes([frame[crc_offset], frame[crc_offset + 1]]);
        if crc16(&frame[..crc_offset]) != received_crc {
            return Err("RTU 响应 CRC 校验失败".into());
        }
        if frame[0] != self.unit {
            return Err("RTU 响应站号与请求不一致".into());
        }
        self.decode_pdu(&frame[1..crc_offset])
    }

    fn decode_pdu(&self, pdu: &[u8]) -> Result<Response, String> {
        let function = self.pdu[0];
        if pdu[0] == function | 0x80 {
            if pdu.len() != 2 {
                return Err("Modbus 异常响应长度错误".into());
            }
            let code = pdu[1];
            let explanation = match code {
                1 => "非法功能",
                2 => "非法数据地址",
                3 => "非法数据值",
                4 => "设备故障",
                5 => "确认，设备正在处理请求",
                6 => "设备忙",
                8 => "存储器奇偶校验错误",
                10 => "网关路径不可用",
                11 => "网关目标设备无响应",
                _ => "未知异常",
            };
            return Err(format!(
                "Modbus 异常功能码 0x{:02X}，异常码 0x{code:02X}：{explanation}",
                pdu[0]
            ));
        }
        if pdu[0] != function {
            return Err("Modbus 响应功能码与请求不一致".into());
        }
        if function <= 4 {
            let quantity = usize::from(self.quantity);
            let byte_count = if function <= 2 {
                quantity.div_ceil(8)
            } else {
                quantity * 2
            };
            if pdu.len() != byte_count + 2 || usize::from(pdu[1]) != byte_count {
                return Err("Modbus 读取响应字节数与请求数量不一致".into());
            }
            if function <= 2 {
                Ok(Response::Bits(
                    (0..quantity)
                        .map(|bit| pdu[2 + bit / 8] & (1 << (bit % 8)) != 0)
                        .collect(),
                ))
            } else {
                Ok(Response::Registers(
                    pdu[2..]
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|bytes| u16::from_be_bytes(*bytes))
                        .collect(),
                ))
            }
        } else if pdu.len() == 5 && pdu[1..5] == self.pdu[1..5] {
            Ok(Response::Written(self.quantity))
        } else {
            Err("Modbus 写入响应的地址、数量或数值与请求不一致".into())
        }
    }
}

fn validate_range(address: u16, quantity: usize, limit: usize) -> Result<(), String> {
    if quantity == 0 || quantity > limit {
        return Err(format!("数量必须在 1..={limit} 范围内"));
    }
    if usize::from(address) + quantity > 65_536 {
        return Err("起始地址加数量超出 0..=65535 地址范围".into());
    }
    Ok(())
}

fn request_header(function: u8, address: u16, value: u16) -> Vec<u8> {
    let mut pdu = vec![function];
    pdu.extend_from_slice(&address.to_be_bytes());
    pdu.extend_from_slice(&value.to_be_bytes());
    pdu
}

/// 根据包含站号的 RTU 响应前缀返回完整帧长度；缺少必要头部时返回 None。
pub fn rtu_response_len(prefix: &[u8]) -> Result<Option<usize>, String> {
    if prefix.len() < 2 {
        return Ok(None);
    }
    match prefix[1] {
        1..=4 => Ok(prefix.get(2).map(|byte_count| usize::from(*byte_count) + 5)),
        5 | 6 | 15 | 16 => Ok(Some(8)),
        0x81..=0x86 | 0x8f | 0x90 => Ok(Some(5)),
        function => Err(format!("不支持的 RTU 响应功能码 0x{function:02X}")),
    }
}

/// CRC-16/MODBUS，发送时低字节在前。
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff;
    for byte in bytes {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    // PDU 黄金帧取自 Modbus Application Protocol V1.1b3 的功能码示例；
    // TCP 头手工构造，RTU CRC 另用固定校验向量核验。
    // https://www.modbus.org/file/secure/modbusprotocolspecification.pdf
    fn tcp(unit: u8, pdu: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x12, 0x34, 0, 0, 0, (pdu.len() + 1) as u8, unit];
        frame.extend_from_slice(pdu);
        frame
    }

    #[test]
    fn area_labels_are_chinese() {
        assert_eq!(Area::Coils.label(), "线圈");
        assert_eq!(Area::DiscreteInputs.label(), "离散输入");
        assert_eq!(Area::HoldingRegisters.label(), "保持寄存器");
        assert_eq!(Area::InputRegisters.label(), "输入寄存器");
    }

    #[test]
    fn exposes_station_and_distinguishes_reads_from_writes() {
        for area in [
            Area::Coils,
            Area::DiscreteInputs,
            Area::HoldingRegisters,
            Area::InputRegisters,
        ] {
            let request = Request::read(255, area, 0, 1).unwrap();
            assert!(request.is_read());
            assert_eq!(request.unit(), 255);
        }
        for request in [
            Request::write_coils(17, 0, &[true]).unwrap(),
            Request::write_coils(17, 0, &[true, false]).unwrap(),
            Request::write_registers(17, 0, &[1]).unwrap(),
            Request::write_registers(17, 0, &[1, 2]).unwrap(),
        ] {
            assert!(!request.is_read());
            assert_eq!(request.unit(), 17);
        }
    }

    #[test]
    fn crc_matches_fixed_vectors() {
        assert_eq!(crc16(b"123456789"), 0x4b37);
        assert_eq!(crc16(&[1, 3, 0, 0, 0, 10]), 0xcdc5);
        assert_eq!(crc16(&[1, 3, 2, 0, 10]), 0x4338);
    }

    #[test]
    fn read_requests_match_golden_frames() {
        for (area, function) in [
            (Area::Coils, 1),
            (Area::DiscreteInputs, 2),
            (Area::HoldingRegisters, 3),
            (Area::InputRegisters, 4),
        ] {
            let request = Request::read(17, area, 0x13, 19).unwrap();
            assert_eq!(
                request.encode_tcp(0x1234),
                tcp(17, &[function, 0, 0x13, 0, 19])
            );
        }
        let request = Request::read(1, Area::HoldingRegisters, 0, 10).unwrap();
        assert_eq!(request.encode_rtu(), [1, 3, 0, 0, 0, 10, 0xc5, 0xcd]);
    }

    #[test]
    fn single_writes_use_functions_05_and_06() {
        for (value, bytes) in [(true, [0xff, 0]), (false, [0, 0])] {
            let request = Request::write_coils(17, 0xac, &[value]).unwrap();
            assert_eq!(
                request.encode_tcp(0x1234),
                tcp(17, &[5, 0, 0xac, bytes[0], bytes[1]])
            );
        }
        let request = Request::write_registers(17, 1, &[3]).unwrap();
        assert_eq!(request.encode_tcp(0x1234), tcp(17, &[6, 0, 1, 0, 3]));
    }

    #[test]
    fn multiple_writes_match_golden_frames() {
        let coils = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let request = Request::write_coils(17, 0x13, &coils).unwrap();
        assert_eq!(
            request.encode_tcp(0x1234),
            tcp(17, &[15, 0, 0x13, 0, 10, 2, 0xcd, 1])
        );
        let request = Request::write_registers(17, 1, &[10, 258]).unwrap();
        assert_eq!(
            request.encode_tcp(0x1234),
            tcp(17, &[16, 0, 1, 0, 2, 4, 0, 10, 1, 2])
        );
    }

    #[test]
    fn reads_coil_bits_in_lsb_order_and_trims_padding() {
        let request = Request::read(17, Area::Coils, 0x13, 19).unwrap();
        assert_eq!(
            request
                .decode_tcp(0x1234, &tcp(17, &[1, 3, 0xcd, 0x6b, 5]))
                .unwrap(),
            Response::Bits(vec![
                true, false, true, true, false, false, true, true, true, true, false, true, false,
                true, true, false, true, false, true
            ])
        );
        let request = Request::read(17, Area::DiscreteInputs, 0, 2).unwrap();
        assert_eq!(
            request.decode_tcp(0x1234, &tcp(17, &[2, 1, 2])).unwrap(),
            Response::Bits(vec![false, true])
        );
    }

    #[test]
    fn reads_big_endian_registers_over_tcp_and_rtu() {
        let request = Request::read(17, Area::HoldingRegisters, 0x6b, 3).unwrap();
        assert_eq!(
            request
                .decode_tcp(0x1234, &tcp(17, &[3, 6, 2, 0x2b, 0, 0, 0, 0x64]))
                .unwrap(),
            Response::Registers(vec![555, 0, 100])
        );
        let request = Request::read(1, Area::InputRegisters, 8, 1).unwrap();
        assert_eq!(
            request.decode_tcp(0x1234, &tcp(1, &[4, 2, 0, 10])).unwrap(),
            Response::Registers(vec![10])
        );
        let request = Request::read(1, Area::HoldingRegisters, 0, 1).unwrap();
        assert_eq!(
            request.decode_rtu(&[1, 3, 2, 0, 10, 0x38, 0x43]).unwrap(),
            Response::Registers(vec![10])
        );
    }

    #[test]
    fn write_responses_confirm_exact_address_value_and_quantity() {
        let requests = [
            (
                Request::write_coils(17, 0xac, &[true]).unwrap(),
                vec![5, 0, 0xac, 0xff, 0],
                1,
            ),
            (
                Request::write_registers(17, 1, &[3]).unwrap(),
                vec![6, 0, 1, 0, 3],
                1,
            ),
            (
                Request::write_coils(17, 0x13, &[true; 10]).unwrap(),
                vec![15, 0, 0x13, 0, 10],
                10,
            ),
            (
                Request::write_registers(17, 1, &[10, 258]).unwrap(),
                vec![16, 0, 1, 0, 2],
                2,
            ),
        ];
        for (request, pdu, count) in requests {
            assert_eq!(
                request.decode_tcp(0x1234, &tcp(17, &pdu)).unwrap(),
                Response::Written(count)
            );
            for offset in [2, 4] {
                let mut wrong = pdu.clone();
                wrong[offset] ^= 1;
                assert!(request.decode_tcp(0x1234, &tcp(17, &wrong)).is_err());
            }
            assert!(request.decode_tcp(0x1234, &tcp(17, &pdu[..4])).is_err());
            let mut extra = pdu;
            extra.push(0);
            assert!(request.decode_tcp(0x1234, &tcp(17, &extra)).is_err());
        }
    }

    #[test]
    fn tcp_rejects_wrong_transaction_protocol_unit_function_and_length() {
        let request = Request::read(17, Area::HoldingRegisters, 0, 1).unwrap();
        let frame = tcp(17, &[3, 2, 0, 10]);
        for offset in [0, 1, 2, 3, 4, 5, 6, 7] {
            let mut wrong = frame.clone();
            wrong[offset] ^= 1;
            assert!(
                request.decode_tcp(0x1234, &wrong).is_err(),
                "offset {offset}"
            );
        }
        for length in 0..frame.len() {
            assert!(request.decode_tcp(0x1234, &frame[..length]).is_err());
        }
        let mut extra = frame;
        extra.push(0);
        assert!(request.decode_tcp(0x1234, &extra).is_err());
    }

    #[test]
    fn read_response_byte_count_must_match_request_and_payload() {
        for area in [
            Area::Coils,
            Area::DiscreteInputs,
            Area::HoldingRegisters,
            Area::InputRegisters,
        ] {
            let request = Request::read(17, area, 0, 9).unwrap();
            let function = request.encode_tcp(0)[7];
            for pdu in [
                vec![function],
                vec![function, 0],
                vec![function, 1, 0],
                vec![function, 2, 0],
                vec![function, 2, 0, 0, 0],
            ] {
                assert!(request.decode_tcp(0x1234, &tcp(17, &pdu)).is_err());
            }
        }
    }

    #[test]
    fn rtu_rejects_bad_crc_short_frames_and_wrong_station() {
        let request = Request::read(1, Area::HoldingRegisters, 0, 1).unwrap();
        let frame = [1, 3, 2, 0, 10, 0x38, 0x43];
        for offset in 0..frame.len() {
            let mut wrong = frame;
            wrong[offset] ^= 1;
            assert!(request.decode_rtu(&wrong).is_err());
        }
        for length in 0..frame.len() {
            assert!(request.decode_rtu(&frame[..length]).is_err());
        }
        let other = Request::read(2, Area::HoldingRegisters, 0, 1).unwrap();
        assert!(other.decode_rtu(&frame).is_err());
    }

    #[test]
    fn exceptions_include_code_and_chinese_explanation() {
        let request = Request::read(17, Area::HoldingRegisters, 0, 1).unwrap();
        for (code, explanation) in [
            (1, "非法功能"),
            (2, "非法数据地址"),
            (3, "非法数据值"),
            (4, "设备故障"),
            (5, "确认"),
            (6, "设备忙"),
            (8, "存储器奇偶校验错误"),
            (10, "网关路径不可用"),
            (11, "网关目标设备无响应"),
            (255, "未知异常"),
        ] {
            let error = request
                .decode_tcp(0x1234, &tcp(17, &[0x83, code]))
                .unwrap_err();
            assert!(error.contains(explanation), "{error}");
            assert!(error.contains(&format!("0x{code:02X}")), "{error}");
        }
        assert!(
            request
                .decode_tcp(0x1234, &tcp(17, &[0x84, 2]))
                .unwrap_err()
                .contains("功能码")
        );
        assert!(request.decode_tcp(0x1234, &tcp(17, &[0x83])).is_err());
        assert!(request.decode_tcp(0x1234, &tcp(17, &[0x83, 2, 0])).is_err());
        let request = Request::read(1, Area::HoldingRegisters, 0, 1).unwrap();
        assert!(
            request
                .decode_rtu(&[1, 0x83, 2, 0xc0, 0xf1])
                .unwrap_err()
                .contains("非法数据地址")
        );
    }

    #[test]
    fn read_quantity_limits_and_last_address_are_enforced() {
        for (area, limit) in [
            (Area::Coils, 2000),
            (Area::DiscreteInputs, 2000),
            (Area::HoldingRegisters, 125),
            (Area::InputRegisters, 125),
        ] {
            assert!(Request::read(1, area, 0, 0).is_err());
            assert!(Request::read(1, area, 0, limit).is_ok());
            assert!(Request::read(1, area, 0, limit + 1).is_err());
            assert!(Request::read(1, area, u16::MAX, 1).is_ok());
            assert!(Request::read(1, area, u16::MAX, 2).is_err());
            assert!(Request::read(1, area, u16::MAX - limit + 1, limit).is_ok());
        }
        for unit in [0, 247, 248, 255] {
            assert!(Request::read(unit, Area::Coils, 0, 1).is_ok());
        }
    }

    #[test]
    fn write_quantity_limits_and_last_address_are_enforced() {
        assert!(Request::write_coils(1, 0, &[]).is_err());
        assert!(Request::write_registers(1, 0, &[]).is_err());
        assert!(Request::write_coils(1, 0, &[true; 1968]).is_ok());
        assert!(Request::write_coils(1, 0, &[true; 1969]).is_err());
        assert!(Request::write_registers(1, 0, &[0; 123]).is_ok());
        assert!(Request::write_registers(1, 0, &[0; 124]).is_err());
        assert!(Request::write_coils(1, u16::MAX, &[true]).is_ok());
        assert!(Request::write_registers(1, u16::MAX, &[0]).is_ok());
        assert!(Request::write_coils(1, u16::MAX, &[true, false]).is_err());
        assert!(Request::write_registers(1, u16::MAX, &[0, 1]).is_err());
        assert!(Request::write_coils(1, 0, &vec![true; 65_536]).is_err());
        assert!(Request::write_registers(1, 0, &vec![0; 65_536]).is_err());
    }

    #[test]
    fn rtu_length_waits_for_read_byte_count_and_identifies_supported_functions() {
        assert_eq!(rtu_response_len(&[]).unwrap(), None);
        assert_eq!(rtu_response_len(&[1]).unwrap(), None);
        for function in [1, 2, 3, 4] {
            assert_eq!(rtu_response_len(&[1, function]).unwrap(), None);
            assert_eq!(rtu_response_len(&[1, function, 6]).unwrap(), Some(11));
            assert_eq!(rtu_response_len(&[1, function, 250]).unwrap(), Some(255));
        }
        for function in [5, 6, 15, 16] {
            assert_eq!(rtu_response_len(&[1, function]).unwrap(), Some(8));
        }
        for function in [0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x8f, 0x90] {
            assert_eq!(rtu_response_len(&[1, function]).unwrap(), Some(5));
        }
        for function in [0, 7, 0x80, 0x87, 0xff] {
            assert!(rtu_response_len(&[1, function]).is_err());
        }
    }
}
