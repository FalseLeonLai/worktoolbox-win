#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    Text,
    Hex,
}

impl ValueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::U8 => "U8",
            Self::I16 => "I16",
            Self::U16 => "U16",
            Self::I32 => "I32",
            Self::U32 => "U32",
            Self::F32 => "F32",
            Self::Text => "文本（UTF-8）",
            Self::Hex => "十六进制",
        }
    }

    pub fn width(self) -> usize {
        match self {
            Self::U8 | Self::Text | Self::Hex => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    Abcd,
    Badc,
    Cdab,
    Dcba,
}

impl ByteOrder {
    pub fn label(self) -> &'static str {
        match self {
            Self::Abcd => "ABCD（大端）",
            Self::Badc => "BADC（字节交换）",
            Self::Cdab => "CDAB（字交换）",
            Self::Dcba => "DCBA（小端）",
        }
    }
}

pub fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut high = None;
    for character in text.chars().filter(|character| !character.is_whitespace()) {
        let digit = character
            .to_digit(16)
            .ok_or_else(|| format!("无效的十六进制字符：{character}"))? as u8;
        if let Some(nibble) = high.take() {
            bytes.push((nibble << 4) | digit);
        } else {
            high = Some(digit);
        }
    }
    if high.is_some() {
        return Err("十六进制字符数必须为偶数".to_owned());
    }
    Ok(bytes)
}

pub fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn encode_values(text: &str, kind: ValueKind, order: ByteOrder) -> Result<Vec<u8>, String> {
    match kind {
        ValueKind::Text => return Ok(text.as_bytes().to_vec()),
        ValueKind::Hex => return parse_hex(text),
        _ => {}
    }
    let mut bytes = Vec::new();
    for token in text
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|token| !token.is_empty())
    {
        let start = bytes.len();
        let invalid = || format!("“{token}”不是有效的 {} 数值或超出范围", kind.label());
        match kind {
            ValueKind::U8 => bytes.push(token.parse::<u8>().map_err(|_| invalid())?),
            ValueKind::I16 => {
                bytes.extend_from_slice(&token.parse::<i16>().map_err(|_| invalid())?.to_be_bytes())
            }
            ValueKind::U16 => {
                bytes.extend_from_slice(&token.parse::<u16>().map_err(|_| invalid())?.to_be_bytes())
            }
            ValueKind::I32 => {
                bytes.extend_from_slice(&token.parse::<i32>().map_err(|_| invalid())?.to_be_bytes())
            }
            ValueKind::U32 => {
                bytes.extend_from_slice(&token.parse::<u32>().map_err(|_| invalid())?.to_be_bytes())
            }
            ValueKind::F32 => {
                let value = token.parse::<f32>().map_err(|_| invalid())?;
                if !value.is_finite() {
                    return Err(format!("“{token}”不是有限的 F32 数值"));
                }
                bytes.extend_from_slice(&value.to_be_bytes());
            }
            ValueKind::Text | ValueKind::Hex => unreachable!(),
        }
        apply_byte_order(&mut bytes[start..], order);
    }
    Ok(bytes)
}

pub fn decode_values(bytes: &[u8], kind: ValueKind, order: ByteOrder) -> Result<String, String> {
    match kind {
        ValueKind::Text => {
            return std::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|_| "字节不是有效的 UTF-8 文本".to_owned());
        }
        ValueKind::Hex => return Ok(format_hex(bytes)),
        _ => {}
    }
    let width = kind.width();
    if !bytes.len().is_multiple_of(width) {
        return Err(format!(
            "字节长度 {} 不是 {} 所需的 {} 字节的整数倍",
            bytes.len(),
            kind.label(),
            width
        ));
    }
    let mut values = Vec::new();
    for chunk in bytes.chunks_exact(width) {
        let mut value = [0; 4];
        value[..width].copy_from_slice(chunk);
        apply_byte_order(&mut value[..width], order);
        values.push(match kind {
            ValueKind::U8 => value[0].to_string(),
            ValueKind::I16 => i16::from_be_bytes([value[0], value[1]]).to_string(),
            ValueKind::U16 => u16::from_be_bytes([value[0], value[1]]).to_string(),
            ValueKind::I32 => i32::from_be_bytes(value).to_string(),
            ValueKind::U32 => u32::from_be_bytes(value).to_string(),
            ValueKind::F32 => {
                let number = f32::from_be_bytes(value);
                if !number.is_finite() {
                    return Err("字节包含非有限的 F32 数值".to_owned());
                }
                number.to_string()
            }
            ValueKind::Text | ValueKind::Hex => unreachable!(),
        });
    }
    Ok(values.join(" "))
}

