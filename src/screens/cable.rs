//! 线缆选型计算：输入负载与线路条件 → 载流量选型 / 温度·并排校正 / 电压降校验 /
//! 空开配合 / 电缆型号建议。标准数据（GB·IEC 与 UL·NEC）编译期内嵌。

use crate::data::cable_engine::{calculate, load_standard, CalcInput, CalcResult, StandardsFile};
use crate::theme::*;
use eframe::egui;

const GB_JSON: &str = include_str!("../../data/standards/gb_iec.json");
const UL_JSON: &str = include_str!("../../data/standards/ul.json");

pub struct CableScreen {
    standards: Vec<Result<StandardsFile, String>>,
    sel_std: usize,
    input: InputState,
}

struct InputState {
    material: String,
    insulation: String,
    method: String,
    phases: u8,
    voltage: f64,
    load_kw: f64,
    power_factor: f64,
    length_m: f64,
    ambient_c: f64,
    grouped: f64,
    vd_limit_pct: f64,
    load_is_motor: bool,
}

impl InputState {
    fn to_calc(&self) -> CalcInput {
        CalcInput {
            material: self.material.clone(),
            insulation: self.insulation.clone(),
            method: self.method.clone(),
            phases: self.phases,
            voltage: self.voltage,
            load_kw: self.load_kw,
            power_factor: self.power_factor,
            length_m: self.length_m,
            ambient_c: self.ambient_c,
            grouped_circuits: self.grouped,
            vd_limit_pct: self.vd_limit_pct,
            load_is_motor: self.load_is_motor,
        }
    }
}

impl CableScreen {
    pub fn new() -> Self {
        Self {
            standards: vec![load_standard(GB_JSON), load_standard(UL_JSON)],
            sel_std: 0,
            input: InputState {
                material: "cu".into(),
                insulation: "pvc".into(),
                method: String::new(),
                phases: 3,
                voltage: 380.0,
                load_kw: 10.0,
                power_factor: 0.85,
                length_m: 50.0,
                ambient_c: 30.0,
                grouped: 1.0,
                vd_limit_pct: 5.0,
                load_is_motor: false,
            },
        }
    }
}

/// 所选的 材料/绝缘/敷设方式 若不在当前标准中，回落到第一项
fn fix_selections(std: &StandardsFile, inp: &mut InputState) {
    if !std.materials.iter().any(|m| m.id == inp.material) {
        if let Some(m) = std.materials.first() {
            inp.material = m.id.clone();
        }
    }
    if !std.insulations.iter().any(|m| m.id == inp.insulation) {
        if let Some(m) = std.insulations.first() {
            inp.insulation = m.id.clone();
        }
    }
    if !std.methods.iter().any(|m| m.id == inp.method) {
        if let Some(m) = std.methods.first() {
            inp.method = m.id.clone();
        }
    }
}

pub fn show(s: &mut CableScreen, ui: &mut egui::Ui) {
    // 标准选择
    let labels: Vec<String> = s
        .standards
        .iter()
        .enumerate()
        .map(|(i, st)| match st {
            Ok(f) => f.label.clone(),
            Err(_) => format!("标准 {}", i + 1),
        })
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(|x| x.as_str()).collect();
    if let Some(i) = crate::ui::segmented(ui, &label_refs, s.sel_std) {
        s.sel_std = i;
    }
    ui.add_space(10.0);

    let sel = s.sel_std.min(s.standards.len() - 1);
    let std = match &s.standards[sel] {
        Ok(f) => f,
        Err(e) => {
            ui.label(egui::RichText::new(format!("标准数据解析失败：{e}")).color(AMBER));
            return;
        }
    };
    fix_selections(std, &mut s.input);

    ui.horizontal_top(|ui| {
        // —— 左：输入 ——
        ui.vertical(|ui| {
            ui.set_width(375.0);
            egui::ScrollArea::vertical()
                .id_salt("cable_input")
                .auto_shrink([false, false])
                .show(ui, |ui| form(ui, std, &mut s.input));
        });
        ui.separator();
        // —— 右：结果 ——
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("cable_out")
                .auto_shrink([false, false])
                .show(ui, |ui| match calculate(std, &s.input.to_calc()) {
                    Ok(r) => result_view(ui, &s.input, &r),
                    Err(e) => {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("无法计算：{e}"))
                                .color(AMBER)
                                .size(15.0),
                        );
                    }
                });
        });
    });
}

