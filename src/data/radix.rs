//! IEEE 754 浮点 / 整数与进制、字节序的互转（纯逻辑，不依赖界面）。
//!
//! 统一约定：
//! - 值一律以「规范字节」保存：按大端排列的逻辑字节；宽度不足 8 字节时高位补 0。
//! - 十进制输入输出表示数值；十六进制 / 二进制输入输出表示位模式，并受字节序影响。
//! - `ByteOrder::apply` 是自逆变换，格式化与解析共用它即可互相还原。

use std::fmt::Write as _;

/// 数值类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumType {
    F32,
    F64,
    U16,
    I16,
    U32,
    I32,
}

impl NumType {
    /// 按界面展示顺序排列的全部类型。
    pub const ALL: [NumType; 6] = [
        NumType::F32,
        NumType::F64,
        NumType::U16,
        NumType::I16,
        NumType::U32,
        NumType::I32,
    ];

    /// 简短名称。
    pub fn short(self) -> &'static str {
        match self {
            NumType::F32 => "F32",
            NumType::F64 => "F64",
            NumType::U16 => "U16",
            NumType::I16 => "I16",
            NumType::U32 => "U32",
            NumType::I32 => "I32",
        }
    }

    /// 展示名称（浮点补充 PLC 类型名）。
    pub fn label(self) -> &'static str {
        match self {
            NumType::F32 => "F32（REAL）",
            NumType::F64 => "F64（LREAL）",
            NumType::U16 => "U16",
            NumType::I16 => "I16",
            NumType::U32 => "U32",
            NumType::I32 => "I32",
        }
    }

    /// 占用字节数。
    pub fn width(self) -> usize {
        match self {
            NumType::F32 | NumType::U32 | NumType::I32 => 4,
            NumType::F64 => 8,
            NumType::U16 | NumType::I16 => 2,
        }
    }

    /// 是否为浮点类型。
    pub fn is_float(self) -> bool {
        matches!(self, NumType::F32 | NumType::F64)
    }

    /// 是否为有符号整数类型。
    pub fn is_signed_int(self) -> bool {
        matches!(self, NumType::I16 | NumType::I32)
    }
}

/// 输入进制。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputBase {
    Dec,
    Hex,
    Bin,
    Oct,
}

impl InputBase {
    /// 按界面展示顺序排列的全部进制。
    pub const ALL: [InputBase; 4] = [
        InputBase::Dec,
        InputBase::Hex,
        InputBase::Bin,
        InputBase::Oct,
    ];

    /// 展示名称。
    pub fn label(self) -> &'static str {
        match self {
            InputBase::Dec => "十进制",
            InputBase::Hex => "十六进制",
            InputBase::Bin => "二进制",
            InputBase::Oct => "八进制",
        }
    }

    /// 该类型是否支持此进制（浮点不支持八进制）。
    pub fn supported(self, ty: NumType) -> bool {
        !(matches!(self, InputBase::Oct) && ty.is_float())
    }
}

/// 字节序 / 字节排列方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    Abcd,
    Badc,
    Cdab,
    Dcba,
}

impl ByteOrder {
    /// 按界面展示顺序排列的全部字节序。
    pub const ALL: [ByteOrder; 4] = [
        ByteOrder::Abcd,
        ByteOrder::Badc,
        ByteOrder::Cdab,
        ByteOrder::Dcba,
    ];

    /// 展示名称。
    pub fn label(self) -> &'static str {
        match self {
            ByteOrder::Abcd => "ABCD（大端）",
            ByteOrder::Badc => "BADC（字节交换）",
            ByteOrder::Cdab => "CDAB（字交换）",
            ByteOrder::Dcba => "DCBA（小端）",
        }
    }

    /// 悬停说明。
    pub fn hover_text(self, width: usize) -> &'static str {
        match self {
            ByteOrder::Abcd => "大端：字节按 ABCD 顺序显示（高字节在前）",
            ByteOrder::Badc => {
                if width == 2 {
                    "2 字节类型下与 DCBA 等价：交换两个字节"
                } else {
                    "字节交换：每个 16 位字内交换高低字节（BADC）"
                }
            }
            ByteOrder::Cdab => {
                if width == 2 {
                    "2 字节类型下与 ABCD 等价：只有一个字，顺序不变"
                } else {
                    "字交换：反转 16 位字的顺序（4 字节 CDAB，8 字节 GHEFCDAB）"
                }
            }
            ByteOrder::Dcba => "小端：整体反转字节顺序（DCBA）",
        }
    }

    /// 该宽度下是否允许选择此字节序（2 字节只允许 ABCD / DCBA）。
    pub fn applicable(self, width: usize) -> bool {
        match width {
            2 => matches!(self, ByteOrder::Abcd | ByteOrder::Dcba),
            4 | 8 => true,
            _ => false,
        }
    }

    /// 归一化到该宽度下等价的可用字节序。
    pub fn normalized(self, width: usize) -> ByteOrder {
        if width == 2 {
            match self {
                ByteOrder::Badc => ByteOrder::Dcba,
                ByteOrder::Cdab => ByteOrder::Abcd,
                other => other,
            }
        } else {
            self
        }
    }

    /// 就地变换字节序（自逆：连续应用两次恢复原样）。
    pub fn apply(self, bytes: &mut [u8]) {
        match self {
            ByteOrder::Abcd => {}
            ByteOrder::Badc => {
                let mut i = 0;
                while i + 1 < bytes.len() {
                    bytes.swap(i, i + 1);
                    i += 2;
                }
            }
            ByteOrder::Cdab => {
                let words = bytes.len() / 2;
                for w in 0..words / 2 {
                    let a = w * 2;
                    let b = (words - 1 - w) * 2;
                    bytes.swap(a, b);
                    bytes.swap(a + 1, b + 1);
                }
            }
            ByteOrder::Dcba => bytes.reverse(),
        }
    }
}

