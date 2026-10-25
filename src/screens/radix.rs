//! 进制转换：IEEE 754 浮点 / 整数 · 二 / 八 / 十 / 十六进制互转，支持字节序。

use crate::data::radix::{self, ByteOrder, InputBase, NumType, Value};
use crate::theme::*;
use eframe::egui;

pub struct RadixScreen {
    ty: NumType,
    base: InputBase,
    order: ByteOrder,
    input: String,
}

impl RadixScreen {
    pub fn new() -> Self {
        Self {
            ty: NumType::F32,
            base: InputBase::Hex,
            order: ByteOrder::Abcd,
            input: String::new(),
        }
    }
}

pub fn show(s: &mut RadixScreen, ui: &mut egui::Ui) {
    crate::ui::page_head(ui, "进制转换", "IEEE754 浮点：十 / 十六 / 二进制 · 整数：二 / 八 / 十 / 十六进制");

    // —— 类型 / 格式 / 输入 / 字节序 ——
    crate::ui::card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("数值类型").color(muted()).size(13.0));
            let items: Vec<&str> = NumType::ALL.iter().map(|t| t.short()).collect();
            let current = NumType::ALL.iter().position(|t| *t == s.ty).unwrap_or(0);
            if let Some(i) = crate::ui::segmented(ui, &items, current) {
                switch_type(s, NumType::ALL[i]);
            }
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("输入格式").color(muted()).size(13.0));
            let bases: Vec<InputBase> = InputBase::ALL
                .iter()
                .copied()
                .filter(|b| b.supported(s.ty))
                .collect();
            let labels: Vec<&str> = bases.iter().map(|b| b.label()).collect();
            let current = bases.iter().position(|b| *b == s.base).unwrap_or(0);
            if let Some(i) = crate::ui::segmented(ui, &labels, current) {
                switch_base(s, bases[i]);
            }
        });

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("输入值").color(muted()).size(13.0));
            ui.add(
                egui::TextEdit::singleline(&mut s.input)
                    .hint_text(hint_text(s.ty, s.base))
                    .desired_width(360.0)
                    .font(egui::TextStyle::Monospace),
            );
            ui.add_space(10.0);
            ui.label(egui::RichText::new("字节序").color(muted()).size(13.0));
            let width = s.ty.width();
            egui::ComboBox::from_id_salt("radix_order")
                .selected_text(s.order.label())
                .width(180.0)
                .show_ui(ui, |ui| {
                    for order in ByteOrder::ALL {
                        if order.applicable(width) {
                            ui.selectable_value(&mut s.order, order, order.label())
                                .on_hover_text(order.hover_text(width));
                        }
                    }
                });
        });
    });

    ui.add_space(10.0);

    // —— 每帧解析一次 ——
    let result = if s.input.trim().is_empty() {
        Ok(None)
    } else {
        radix::parse(&s.input, s.ty, s.base, s.order).map(Some)
    };

    // —— 结果卡 ——
    crate::ui::card(ui, |ui| {
        crate::ui::section(ui, "转换结果");
        match &result {
            Err(err) => {
                ui.colored_label(danger(), format!("解析失败：{err}"));
            }
            Ok(value) => {
                egui::Grid::new("radix_results")
                    .num_columns(3)
                    .spacing([20.0, 7.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (label, display, copy) in result_rows(s.ty, *value, s.order) {
                            ui.label(egui::RichText::new(label).color(muted()).size(12.5));
                            ui.label(m(display).color(text()).size(13.5));
                            let enabled = copy.is_some();
                            let btn = ui.add_enabled(
                                enabled,
                                egui::Button::new(egui::RichText::new("复制").size(12.0)),
                            );
                            if btn.clicked() {
                                if let Some(text) = copy {
                                    ui.ctx().copy_text(text);
                                }
                            }
                            ui.end_row();
                        }
                    });
            }
        }
    });
}

/// 切换类型：保留输入文本重新解释；新类型不支持当前格式时退回十六进制，并归一化字节序。
fn switch_type(s: &mut RadixScreen, new_ty: NumType) {
    if new_ty == s.ty {
        return;
    }
    s.ty = new_ty;
    if !s.base.supported(new_ty) {
        s.base = InputBase::Hex;
    }
    s.order = s.order.normalized(new_ty.width());
}