fn chip(ui: &mut egui::Ui, active: bool, text: &str) -> bool {
    crate::ui::chip(ui, active, text)
}

fn muted(t: &str) -> egui::RichText {
    egui::RichText::new(t).color(MUTED).size(13.0)
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    crate::ui::section(ui, title);
    ui.add_space(4.0);
    crate::ui::card(ui, body);
    ui.add_space(10.0);
}

fn combo_row(ui: &mut egui::Ui, label: &str, options: &[(String, String)], val: &mut String) {
    ui.horizontal(|ui| {
        ui.add_sized([112.0, 18.0], egui::Label::new(muted(label)));
        let cur = options
            .iter()
            .find(|(id, _)| id == val)
            .map(|(_, l)| l.clone())
            .unwrap_or_else(|| val.clone());
        egui::ComboBox::from_id_salt(label)
            .width(210.0)
            .selected_text(cur)
            .show_ui(ui, |ui| {
                for (id, l) in options {
                    ui.selectable_value(val, id.clone(), l);
                }
            });
    });
    ui.add_space(3.0);
}

fn drag_row(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    unit: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([112.0, 18.0], egui::Label::new(muted(label)));
        ui.add(egui::DragValue::new(val).speed(speed).range(range));
        if !unit.is_empty() {
            ui.label(egui::RichText::new(unit).color(MUTED).size(12.5));
        }
    });
    ui.add_space(3.0);
}

fn form(ui: &mut egui::Ui, std: &StandardsFile, inp: &mut InputState) {
    card(ui, "负载与电源", |ui| {
        let m_opts: Vec<(String, String)> = std
            .materials
            .iter()
            .map(|m| (m.id.clone(), m.label.clone()))
            .collect();
        let i_opts: Vec<(String, String)> = std
            .insulations
            .iter()
            .map(|m| {
                let label = if m.label.contains('℃') {
                    m.label.clone()
                } else {
                    format!("{}（{}℃）", m.label, m.max_temp_c)
                };
                (m.id.clone(), label)
            })
            .collect();
        let w_opts: Vec<(String, String)> = std
            .methods
            .iter()
            .map(|m| (m.id.clone(), m.label.clone()))
            .collect();
        combo_row(ui, "导体材料", &m_opts, &mut inp.material);
        combo_row(ui, "绝缘类型", &i_opts, &mut inp.insulation);
        combo_row(ui, "敷设方式", &w_opts, &mut inp.method);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 18.0], egui::Label::new(muted("相数")));
            let sel = if inp.phases == 3 { 0 } else { 1 };
            if let Some(i) = crate::ui::segmented(ui, &["三相", "单相"], sel) {
                if i == 0 {
                    inp.phases = 3;
                    inp.voltage = 380.0;
                } else {
                    inp.phases = 1;
                    inp.voltage = 220.0;
                }
            }
        });
        ui.add_space(3.0);
        drag_row(ui, "电压 U", &mut inp.voltage, 1.0, 100.0..=1500.0, "V");
        drag_row(ui, "负载功率", &mut inp.load_kw, 0.5, 0.1..=3000.0, "kW");
        drag_row(ui, "功率因数", &mut inp.power_factor, 0.01, 0.3..=1.0, "");
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 18.0], egui::Label::new(muted("负载类型")));
            ui.checkbox(&mut inp.load_is_motor, "电动机（直接启动）");
        });
    });
    card(ui, "线路条件", |ui| {
        drag_row(ui, "线路长度", &mut inp.length_m, 1.0, 0.5..=3000.0, "m");
        drag_row(ui, "环境温度", &mut inp.ambient_c, 1.0, -10.0..=70.0, "℃");
        drag_row(ui, "并排回路数", &mut inp.grouped, 1.0, 1.0..=30.0, "回");
        drag_row(ui, "允许压降", &mut inp.vd_limit_pct, 0.5, 0.5..=15.0, "%");
    });
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(format!("数据来源：{}", std.source_note))
            .color(MUTED)
            .size(11.5),
    );
}

fn stat(ui: &mut egui::Ui, k: &str, v: &str, col: egui::Color32) {
    crate::ui::stat(ui, k, v, col);
}