/// 一个数值：规范字节按大端排列，宽度之外的高位字节为 0。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value {
    pub ty: NumType,
    pub bytes: [u8; 8],
}

/// 解析用户输入。
pub fn parse(input: &str, ty: NumType, base: InputBase, order: ByteOrder) -> Result<Value, String> {
    if !base.supported(ty) {
        return Err(format!("{} 不支持{}输入", ty.short(), base.label()));
    }
    match base {
        InputBase::Dec => parse_dec(input, ty),
        InputBase::Hex => parse_hex(input, ty, order),
        InputBase::Bin => parse_bin(input, ty, order),
        InputBase::Oct => parse_oct(input, ty),
    }
}

/// 按指定进制回填输入框文本（浮点 + 八进制返回 None）。
pub fn format_input(v: &Value, base: InputBase, order: ByteOrder) -> Option<String> {
    match base {
        InputBase::Dec => match v.ty {
            NumType::F32 | NumType::F64 => fmt_float(v),
            NumType::I16 | NumType::I32 => fmt_dec_signed(v),
            NumType::U16 | NumType::U32 => fmt_dec_unsigned(v),
        },
        InputBase::Hex => Some(fmt_hex(v, order)),
        InputBase::Bin => Some(fmt_bin(v, order)),
        InputBase::Oct => fmt_oct(v),
    }
}

/// 浮点数值文本（最短往返表示；NaN / Inf / -Inf / -0 特殊处理）。
pub fn fmt_float(v: &Value) -> Option<String> {
    match v.ty {
        NumType::F32 => Some(fmt_f32(f32_from_value(v))),
        NumType::F64 => Some(fmt_f64(f64_from_value(v))),
        _ => None,
    }
}

/// 浮点附注：非规范数（subnormal）时返回提示。
pub fn float_note(v: &Value) -> Option<&'static str> {
    let is_subnormal = match v.ty {
        NumType::F32 => {
            let x = f32_from_value(v);
            x.is_finite() && x != 0.0 && !x.is_normal()
        }
        NumType::F64 => {
            let x = f64_from_value(v);
            x.is_finite() && x != 0.0 && !x.is_normal()
        }
        _ => false,
    };
    if is_subnormal {
        Some("非规范数")
    } else {
        None
    }
}

/// 有符号十进制视图（仅整数）。
pub fn fmt_dec_signed(v: &Value) -> Option<String> {
    match v.ty {
        NumType::U16 | NumType::I16 => Some((u16_bits(v) as i16).to_string()),
        NumType::U32 | NumType::I32 => Some((u32_bits(v) as i32).to_string()),
        NumType::F32 | NumType::F64 => None,
    }
}

/// 无符号十进制视图（仅整数）。
pub fn fmt_dec_unsigned(v: &Value) -> Option<String> {
    match v.ty {
        NumType::U16 | NumType::I16 => Some(u16_bits(v).to_string()),
        NumType::U32 | NumType::I32 => Some(u32_bits(v).to_string()),
        NumType::F32 | NumType::F64 => None,
    }
}

/// 十六进制位模式（大写，空格分隔）。
pub fn fmt_hex(v: &Value, order: ByteOrder) -> String {
    let bytes = wire_vec(v, order);
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{b:02X}");
    }
    out
}

/// 十六进制位模式（无空格）。
pub fn fmt_hex_compact(v: &Value, order: ByteOrder) -> String {
    let bytes = wire_vec(v, order);
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02X}");
    }
    out
}