fn apply_byte_order(bytes: &mut [u8], order: ByteOrder) {
    match (bytes.len(), order) {
        (2, ByteOrder::Badc | ByteOrder::Dcba) => bytes.swap(0, 1),
        (4, ByteOrder::Badc) => {
            bytes.swap(0, 1);
            bytes.swap(2, 3);
        }
        (4, ByteOrder::Cdab) => bytes.rotate_left(2),
        (4, ByteOrder::Dcba) => bytes.reverse(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_widths_match_value_types() {
        for (kind, label, width) in [
            (ValueKind::U8, "U8", 1),
            (ValueKind::I16, "I16", 2),
            (ValueKind::U16, "U16", 2),
            (ValueKind::I32, "I32", 4),
            (ValueKind::U32, "U32", 4),
            (ValueKind::F32, "F32", 4),
            (ValueKind::Text, "文本（UTF-8）", 1),
            (ValueKind::Hex, "十六进制", 1),
        ] {
            assert_eq!(kind.label(), label);
            assert_eq!(kind.width(), width);
        }
        assert_eq!(ByteOrder::Abcd.label(), "ABCD（大端）");
        assert_eq!(ByteOrder::Badc.label(), "BADC（字节交换）");
        assert_eq!(ByteOrder::Cdab.label(), "CDAB（字交换）");
        assert_eq!(ByteOrder::Dcba.label(), "DCBA（小端）");
    }

    #[test]
    fn parses_contiguous_and_whitespace_separated_hex() {
        assert_eq!(parse_hex("01aBFF").unwrap(), [0x01, 0xAB, 0xFF]);
        assert_eq!(parse_hex(" 01\tAb\nff ").unwrap(), [0x01, 0xAB, 0xFF]);
        assert!(parse_hex(" \n").unwrap().is_empty());
    }

    #[test]
    fn rejects_odd_or_invalid_hex() {
        for input in ["0", "001", "GG", "0x12", "01,02", "你好", "ＡＢ"] {
            assert!(parse_hex(input).is_err(), "{input}");
        }
    }

    #[test]
    fn formats_uppercase_padded_hex() {
        assert_eq!(format_hex(&[0, 1, 0xAB, 0xFF]), "00 01 AB FF");
        assert_eq!(format_hex(&[]), "");
    }

    #[test]
    fn encodes_four_byte_orders_using_golden_bytes() {
        for (order, expected) in [
            (ByteOrder::Abcd, [1, 2, 3, 4]),
            (ByteOrder::Badc, [2, 1, 4, 3]),
            (ByteOrder::Cdab, [3, 4, 1, 2]),
            (ByteOrder::Dcba, [4, 3, 2, 1]),
        ] {
            assert_eq!(
                encode_values("16909060", ValueKind::U32, order).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn decodes_four_byte_orders_using_golden_bytes() {
        for (order, bytes) in [
            (ByteOrder::Abcd, [1, 2, 3, 4]),
            (ByteOrder::Badc, [2, 1, 4, 3]),
            (ByteOrder::Cdab, [3, 4, 1, 2]),
            (ByteOrder::Dcba, [4, 3, 2, 1]),
        ] {
            assert_eq!(
                decode_values(&bytes, ValueKind::U32, order).unwrap(),
                "16909060"
            );
        }
    }

    #[test]
    fn two_byte_orders_use_the_requested_endianness() {
        for (order, bytes) in [
            (ByteOrder::Abcd, [0x12, 0x34]),
            (ByteOrder::Badc, [0x34, 0x12]),
            (ByteOrder::Cdab, [0x12, 0x34]),
            (ByteOrder::Dcba, [0x34, 0x12]),
        ] {
            assert_eq!(encode_values("4660", ValueKind::U16, order).unwrap(), bytes);
            assert_eq!(
                decode_values(&bytes, ValueKind::U16, order).unwrap(),
                "4660"
            );
        }
    }

    #[test]
    fn preserves_signed_integer_golden_values() {
        assert_eq!(
            encode_values("-2,-32768 32767", ValueKind::I16, ByteOrder::Abcd).unwrap(),
            [0xFF, 0xFE, 0x80, 0, 0x7F, 0xFF]
        );
        assert_eq!(
            decode_values(
                &[0xFF, 0xFE, 0x80, 0, 0x7F, 0xFF],
                ValueKind::I16,
                ByteOrder::Abcd
            )
            .unwrap(),
            "-2 -32768 32767"
        );
        assert_eq!(
            encode_values("-2147483648 2147483647", ValueKind::I32, ByteOrder::Abcd).unwrap(),
            [0x80, 0, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF]
        );
        assert_eq!(
            decode_values(
                &[0x80, 0, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF],
                ValueKind::I32,
                ByteOrder::Abcd
            )
            .unwrap(),
            "-2147483648 2147483647"
        );
    }

    #[test]
    fn accepts_unsigned_limits_and_numeric_separators() {
        assert_eq!(
            encode_values("0, 255\n1", ValueKind::U8, ByteOrder::Dcba).unwrap(),
            [0, 255, 1]
        );
        assert_eq!(
            decode_values(&[0, 255, 1], ValueKind::U8, ByteOrder::Dcba).unwrap(),
            "0 255 1"
        );
        assert_eq!(
            encode_values("65535", ValueKind::U16, ByteOrder::Abcd).unwrap(),
            [0xFF; 2]
        );
        assert_eq!(
            encode_values("4294967295", ValueKind::U32, ByteOrder::Abcd).unwrap(),
            [0xFF; 4]
        );
    }

    #[test]
    fn rejects_invalid_or_out_of_range_integers() {
        for (kind, inputs) in [
            (ValueKind::U8, ["-1", "256", "1.5", "abc"]),
            (ValueKind::I16, ["-32769", "32768", "1.5", "abc"]),
            (ValueKind::U16, ["-1", "65536", "1.5", "abc"]),
            (ValueKind::I32, ["-2147483649", "2147483648", "1.5", "abc"]),
            (ValueKind::U32, ["-1", "4294967296", "1.5", "abc"]),
        ] {
            for input in inputs {
                assert!(
                    encode_values(input, kind, ByteOrder::Abcd).is_err(),
                    "{kind:?}: {input}"
                );
            }
        }
    }

    #[test]
    fn encodes_and_decodes_finite_float_golden_bytes() {
        assert_eq!(
            encode_values("-1.5,1e0", ValueKind::F32, ByteOrder::Abcd).unwrap(),
            [0xBF, 0xC0, 0, 0, 0x3F, 0x80, 0, 0]
        );
        assert_eq!(
            decode_values(
                &[0xBF, 0xC0, 0, 0, 0x3F, 0x80, 0, 0],
                ValueKind::F32,
                ByteOrder::Abcd
            )
            .unwrap(),
            "-1.5 1"
        );
        assert_eq!(
            encode_values("-1.5", ValueKind::F32, ByteOrder::Cdab).unwrap(),
            [0, 0, 0xBF, 0xC0]
        );
        assert_eq!(
            decode_values(&[0, 0, 0xBF, 0xC0], ValueKind::F32, ByteOrder::Cdab).unwrap(),
            "-1.5"
        );
    }

    #[test]
    fn rejects_non_finite_floats_in_both_directions() {
        for input in ["NaN", "inf", "-inf", "1e100", "abc"] {
            assert!(
                encode_values(input, ValueKind::F32, ByteOrder::Abcd).is_err(),
                "{input}"
            );
        }
        for bytes in [[0x7F, 0x80, 0, 0], [0xFF, 0x80, 0, 0], [0x7F, 0xC0, 0, 0]] {
            assert!(decode_values(&bytes, ValueKind::F32, ByteOrder::Abcd).is_err());
        }
    }

    #[test]
    fn text_preserves_utf8_and_whitespace_without_byte_reordering() {
        assert_eq!(
            encode_values(" 中\n", ValueKind::Text, ByteOrder::Dcba).unwrap(),
            [0x20, 0xE4, 0xB8, 0xAD, 0x0A]
        );
        assert_eq!(
            decode_values(
                &[0x20, 0xE4, 0xB8, 0xAD, 0x0A],
                ValueKind::Text,
                ByteOrder::Dcba
            )
            .unwrap(),
            " 中\n"
        );
    }

    #[test]
    fn text_rejects_invalid_or_truncated_utf8() {
        for bytes in [&[0xFF][..], &[0xE4, 0xB8][..]] {
            assert!(decode_values(bytes, ValueKind::Text, ByteOrder::Abcd).is_err());
        }
    }

    #[test]
    fn hex_values_preserve_wire_order() {
        assert_eq!(
            encode_values("01aB ff", ValueKind::Hex, ByteOrder::Dcba).unwrap(),
            [1, 0xAB, 0xFF]
        );
        assert_eq!(
            decode_values(&[1, 0xAB, 0xFF], ValueKind::Hex, ByteOrder::Dcba).unwrap(),
            "01 AB FF"
        );
        assert!(encode_values("GG", ValueKind::Hex, ByteOrder::Abcd).is_err());
    }

    #[test]
    fn rejects_incomplete_numeric_values_with_chinese_errors() {
        for kind in [
            ValueKind::I16,
            ValueKind::U16,
            ValueKind::I32,
            ValueKind::U32,
            ValueKind::F32,
        ] {
            let error = decode_values(&[1], kind, ByteOrder::Abcd).unwrap_err();
            assert!(error.contains("字节长度"), "{error}");
        }
    }

    #[test]
    fn accepts_empty_values() {
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
            assert!(encode_values("", kind, ByteOrder::Abcd).unwrap().is_empty());
            assert_eq!(decode_values(&[], kind, ByteOrder::Abcd).unwrap(), "");
        }
    }
}