fn kv_line(ui: &mut egui::Ui, k: &str, v: &str, col: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{k}："))
                .color(FAINT_TEXT)
                .size(13.0),
        );
        ui.label(egui::RichText::new(v).color(col).size(13.5));
    });
}

fn result_view(ui: &mut egui::Ui, inp: &InputState, r: &CalcResult) {
    // 汇总数字
    ui.horizontal(|ui| {
        stat(ui, "计算电流 Ib", &format!("{:.1} A", r.load_current_a), TEXT);
        ui.add_space(22.0);
        stat(ui, "温度校正", &format!("×{:.2}", r.derate_temp), MUTED);
        ui.add_space(22.0);
        stat(ui, "并排校正", &format!("×{:.2}", r.derate_group), MUTED);
    });
    ui.add_space(12.0);

    if let Some(c) = &r.selected {
        egui::Frame::new()
            .fill(CARD)
            .stroke(border(20))
            .corner_radius(12)
            .inner_margin(16)
            .show(ui, |ui| {
                crate::ui::section(ui, "推荐方案");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        crate::theme::b(&c.size_label)
                            .color(OK_GREEN)
                            .size(28.0),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "载流量 {} A · 校正后 {:.1} A",
                            c.ampacity_base, c.ampacity_corrected
                        ))
                        .color(TEXT)
                        .size(14.0),
                    );
                });
                ui.add_space(6.0);
                kv_line(
                    ui,
                    "配套空开",
                    &format!("≤ {:.0} A 档（In ≤ Iz 配合）", c.breaker_a),
                    TEXT,
                );
                let vd_ok = c.vd_pct <= inp.vd_limit_pct + 1e-9;
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("电压降：")
                            .color(FAINT_TEXT)
                            .size(13.0),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.1} V（{:.2}%）· 允许 ≤ {}%",
                            c.vd_v,
                            c.vd_pct,
                            fmt_limit(inp.vd_limit_pct)
                        ))
                        .color(TEXT)
                        .size(13.5),
                    );
                    crate::ui::pill(
                        ui,
                        if vd_ok { "满足" } else { "超限" },
                        if vd_ok { OK_GREEN } else { AMBER },
                    );
                });
            });
        ui.add_space(12.0);
    } else {
        ui.label(
            egui::RichText::new("⚠ 在当前条件下未能选出合适截面（见下方说明）")
                .color(AMBER)
                .size(15.0),
        );
        ui.add_space(8.0);
    }

    if !r.alternatives.is_empty() {
        crate::ui::section(ui, "备选截面（更大 1~3 档）");
        ui.add_space(2.0);
        egui::Grid::new("cable_alt")
            .num_columns(4)
            .spacing([18.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for h in ["截面", "载流量（校正后）", "空开", "压降"] {
                    ui.label(egui::RichText::new(h).color(MUTED).size(12.5));
                }
                ui.end_row();
                for c in &r.alternatives {
                    ui.label(egui::RichText::new(&c.size_label).size(13.5));
                    ui.label(
                        egui::RichText::new(format!("{:.1} A", c.ampacity_corrected)).size(13.0),
                    );
                    ui.label(egui::RichText::new(format!("{:.0} A", c.breaker_a)).size(13.0));
                    ui.label(egui::RichText::new(format!("{:.2}%", c.vd_pct)).size(13.0));
                    ui.end_row();
                }
            });
        ui.add_space(12.0);
    }

    if !r.suggested_types.is_empty() {
        crate::ui::section(ui, "可选电缆型号");
        ui.add_space(3.0);
        for t in &r.suggested_types {
            ui.label(egui::RichText::new(format!("· {t}")).color(TEXT).size(13.5));
            ui.add_space(2.0);
        }
        ui.add_space(10.0);
    }

    for w in &r.warnings {
        ui.label(egui::RichText::new(format!("⚠ {w}")).color(AMBER).size(13.5));
        ui.add_space(3.0);
    }
    if !r.notes.is_empty() {
        ui.add_space(2.0);
        for n in &r.notes {
            ui.label(egui::RichText::new(format!("· {n}")).color(MUTED).size(12.5));
            ui.add_space(2.0);
        }
    }
}

fn fmt_limit(v: f64) -> String {
    let mut s = format!("{v}");
    if !s.contains('.') {
        s.push_str(".0");
    }
    s
}