/// 二进制位模式（每 4 位一组，空格分隔）。
pub fn fmt_bin(v: &Value, order: ByteOrder) -> String {
    let compact = fmt_bin_compact(v, order);
    let mut out = String::with_capacity(compact.len() + compact.len() / 4);
    for (i, c) in compact.chars().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// 二进制位模式（无空格）。
pub fn fmt_bin_compact(v: &Value, order: ByteOrder) -> String {
    let bytes = wire_vec(v, order);
    let mut out = String::with_capacity(bytes.len() * 8);
    for b in bytes {
        let _ = write!(out, "{b:08b}");
    }
    out
}

/// 八进制无符号视图（仅整数，不补零）。
pub fn fmt_oct(v: &Value) -> Option<String> {
    match v.ty {
        NumType::U16 | NumType::I16 => Some(format!("{:o}", u16_bits(v))),
        NumType::U32 | NumType::I32 => Some(format!("{:o}", u32_bits(v))),
        NumType::F32 | NumType::F64 => None,
    }
}

// —— 内部实现 ——

/// 由规范字节构造 Value。
fn value_from_slice(ty: NumType, src: &[u8]) -> Value {
    let mut bytes = [0u8; 8];
    bytes[8 - ty.width()..].copy_from_slice(src);
    Value { ty, bytes }
}

/// 规范字节（去掉高位补零）。
fn canonical_slice(v: &Value) -> &[u8] {
    &v.bytes[8 - v.ty.width()..]
}

/// 规范字节按指定字节序变换后的线序字节。
fn wire_vec(v: &Value, order: ByteOrder) -> Vec<u8> {
    let mut out = canonical_slice(v).to_vec();
    order.apply(&mut out);
    out
}

fn f32_from_value(v: &Value) -> f32 {
    f32::from_bits(u32::from_be_bytes([
        v.bytes[4], v.bytes[5], v.bytes[6], v.bytes[7],
    ]))
}

fn f64_from_value(v: &Value) -> f64 {
    f64::from_bits(u64::from_be_bytes(v.bytes))
}

fn u16_bits(v: &Value) -> u16 {
    u16::from_be_bytes([v.bytes[6], v.bytes[7]])
}

fn u32_bits(v: &Value) -> u32 {
    u32::from_be_bytes([v.bytes[4], v.bytes[5], v.bytes[6], v.bytes[7]])
}

fn fmt_f32(x: f32) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() { "-Inf" } else { "Inf" }.to_string();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    let abs = x.abs();
    if abs < 1e-6 || abs >= 1e12 {
        format!("{x:e}")
    } else {
        x.to_string()
    }
}

fn fmt_f64(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() { "-Inf" } else { "Inf" }.to_string();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    let abs = x.abs();
    if abs < 1e-6 || abs >= 1e12 {
        format!("{x:e}")
    } else {
        x.to_string()
    }
}

/// 去掉空白与下划线。
fn strip_separators(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect()
}

/// 去掉可选进制前缀（大小写均可）。
fn strip_radix_prefix<'a>(s: &'a str, lower: &str, upper: &str) -> &'a str {
    if let Some(rest) = s.strip_prefix(lower) {
        rest
    } else if let Some(rest) = s.strip_prefix(upper) {
        rest
    } else {
        s
    }
}

fn parse_dec(input: &str, ty: NumType) -> Result<Value, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("输入为空".to_string());
    }
    match ty {
        NumType::F32 => {
            let x: f32 = s.parse().map_err(|_| format!("无效的十进制浮点数：{s}"))?;
            Ok(value_from_slice(ty, &x.to_be_bytes()))
        }
        NumType::F64 => {
            let x: f64 = s.parse().map_err(|_| format!("无效的十进制浮点数：{s}"))?;
            Ok(value_from_slice(ty, &x.to_be_bytes()))
        }
        NumType::U16 | NumType::U32 => {
            if s.starts_with('-') {
                return Err("无符号类型不支持负数".to_string());
            }
            let n: u64 = s.parse().map_err(|_| format!("无效的十进制整数：{s}"))?;
            let max = if ty == NumType::U16 {
                u16::MAX as u64
            } else {
                u32::MAX as u64
            };
            if n > max {
                return Err(format!("数值超出 {} 范围（0 ~ {}）", ty.short(), max));
            }
            let wire: Vec<u8> = if ty == NumType::U16 {
                (n as u16).to_be_bytes().to_vec()
            } else {
                (n as u32).to_be_bytes().to_vec()
            };
            Ok(value_from_slice(ty, &wire))
        }
        NumType::I16 | NumType::I32 => {
            let n: i64 = s.parse().map_err(|_| format!("无效的十进制整数：{s}"))?;
            let (min, max) = if ty == NumType::I16 {
                (i16::MIN as i64, i16::MAX as i64)
            } else {
                (i32::MIN as i64, i32::MAX as i64)
            };
            if n < min || n > max {
                return Err(format!("数值超出 {} 范围（{} ~ {}）", ty.short(), min, max));
            }
            let wire: Vec<u8> = if ty == NumType::I16 {
                (n as i16).to_be_bytes().to_vec()
            } else {
                (n as i32).to_be_bytes().to_vec()
            };
            Ok(value_from_slice(ty, &wire))
        }
    }
}