/// 切换输入格式：当前输入可解析时按新格式回填；否则只切换格式。
fn switch_base(s: &mut RadixScreen, new_base: InputBase) {
    if new_base == s.base {
        return;
    }
    if let Ok(v) = radix::parse(&s.input, s.ty, s.base, s.order) {
        if let Some(text) = radix::format_input(&v, new_base, s.order) {
            s.input = text;
        }
    }
    s.base = new_base;
}

/// 输入提示：随类型（含 PLC 名称与取值范围）与格式变化。
fn hint_text(ty: NumType, base: InputBase) -> String {
    match base {
        InputBase::Dec => {
            if ty.is_float() {
                format!("{} 十进制浮点数，如 1.5 / -0.25 / 3.14e-5", ty.label())
            } else if ty.is_signed_int() {
                let (min, max) = if ty.width() == 2 {
                    (i16::MIN as i64, i16::MAX as i64)
                } else {
                    (i32::MIN as i64, i32::MAX as i64)
                };
                format!("{} 十进制有符号整数（{min} ~ {max}）", ty.label())
            } else {
                let max = if ty.width() == 2 {
                    u16::MAX as u64
                } else {
                    u32::MAX as u64
                };
                format!("{} 十进制无符号整数（0 ~ {max}）", ty.label())
            }
        }
        InputBase::Hex => {
            let sample = match ty.width() {
                2 => "FF FF / 0xFFFF",
                4 => "3F C0 00 00 / 0x3FC00000",
                _ => "3F F8 00 00 00 00 00 00",
            };
            format!("{} 十六进制位模式，如 {sample}", ty.label())
        }
        InputBase::Bin => format!("{} 二进制位模式；可用空格分组 / 0b 前缀", ty.label()),
        InputBase::Oct => format!("{} 八进制数值，可加 0o 前缀", ty.label()),
    }
}

/// 结果行：标签 / 展示文本 / 复制文本（空输入时展示「—」，复制按钮禁用）。
fn result_rows(
    ty: NumType,
    value: Option<Value>,
    order: ByteOrder,
) -> Vec<(&'static str, String, Option<String>)> {
    let mut rows: Vec<(&'static str, String, Option<String>)> = Vec::with_capacity(5);
    let mut push = |label: &'static str, display: Option<String>, copy: Option<String>| {
        rows.push((label, display.unwrap_or_else(|| "—".to_string()), copy));
    };

    if ty.is_float() {
        let text = value.and_then(|v| radix::fmt_float(&v));
        let display = match (&text, value.and_then(|v| radix::float_note(&v))) {
            (Some(t), Some(note)) => Some(format!("{t}（{note}）")),
            _ => text.clone(),
        };
        push("十进制（浮点）", display, text);
        push(
            "十六进制位模式",
            value.map(|v| radix::fmt_hex(&v, order)),
            value.map(|v| radix::fmt_hex_compact(&v, order)),
        );
        push(
            "二进制位模式",
            value.map(|v| radix::fmt_bin(&v, order)),
            value.map(|v| radix::fmt_bin_compact(&v, order)),
        );
        return rows;
    }

    let signed = ty.is_signed_int();
    let (first_label, second_label) = if signed {
        ("十进制（有符号）", "十进制（无符号重解释）")
    } else {
        ("十进制（无符号）", "十进制（有符号重解释）")
    };
    let (first, second) = if signed {
        (
            value.and_then(|v| radix::fmt_dec_signed(&v)),
            value.and_then(|v| radix::fmt_dec_unsigned(&v)),
        )
    } else {
        (
            value.and_then(|v| radix::fmt_dec_unsigned(&v)),
            value.and_then(|v| radix::fmt_dec_signed(&v)),
        )
    };
    push(first_label, first.clone(), first);
    push(second_label, second.clone(), second);
    push(
        "十六进制位模式",
        value.map(|v| radix::fmt_hex(&v, order)),
        value.map(|v| radix::fmt_hex_compact(&v, order)),
    );
    push(
        "二进制位模式",
        value.map(|v| radix::fmt_bin(&v, order)),
        value.map(|v| radix::fmt_bin_compact(&v, order)),
    );
    let oct = value.and_then(|v| radix::fmt_oct(&v));
    push("八进制", oct.clone(), oct);
    rows
}