fn parse_hex(input: &str, ty: NumType, order: ByteOrder) -> Result<Value, String> {
    let cleaned = strip_separators(input);
    let digits = strip_radix_prefix(&cleaned, "0x", "0X");
    if digits.is_empty() {
        return Err("输入为空".to_string());
    }
    let width = ty.width();
    let max_digits = width * 2;
    let mut count = 0;
    for c in digits.chars() {
        if !c.is_ascii_hexdigit() {
            return Err(format!("无效的十六进制字符：{c}"));
        }
        count += 1;
    }
    if count > max_digits {
        return Err(format!(
            "十六进制位数超出 {} 上限（最多 {max_digits} 位）",
            ty.short()
        ));
    }
    let padded: String = "0"
        .repeat(max_digits - count)
        .chars()
        .chain(digits.chars())
        .collect();
    let mut wire = Vec::with_capacity(width);
    for pair in padded.as_bytes().chunks_exact(2) {
        wire.push(hex_nibble(pair[0]) << 4 | hex_nibble(pair[1]));
    }
    order.apply(&mut wire);
    Ok(value_from_slice(ty, &wire))
}

/// 已校验的 ASCII 十六进制字符转半字节。
fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => c - b'A' + 10,
    }
}

fn parse_bin(input: &str, ty: NumType, order: ByteOrder) -> Result<Value, String> {
    let cleaned = strip_separators(input);
    let bits = strip_radix_prefix(&cleaned, "0b", "0B");
    if bits.is_empty() {
        return Err("输入为空".to_string());
    }
    let width = ty.width();
    let max_bits = width * 8;
    let mut count = 0;
    for c in bits.chars() {
        if c != '0' && c != '1' {
            return Err(format!("无效的二进制字符：{c}"));
        }
        count += 1;
    }
    if count > max_bits {
        return Err(format!(
            "二进制位数超出 {} 上限（最多 {max_bits} 位）",
            ty.short()
        ));
    }
    let padded = format!("{}{}", "0".repeat(max_bits - count), bits);
    let mut wire = vec![0u8; width];
    for (i, c) in padded.bytes().enumerate() {
        if c == b'1' {
            wire[i / 8] |= 0x80u8 >> (i % 8);
        }
    }
    order.apply(&mut wire);
    Ok(value_from_slice(ty, &wire))
}

fn parse_oct(input: &str, ty: NumType) -> Result<Value, String> {
    if ty.is_float() {
        return Err(format!("{} 不支持八进制输入", ty.short()));
    }
    let cleaned = strip_separators(input);
    let digits = strip_radix_prefix(&cleaned, "0o", "0O");
    if digits.is_empty() {
        return Err("输入为空".to_string());
    }
    let width = ty.width();
    let max_digits = (width * 8 + 2) / 3;
    let mut count = 0;
    let mut n: u64 = 0;
    for c in digits.chars() {
        if !matches!(c, '0'..='7') {
            return Err(format!("无效的八进制字符：{c}"));
        }
        count += 1;
        n = n * 8 + (c as u8 - b'0') as u64;
    }
    if count > max_digits {
        return Err(format!(
            "八进制位数超出 {} 上限（最多 {max_digits} 位）",
            ty.short()
        ));
    }
    let max = if width == 2 {
        u16::MAX as u64
    } else {
        u32::MAX as u64
    };
    if n > max {
        return Err(format!("数值超出 {} 范围（0 ~ {}）", ty.short(), max));
    }
    let wire: Vec<u8> = if width == 2 {
        (n as u16).to_be_bytes().to_vec()
    } else {
        (n as u32).to_be_bytes().to_vec()
    };
    Ok(value_from_slice(ty, &wire))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(ty: NumType, s: &str, base: InputBase, order: ByteOrder) -> Value {
        parse(s, ty, base, order).unwrap_or_else(|e| panic!("解析 {s:?} 失败：{e}"))
    }

    fn canonical_bytes(v: &Value) -> &[u8] {
        &v.bytes[8 - v.ty.width()..]
    }

    fn permute(order: ByteOrder, input: &[u8]) -> Vec<u8> {
        let mut bytes = input.to_vec();
        order.apply(&mut bytes);
        bytes
    }

    #[test]
    fn test_num_type_table() {
        let rows = [
            (NumType::F32, "F32", "F32（REAL）", 4usize, true, false),
            (NumType::F64, "F64", "F64（LREAL）", 8, true, false),
            (NumType::U16, "U16", "U16", 2, false, false),
            (NumType::I16, "I16", "I16", 2, false, true),
            (NumType::U32, "U32", "U32", 4, false, false),
            (NumType::I32, "I32", "I32", 4, false, true),
        ];
        assert_eq!(NumType::ALL.len(), rows.len());
        for (i, (ty, short, label, width, is_float, is_signed)) in rows.iter().enumerate() {
            let ty = *ty;
            assert_eq!(NumType::ALL[i], ty);
            assert_eq!(ty.short(), *short);
            assert_eq!(ty.label(), *label);
            assert_eq!(ty.width(), *width);
            assert_eq!(ty.is_float(), *is_float);
            assert_eq!(ty.is_signed_int(), *is_signed);
        }
    }

    #[test]
    fn test_byte_order_golden() {
        use ByteOrder::{Abcd, Badc, Cdab, Dcba};

        // 元数据
        assert_eq!(
            ByteOrder::ALL.map(|o| o.label()),
            [
                "ABCD（大端）",
                "BADC（字节交换）",
                "CDAB（字交换）",
                "DCBA（小端）"
            ]
        );
        assert_eq!(
            ByteOrder::ALL.map(|o| o.applicable(2)),
            [true, false, false, true]
        );
        assert_eq!(ByteOrder::ALL.map(|o| o.applicable(4)), [true; 4]);
        assert_eq!(ByteOrder::ALL.map(|o| o.applicable(8)), [true; 4]);
        assert_eq!(Badc.normalized(2), Dcba);
        assert_eq!(Cdab.normalized(2), Abcd);
        assert_eq!(Badc.normalized(4), Badc);
        assert_eq!(Cdab.normalized(8), Cdab);
        assert!(Abcd.hover_text(2).contains("大端"));
        assert!(Dcba.hover_text(8).contains("小端"));
        for width in [2, 4, 8] {
            for order in ByteOrder::ALL {
                assert!(!order.hover_text(width).is_empty());
            }
        }

        // 2 字节
        assert_eq!(permute(Abcd, &[1, 2]), vec![1, 2]);
        assert_eq!(permute(Badc, &[1, 2]), vec![2, 1]);
        assert_eq!(permute(Cdab, &[1, 2]), vec![1, 2]);
        assert_eq!(permute(Dcba, &[1, 2]), vec![2, 1]);
        // 4 字节
        assert_eq!(permute(Abcd, &[1, 2, 3, 4]), vec![1, 2, 3, 4]);
        assert_eq!(permute(Badc, &[1, 2, 3, 4]), vec![2, 1, 4, 3]);
        assert_eq!(permute(Cdab, &[1, 2, 3, 4]), vec![3, 4, 1, 2]);
        assert_eq!(permute(Dcba, &[1, 2, 3, 4]), vec![4, 3, 2, 1]);
        // 8 字节
        assert_eq!(
            permute(Abcd, &[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            permute(Badc, &[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![2, 1, 4, 3, 6, 5, 8, 7]
        );
        assert_eq!(
            permute(Cdab, &[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![7, 8, 5, 6, 3, 4, 1, 2]
        );
        assert_eq!(
            permute(Dcba, &[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![8, 7, 6, 5, 4, 3, 2, 1]
        );
    }

    #[test]
    fn test_byte_order_self_inverse() {
        for order in ByteOrder::ALL {
            for width in [2usize, 4, 8] {
                let original: Vec<u8> = (1..=width as u8).collect();
                let mut bytes = original.clone();
                order.apply(&mut bytes);
                order.apply(&mut bytes);
                assert_eq!(bytes, original, "{} 在 {width} 字节下不自逆", order.label());
            }
        }
    }

    #[test]
    fn test_parse_hex_formats() {
        let a = parse_ok(
            NumType::F32,
            "0x3f c0 00 00",
            InputBase::Hex,
            ByteOrder::Abcd,
        );
        assert_eq!(canonical_bytes(&a), &[0x3F, 0xC0, 0x00, 0x00]);
        let b = parse_ok(NumType::F32, "3FC00000", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(a.bytes, b.bytes);
        assert_eq!(fmt_float(&a).as_deref(), Some("1.5"));
        let c = parse_ok(NumType::F32, "0X3fc00000", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(c.bytes, a.bytes);

        let d = parse_ok(NumType::U16, "ff", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(canonical_bytes(&d), &[0x00, 0xFF]);
        assert_eq!(fmt_dec_unsigned(&d).as_deref(), Some("255"));

        let e = parse_ok(NumType::I16, "FF_FF", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(fmt_dec_signed(&e).as_deref(), Some("-1"));

        let f = parse_ok(NumType::U16, "001", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(fmt_dec_unsigned(&f).as_deref(), Some("1"));
    }

    #[test]
    fn test_parse_hex_errors() {
        assert!(parse("GG", NumType::U16, InputBase::Hex, ByteOrder::Abcd).is_err());
        assert!(parse("010203", NumType::U16, InputBase::Hex, ByteOrder::Abcd).is_err());
        assert_eq!(
            parse("", NumType::U16, InputBase::Hex, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("  ", NumType::U16, InputBase::Hex, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("0x", NumType::U16, InputBase::Hex, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        let err = parse("GG", NumType::U16, InputBase::Hex, ByteOrder::Abcd).unwrap_err();
        assert!(err.contains("无效的十六进制字符"), "错误信息不符：{err}");
    }

    #[test]
    fn test_parse_bin_and_oct() {
        let a = parse_ok(NumType::U16, "0b1010", InputBase::Bin, ByteOrder::Abcd);
        assert_eq!(fmt_dec_unsigned(&a).as_deref(), Some("10"));
        let b = parse_ok(NumType::U16, "1010 0011", InputBase::Bin, ByteOrder::Abcd);
        assert_eq!(fmt_dec_unsigned(&b).as_deref(), Some("163"));
        let c = parse_ok(NumType::U16, "0o17", InputBase::Oct, ByteOrder::Abcd);
        assert_eq!(fmt_dec_unsigned(&c).as_deref(), Some("15"));
        let d = parse_ok(NumType::I16, "177777", InputBase::Oct, ByteOrder::Abcd);
        assert_eq!(fmt_dec_signed(&d).as_deref(), Some("-1"));
        assert_eq!(fmt_oct(&d).as_deref(), Some("177777"));

        // 浮点位模式也可用二进制输入
        let bits = format!("{:032b}", 0x3FC0_0000u32);
        let e = parse_ok(NumType::F32, &bits, InputBase::Bin, ByteOrder::Abcd);
        assert_eq!(fmt_float(&e).as_deref(), Some("1.5"));

        // 非法字符与范围
        assert!(parse("102", NumType::U16, InputBase::Bin, ByteOrder::Abcd).is_err());
        assert!(parse("8", NumType::U16, InputBase::Oct, ByteOrder::Abcd).is_err());
        assert!(parse("1000000", NumType::U16, InputBase::Oct, ByteOrder::Abcd).is_err());
        assert!(parse("200000", NumType::U16, InputBase::Oct, ByteOrder::Abcd).is_err());
        assert!(
            parse(
                &"1".repeat(17),
                NumType::U16,
                InputBase::Bin,
                ByteOrder::Abcd
            )
            .is_err()
        );
        assert_eq!(
            parse("0b", NumType::U16, InputBase::Bin, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("0o", NumType::U16, InputBase::Oct, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
    }

    #[test]
    fn test_parse_dec_int_range() {
        assert_eq!(
            fmt_dec_signed(&parse_ok(
                NumType::I16,
                "-32768",
                InputBase::Dec,
                ByteOrder::Abcd
            ))
            .as_deref(),
            Some("-32768")
        );
        assert_eq!(
            fmt_dec_signed(&parse_ok(
                NumType::I16,
                "32767",
                InputBase::Dec,
                ByteOrder::Abcd
            ))
            .as_deref(),
            Some("32767")
        );
        assert!(parse("32768", NumType::I16, InputBase::Dec, ByteOrder::Abcd).is_err());
        assert!(parse("-32769", NumType::I16, InputBase::Dec, ByteOrder::Abcd).is_err());

        assert_eq!(
            fmt_dec_unsigned(&parse_ok(
                NumType::U16,
                "65535",
                InputBase::Dec,
                ByteOrder::Abcd
            ))
            .as_deref(),
            Some("65535")
        );
        assert!(parse("65536", NumType::U16, InputBase::Dec, ByteOrder::Abcd).is_err());
        assert_eq!(
            parse("-1", NumType::U16, InputBase::Dec, ByteOrder::Abcd).unwrap_err(),
            "无符号类型不支持负数"
        );

        assert_eq!(
            fmt_dec_unsigned(&parse_ok(
                NumType::U32,
                "4294967295",
                InputBase::Dec,
                ByteOrder::Abcd
            ))
            .as_deref(),
            Some("4294967295")
        );
        assert!(parse("4294967296", NumType::U32, InputBase::Dec, ByteOrder::Abcd).is_err());
        assert!(parse("2147483647", NumType::I32, InputBase::Dec, ByteOrder::Abcd).is_ok());
        assert!(parse("-2147483648", NumType::I32, InputBase::Dec, ByteOrder::Abcd).is_ok());
        assert!(parse("2147483648", NumType::I32, InputBase::Dec, ByteOrder::Abcd).is_err());

        assert!(parse("1.5", NumType::I16, InputBase::Dec, ByteOrder::Abcd).is_err());
    }

    #[test]
    fn test_float_dec_round_trip() {
        let cases = [
            (NumType::F32, "0.1"),
            (NumType::F32, "-1.5"),
            (NumType::F32, "1e10"),
            (NumType::F32, "1e-45"),
            (NumType::F64, "0.30000000000000004"),
        ];
        for (ty, s) in cases {
            let a = parse_ok(ty, s, InputBase::Dec, ByteOrder::Abcd);
            let text = fmt_float(&a).unwrap_or_else(|| panic!("{s} 没有浮点文本"));
            let b = parse_ok(ty, &text, InputBase::Dec, ByteOrder::Abcd);
            assert_eq!(a.bytes, b.bytes, "往返不一致：{s} -> {text}");
        }
    }

    #[test]
    fn test_float_special_values() {
        for (s, expect) in [
            ("NaN", "NaN"),
            ("nan", "NaN"),
            ("Inf", "Inf"),
            ("-Inf", "-Inf"),
        ] {
            let v = parse_ok(NumType::F32, s, InputBase::Dec, ByteOrder::Abcd);
            assert_eq!(fmt_float(&v).as_deref(), Some(expect));
        }
        let inf = parse_ok(NumType::F64, "-inf", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(fmt_float(&inf).as_deref(), Some("-Inf"));

        let zero = parse_ok(NumType::F32, "-0", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(fmt_float(&zero).as_deref(), Some("-0"));
        assert_eq!(canonical_bytes(&zero), &[0x80, 0x00, 0x00, 0x00]);

        let sub = parse_ok(NumType::F32, "1e-45", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(float_note(&sub), Some("非规范数"));
        assert_eq!(fmt_float(&sub).as_deref(), Some("1e-45"));
        assert_eq!(
            float_note(&parse_ok(
                NumType::F32,
                "1.5",
                InputBase::Dec,
                ByteOrder::Abcd
            )),
            None
        );
        assert_eq!(
            float_note(&parse_ok(
                NumType::U16,
                "1",
                InputBase::Dec,
                ByteOrder::Abcd
            )),
            None
        );

        let sub64 = parse_ok(NumType::F64, "5e-324", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(float_note(&sub64), Some("非规范数"));
    }

    #[test]
    fn test_float_bit_patterns() {
        assert_eq!(
            fmt_hex(
                &parse_ok(NumType::F32, "1.5", InputBase::Dec, ByteOrder::Abcd),
                ByteOrder::Abcd
            ),
            "3F C0 00 00"
        );
        assert_eq!(
            fmt_hex(
                &parse_ok(NumType::F32, "-1.5", InputBase::Dec, ByteOrder::Abcd),
                ByteOrder::Abcd
            ),
            "BF C0 00 00"
        );
        assert_eq!(
            fmt_hex(
                &parse_ok(NumType::F32, "0", InputBase::Dec, ByteOrder::Abcd),
                ByteOrder::Abcd
            ),
            "00 00 00 00"
        );

        let nan = parse_ok(NumType::F32, "0x7FC00000", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(fmt_float(&nan).as_deref(), Some("NaN"));
        let inf = parse_ok(NumType::F32, "0x7F800000", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(fmt_float(&inf).as_deref(), Some("Inf"));
        let min_sub = parse_ok(NumType::F32, "0x00000001", InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(fmt_float(&min_sub).as_deref(), Some("1e-45"));
        assert_eq!(float_note(&min_sub), Some("非规范数"));
    }

    #[test]
    fn test_int_views() {
        let m1 = parse_ok(NumType::I16, "-1", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(fmt_dec_signed(&m1).as_deref(), Some("-1"));
        assert_eq!(fmt_dec_unsigned(&m1).as_deref(), Some("65535"));
        assert_eq!(fmt_hex(&m1, ByteOrder::Abcd), "FF FF");
        assert_eq!(fmt_bin(&m1, ByteOrder::Abcd), "1111 1111 1111 1111");
        assert_eq!(fmt_oct(&m1).as_deref(), Some("177777"));

        let u = parse_ok(NumType::U32, "16909060", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(fmt_dec_unsigned(&u).as_deref(), Some("16909060"));
        assert_eq!(fmt_dec_signed(&u).as_deref(), Some("16909060"));
        assert_eq!(fmt_hex(&u, ByteOrder::Abcd), "01 02 03 04");
        assert_eq!(fmt_oct(&u).as_deref(), Some("100401404"));

        let f = parse_ok(NumType::F32, "1.5", InputBase::Dec, ByteOrder::Abcd);
        assert!(fmt_dec_signed(&f).is_none());
        assert!(fmt_dec_unsigned(&f).is_none());
        assert!(fmt_oct(&f).is_none());
    }

    #[test]
    fn test_byte_order_affects_only_bit_views() {
        let v = parse_ok(NumType::F32, "1.5", InputBase::Dec, ByteOrder::Dcba);
        assert_eq!(fmt_float(&v).as_deref(), Some("1.5"));
        assert_eq!(fmt_hex(&v, ByteOrder::Dcba), "00 00 C0 3F");

        let back = parse_ok(NumType::F32, "00 00 C0 3F", InputBase::Hex, ByteOrder::Dcba);
        assert_eq!(back.bytes, v.bytes);
        assert_eq!(fmt_float(&back).as_deref(), Some("1.5"));

        assert_eq!(fmt_hex(&v, ByteOrder::Badc), "C0 3F 00 00");
        let badc = parse_ok(NumType::F32, "C0 3F 00 00", InputBase::Hex, ByteOrder::Badc);
        assert_eq!(badc.bytes, v.bytes);
    }

    #[test]
    fn test_f64_word_swap() {
        let v = parse_ok(NumType::F64, "1.5", InputBase::Dec, ByteOrder::Cdab);
        assert_eq!(fmt_hex_compact(&v, ByteOrder::Cdab), "0000000000003FF8");
        let back = parse_ok(
            NumType::F64,
            "0000000000003FF8",
            InputBase::Hex,
            ByteOrder::Cdab,
        );
        assert_eq!(back.bytes, v.bytes);
        assert_eq!(fmt_float(&back).as_deref(), Some("1.5"));
    }

    #[test]
    fn test_hex_round_trip_all_types() {
        let samples = [
            (NumType::F32, "1.5"),
            (NumType::F64, "-1.5"),
            (NumType::U16, "65535"),
            (NumType::I16, "-1"),
            (NumType::U32, "4294967295"),
            (NumType::I32, "-1"),
        ];
        for (ty, s) in samples {
            let base = parse_ok(ty, s, InputBase::Dec, ByteOrder::Abcd);
            for order in ByteOrder::ALL {
                if !order.applicable(ty.width()) {
                    continue;
                }
                let hex = fmt_hex_compact(&base, order);
                let back = parse_ok(ty, &hex, InputBase::Hex, order);
                assert_eq!(
                    back.bytes,
                    base.bytes,
                    "{} {} 十六进制往返失败：{hex}",
                    ty.short(),
                    order.label()
                );

                let bin = fmt_bin_compact(&base, order);
                let back_bin = parse_ok(ty, &bin, InputBase::Bin, order);
                assert_eq!(
                    back_bin.bytes,
                    base.bytes,
                    "{} {} 二进制往返失败",
                    ty.short(),
                    order.label()
                );
            }
        }
    }

    #[test]
    fn test_format_input() {
        let f = parse_ok(NumType::F32, "1.5", InputBase::Dec, ByteOrder::Abcd);
        let hex = format_input(&f, InputBase::Hex, ByteOrder::Abcd).unwrap();
        assert_eq!(hex, "3F C0 00 00");
        let back = parse_ok(NumType::F32, &hex, InputBase::Hex, ByteOrder::Abcd);
        assert_eq!(
            format_input(&back, InputBase::Dec, ByteOrder::Abcd).as_deref(),
            Some("1.5")
        );

        let u = parse_ok(NumType::U16, "65535", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(
            format_input(&u, InputBase::Hex, ByteOrder::Abcd).as_deref(),
            Some("FF FF")
        );
        assert_eq!(
            format_input(&u, InputBase::Dec, ByteOrder::Abcd).as_deref(),
            Some("65535")
        );
        assert_eq!(
            format_input(&u, InputBase::Oct, ByteOrder::Abcd).as_deref(),
            Some("177777")
        );
        assert_eq!(
            format_input(&u, InputBase::Bin, ByteOrder::Abcd).as_deref(),
            Some("1111 1111 1111 1111")
        );

        let i = parse_ok(NumType::I16, "-1", InputBase::Dec, ByteOrder::Abcd);
        assert_eq!(
            format_input(&i, InputBase::Dec, ByteOrder::Abcd).as_deref(),
            Some("-1")
        );
    }

    #[test]
    fn test_empty_input_and_oct_unsupported() {
        assert_eq!(
            parse("", NumType::U16, InputBase::Dec, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("   ", NumType::F32, InputBase::Dec, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("", NumType::U16, InputBase::Bin, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );
        assert_eq!(
            parse("", NumType::U16, InputBase::Oct, ByteOrder::Abcd).unwrap_err(),
            "输入为空"
        );

        assert_eq!(
            InputBase::ALL.map(|b| b.label()),
            ["十进制", "十六进制", "二进制", "八进制"]
        );
        assert!(!InputBase::Oct.supported(NumType::F32));
        assert!(!InputBase::Oct.supported(NumType::F64));
        assert!(InputBase::Oct.supported(NumType::U16));
        assert!(InputBase::Oct.supported(NumType::I16));
        for ty in NumType::ALL {
            assert!(InputBase::Dec.supported(ty));
            assert!(InputBase::Hex.supported(ty));
            assert!(InputBase::Bin.supported(ty));
        }

        let f = parse_ok(NumType::F32, "1.5", InputBase::Dec, ByteOrder::Abcd);
        assert!(format_input(&f, InputBase::Oct, ByteOrder::Abcd).is_none());
        assert!(parse("0o17", NumType::F32, InputBase::Oct, ByteOrder::Abcd).is_err());
    }
}
