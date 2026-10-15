//! 线缆选型计算引擎（多标准：GB/IEC 与 UL）。
//! 纯计算逻辑，无 UI；标准数据来自 `data/standards/*.json`。
//!
//! 计算流程（与界面输入 `CalcInput` 对应）：
//! 1. 设计电流 Ib：三相 Ib = P / (√3·U·cosφ)；单相 Ib = P / (U·cosφ)；
//! 2. 校正系数 k = k_temp · k_group（查表线性内插，越界取端点并给出提示）；
//! 3. 选截面：对应载流量表中「表值 × k ≥ Ib」的最小截面；
//! 4. 校验断路器：选「≥Ib 且 ≤表值×k」的最小标准档，不存在则给出警告；
//! 5. 电压降：三相 ΔU = √3·Ib·L·(R·cosφ + X·sinφ)；单相 ΔU = 2·Ib·L·(R·cosφ + X·sinφ)，
//!    其中 R = ρ·1000 / mm²（Ω/km），X 取默认值 0.08 Ω/km；
//!    超过限值时逐档放大截面，直到满足或达到表内最大截面。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardsFile {
    pub standard_id: String, // "gb_iec" | "ul"
    pub label: String,       // 显示名（如：国标 GB / IEC·EN（CE））
    pub source_note: String, // 数据来源与节选说明
    pub materials: Vec<NamedItem>,
    pub insulations: Vec<InsulationItem>,
    pub methods: Vec<NamedItem>,
    pub sizes: Vec<SizeItem>,
    pub ampacity: Vec<AmpacityTable>,
    pub temp_correction: Vec<TempCorrection>,
    pub grouping_correction: Vec<GroupingCorrection>,
    pub breaker_sizes: Vec<f64>,
    pub cable_types: Vec<CableTypeHint>,
    pub vd_limits: VdLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamedItem {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsulationItem {
    pub id: String,
    pub label: String,
    pub max_temp_c: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SizeItem {
    pub label: String, // 如 "2.5 mm²" 或 "10 AWG"
    pub mm2: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AmpacityTable {
    pub method: String,
    pub material: String,
    pub insulation: String,
    /// 按截面升序
    pub points: Vec<AmpacityPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AmpacityPoint {
    pub size: String,
    pub mm2: f64,
    pub amps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TempCorrection {
    pub insulation: String,
    /// (环境温度 °C, 校正系数)
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupingCorrection {
    /// (并排回路数, 校正系数)
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CableTypeHint {
    pub insulation: String,
    pub method: Option<String>,
    pub example_types: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VdLimits {
    pub general_pct: f64,
    pub lighting_pct: f64,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct CalcInput {
    pub material: String,      // "cu" | "al"
    pub insulation: String,    // "pvc" | "xlpe"（GB）/ "60c" | "75c" | "90c"（UL）
    pub method: String,
    pub phases: u8,            // 1 | 3
    pub voltage: f64,          // V
    pub load_kw: f64,
    pub power_factor: f64,     // 0.1..1
    pub length_m: f64,
    pub ambient_c: f64,
    pub grouped_circuits: f64, // 1 = 单回路
    pub vd_limit_pct: f64,     // 允许电压降 %
    pub load_is_motor: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Candidate {
    pub size_label: String,
    pub mm2: f64,
    pub ampacity_base: f64,
    pub ampacity_corrected: f64,
    pub breaker_a: f64,
    pub vd_pct: f64,
    pub vd_v: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CalcResult {
    pub load_current_a: f64,
    pub derate_temp: f64,
    pub derate_group: f64,
    pub selected: Option<Candidate>,
    pub alternatives: Vec<Candidate>, // 更大 1~3 档
    pub suggested_types: Vec<String>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// 数值工具
// ---------------------------------------------------------------------------

/// 在 (x, y) 点列上线性内插；越界时取端点值，并给出说明文本。
fn interp(points: &[(f64, f64)], x: f64, what: &str) -> Result<(f64, Option<String>), String> {
    if points.is_empty() {
        return Err(format!("{what}：校正表数据为空"));
    }
    let mut pts: Vec<(f64, f64)> = points.to_vec();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let (x0, y0) = pts[0];
    let (xn, yn) = pts[pts.len() - 1];

    if x <= x0 {
        let note = if x < x0 {
            Some(format!(
                "{what} {:.1} 低于表内最小值 {:.1}，已按端点系数 {:.3} 取值",
                x, x0, y0
            ))
        } else {
            None
        };
        return Ok((y0, note));
    }
    if x >= xn {
        let note = if x > xn {
            Some(format!(
                "{what} {:.1} 高于表内最大值 {:.1}，已按端点系数 {:.3} 取值",
                x, xn, yn
            ))
        } else {
            None
        };
        return Ok((yn, note));
    }
    for w in pts.windows(2) {
        let (xa, ya) = w[0];
        let (xb, yb) = w[1];
        if x >= xa && x <= xb {
            let t = if (xb - xa).abs() < f64::EPSILON {
                0.0
            } else {
                (x - xa) / (xb - xa)
            };
            return Ok((ya + (yb - ya) * t, None));
        }
    }
    Ok((yn, None))
}

/// 选断路器：标准档位中「≥ Ib 且 ≤ Iz」的最小档；不存在时返回最小可用档 + 警告。
fn pick_breaker(sizes: &[f64], ib: f64, iz: f64) -> (f64, Option<String>) {
    let eps = 1e-9;
    let mut up = f64::INFINITY;
    let mut fit = f64::INFINITY;
    for &b in sizes {
        if b + eps >= ib {
            up = up.min(b);
            if b <= iz + eps {
                fit = fit.min(b);
            }
        }
    }
    if !up.is_finite() {
        return (
            0.0,
            Some(format!(
                "无 ≥ 设计电流 {:.1} A 的标准保护档位，请增大截面或分回路",
                ib
            )),
        );
    }
    if fit.is_finite() {
        (fit, None)
    } else {
        (
            up,
            Some(format!(
                "该截面下不满足 Ib ≤ In ≤ Iz：最小可用档 {:.0} A 大于校正后载流量 {:.1} A，建议放大截面或改善散热条件",
                up, iz
            )),
        )
    }
}

// ---------------------------------------------------------------------------
// 公开 API
// ---------------------------------------------------------------------------

/// 解析标准数据 JSON，并做基本的完整性检查。
pub fn load_standard(json_text: &str) -> Result<StandardsFile, String> {
    let std: StandardsFile =
        serde_json::from_str(json_text).map_err(|e| format!("标准数据 JSON 解析失败：{e}"))?;
    if std.standard_id.trim().is_empty() {
        return Err("标准数据缺少 standard_id 字段".to_string());
    }
    if std.ampacity.is_empty() {
        return Err(format!("标准 {} 缺少载流量表（ampacity）", std.standard_id));
    }
    if std.breaker_sizes.is_empty() {
        return Err(format!(
            "标准 {} 缺少断路器档位（breaker_sizes）",
            std.standard_id
        ));
    }
    if std.temp_correction.is_empty() {
        return Err(format!(
            "标准 {} 缺少环境温度校正表（temp_correction）",
            std.standard_id
        ));
    }
    if std.grouping_correction.is_empty() {
        return Err(format!(
            "标准 {} 缺少多回路校正表（grouping_correction）",
            std.standard_id
        ));
    }
    Ok(std)
}

/// 按工况计算选型结果（截面 + 断路器 + 电压降 + 备选与说明）。
pub fn calculate(std: &StandardsFile, input: &CalcInput) -> Result<CalcResult, String> {
    // ---------- 输入校验 ----------
    if input.phases != 1 && input.phases != 3 {
        return Err(format!("相数仅支持 1 或 3，当前为 {}", input.phases));
    }
    if !(input.voltage > 0.0) {
        return Err("电压必须大于 0".to_string());
    }
    if !(input.load_kw > 0.0) {
        return Err("负载功率必须大于 0".to_string());
    }
    if !(input.power_factor > 0.0 && input.power_factor <= 1.0) {
        return Err("功率因数应在 (0, 1] 范围内".to_string());
    }
    if input.length_m < 0.0 {
        return Err("线路长度不能为负数".to_string());
    }
    if !(input.vd_limit_pct > 0.0) {
        return Err("允许电压降限值必须大于 0".to_string());
    }
    if input.grouped_circuits < 1.0 {
        return Err("并排回路数不能小于 1".to_string());
    }

    // ---------- 材料 / 绝缘 / 敷设方式检查 ----------
    if !std.materials.iter().any(|m| m.id == input.material) {
        let ids: Vec<&str> = std.materials.iter().map(|m| m.id.as_str()).collect();
        return Err(format!(
            "标准 {} 不支持导体材料 “{}”（可选：{}）",
            std.standard_id,
            input.material,
            ids.join(" / ")
        ));
    }
    if !std.insulations.iter().any(|i| i.id == input.insulation) {
        let ids: Vec<&str> = std.insulations.iter().map(|i| i.id.as_str()).collect();
        return Err(format!(
            "标准 {} 不支持绝缘类型 “{}”（可选：{}）",
            std.standard_id,
            input.insulation,
            ids.join(" / ")
        ));
    }
    if !std.methods.iter().any(|m| m.id == input.method) {
        let ids: Vec<&str> = std.methods.iter().map(|m| m.id.as_str()).collect();
        return Err(format!(
            "标准 {} 不支持敷设方式 “{}”（可选：{}）",
            std.standard_id,
            input.method,
            ids.join(" / ")
        ));
    }
    // 导体电阻率（20 °C，Ω·mm²/m）：铜 0.0175、铝 0.0282
    let rho = match input.material.as_str() {
        "cu" | "copper" => 0.0175,
        "al" | "aluminium" | "aluminum" => 0.0282,
        other => {
            return Err(format!(
                "未定义导体材料的电阻率：{}（请在引擎中补充该材料的 ρ 值）",
                other
            ));
        }
    };

    let mut notes: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // ---------- 设计电流 Ib ----------
    let three_phase = input.phases == 3;
    let p_w = input.load_kw * 1000.0;
    let load_current = if three_phase {
        p_w / (3.0_f64.sqrt() * input.voltage * input.power_factor)
    } else {
        p_w / (input.voltage * input.power_factor)
    };
    let formula = if three_phase {
        "三相 Ib = P / (√3·U·cosφ)"
    } else {
        "单相 Ib = P / (U·cosφ)"
    };
    notes.push(format!(
        "设计电流 Ib = {:.2} A（{}，P = {:.3} kW，U = {:.1} V，cosφ = {:.2}）",
        load_current, formula, input.load_kw, input.voltage, input.power_factor
    ));

    // ---------- 校正系数 k = k_temp · k_group ----------
    let tc = std
        .temp_correction
        .iter()
        .find(|t| t.insulation == input.insulation)
        .ok_or_else(|| {
            format!(
                "标准 {} 缺少绝缘类型 “{}” 的环境温度校正表",
                std.standard_id, input.insulation
            )
        })?;
    let (k_temp, temp_note) = interp(&tc.points, input.ambient_c, "环境温度")?;
    if let Some(n) = temp_note {
        notes.push(n);
    }

    // 多回路校正采用数据文件中的第一张表：
    // GB/IEC = B.52.17 项1（成束，保守取法）；UL = NEC 310.15(C)(1)（按每回路 3 根载流导体折算）
    let gc = std
        .grouping_correction
        .first()
        .ok_or_else(|| format!("标准 {} 缺少多回路校正表", std.standard_id))?;
    let (k_group, group_note) = interp(&gc.points, input.grouped_circuits, "并排回路数")?;
    if let Some(n) = group_note {
        notes.push(n);
    }

    let k = k_temp * k_group;
    notes.push(format!(
        "校正系数：温度 k1 = {:.3}（环境 {:.1} °C，基准 30 °C），多回路 k2 = {:.3}（{:.0} 回路并排），综合 k = {:.3}",
        k_temp, input.ambient_c, k_group, input.grouped_circuits, k
    ));

    // ---------- 查找载流量表 ----------
    let table = std
        .ampacity
        .iter()
        .find(|t| {
            t.method == input.method && t.material == input.material && t.insulation == input.insulation
        })
        .ok_or_else(|| {
            format!(
                "标准 {} 缺少载流量表组合：材料 “{}” × 绝缘 “{}” × 敷设 “{}”；请更换组合或补充数据",
                std.standard_id, input.material, input.insulation, input.method
            )
        })?;
    let mut pts = table.points.clone();
    pts.sort_by(|a, b| a.mm2.partial_cmp(&b.mm2).unwrap_or(std::cmp::Ordering::Equal));
    if pts.is_empty() {
        return Err(format!(
            "载流量表（材料 {} / 绝缘 {} / 敷设 {}）数据点为空",
            input.material, input.insulation, input.method
        ));
    }

    // ---------- 选截面：最小的 表值×k ≥ Ib ----------
    let sel_idx = pts
        .iter()
        .position(|p| p.amps * k + 1e-9 >= load_current)
        .ok_or_else(|| {
            let max = pts.last().unwrap();
            format!(
                "设计电流 {:.1} A 超出表内最大截面 {} 的承载能力（校正后 {:.1} A）；请减小负载、增加回路或改善敷设散热条件",
                load_current,
                max.size,
                max.amps * k
            )
        })?;

    // ---------- 电压降 ----------
    let sin_phi = (1.0 - input.power_factor * input.power_factor).max(0.0).sqrt();
    let x_ohm_km = 0.08; // 默认感抗，单位 Ω/km
    let l_km = input.length_m / 1000.0;
    let vd_v_at = |mm2: f64| -> f64 {
        let r_km = rho * 1000.0 / mm2.max(1e-9);
        let z = r_km * input.power_factor + x_ohm_km * sin_phi;
        if three_phase {
            3.0_f64.sqrt() * load_current * l_km * z
        } else {
            2.0 * load_current * l_km * z
        }
    };
    let limit_v = input.vd_limit_pct / 100.0 * input.voltage;

    // 超限则逐档放大截面
    let mut final_idx = sel_idx;
    let mut vd_v = vd_v_at(pts[final_idx].mm2);
    while vd_v > limit_v && final_idx + 1 < pts.len() {
        final_idx += 1;
        vd_v = vd_v_at(pts[final_idx].mm2);
    }
    if final_idx > sel_idx {
        notes.push(format!(
            "电压降校验：{} 时 ΔU ≈ {:.1} V（{:.2}%）超过限值 {:.1}%（{:.1} V），已逐档放大到 {}",
            pts[sel_idx].size,
            vd_v_at(pts[sel_idx].mm2),
            vd_v_at(pts[sel_idx].mm2) / input.voltage * 100.0,
            input.vd_limit_pct,
            limit_v,
            pts[final_idx].size
        ));
    }
    if vd_v > limit_v {
        warnings.push(format!(
            "已取表内最大截面 {}，电压降 {:.2}%（限值 {:.1}%）仍不满足；请缩短线路、并联电缆或提高电压等级",
            pts[final_idx].size,
            vd_v / input.voltage * 100.0,
            input.vd_limit_pct
        ));
    }
    notes.push(format!(
        "电压降 ΔU = {:.1} V（{:.2}%）：R = ρ·L/A（铜 0.0175 / 铝 0.0282 Ω·mm²/m @20 °C），X = {:.2} Ω/km（默认假设值，未区分敷设方式）",
        vd_v,
        vd_v / input.voltage * 100.0,
        x_ohm_km
    ));
    notes.push(
        "电阻按导体 20 °C 直流值估算，未计入运行温升（约 +20%）、集肤/邻近效应与谐波影响；精确设计请使用电缆厂家数据或 IEC 60287 计算。"
            .to_string(),
    );

    // ---------- 候选结果 ----------
    let make_candidate = |i: usize| -> Candidate {
        let p = &pts[i];
        let amps_corrected = p.amps * k;
        let (breaker, _warn) = pick_breaker(&std.breaker_sizes, load_current, amps_corrected);
        let v = vd_v_at(p.mm2);
        Candidate {
            size_label: p.size.clone(),
            mm2: p.mm2,
            ampacity_base: p.amps,
            ampacity_corrected: amps_corrected,
            breaker_a: breaker,
            vd_pct: v / input.voltage * 100.0,
            vd_v: v,
        }
    };

    let selected = make_candidate(final_idx);
    let (_, sel_warn) = pick_breaker(
        &std.breaker_sizes,
        load_current,
        selected.ampacity_corrected,
    );
    if let Some(w) = sel_warn {
        warnings.push(format!("选中截面 {}：{}", selected.size_label, w));
    }

    // 更大 1~3 档作为备选（含各自压降）
    let alternatives: Vec<Candidate> = (final_idx + 1..pts.len())
        .take(3)
        .map(make_candidate)
        .collect();

    // ---------- 建议电缆型号 ----------
    let fmt_hint = |t: &CableTypeHint| -> String {
        if t.note.trim().is_empty() {
            t.example_types.clone()
        } else {
            format!("{}（{}）", t.example_types, t.note)
        }
    };
    let mut suggested: Vec<String> = std
        .cable_types
        .iter()
        .filter(|t| {
            t.insulation == input.insulation
                && t.method.as_deref().map_or(true, |m| m == input.method)
        })
        .map(fmt_hint)
        .collect();
    if suggested.is_empty() {
        // 敷设方式无精确匹配时放宽到同绝缘类型的全部提示
        suggested = std
            .cable_types
            .iter()
            .filter(|t| t.insulation == input.insulation)
            .map(fmt_hint)
            .collect();
    }

    // ---------- 其他说明 ----------
    if input.phases == 1 && std.standard_id.starts_with("gb") {
        notes.push(
            "本载流量表按三根载流导体（三相）给值；用于单相（两根载流导体）回路时结果偏保守、留有余量。"
                .to_string(),
        );
    }
    if input.load_is_motor {
        notes.push(
            "电动机负载：选型按额定电流进行；直接起动的启动电流可达额定电流 6~7 倍，起动瞬间的电压降（一般要求 ≤ 15%）与接触器/热继电器整定需另行校验。"
                .to_string(),
        );
        notes.push(
            "变频器（VFD）供电时：建议采用屏蔽电缆，注意谐波电流（尤其 3 次）导致的附加发热与中性线电流，必要时按厂家手册增大截面（可参考 IEC 60364-5-52 附录 E）。"
                .to_string(),
        );
    }

    Ok(CalcResult {
        load_current_a: load_current,
        derate_temp: k_temp,
        derate_group: k_group,
        selected: Some(selected),
        alternatives,
        suggested_types: suggested,
        notes,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB_JSON: &str = include_str!("../../data/standards/gb_iec.json");
    const UL_JSON: &str = include_str!("../../data/standards/ul.json");

    fn input_gb() -> CalcInput {
        CalcInput {
            material: "cu".to_string(),
            insulation: "pvc".to_string(),
            method: "B1".to_string(),
            phases: 3,
            voltage: 380.0,
            load_kw: 10.0,
            power_factor: 0.85,
            length_m: 30.0,
            ambient_c: 30.0,
            grouped_circuits: 1.0,
            vd_limit_pct: 5.0,
            load_is_motor: false,
        }
    }

    #[test]
    fn gb_json_parses_and_baseline_values() {
        let std = load_standard(GB_JSON).expect("gb_iec.json 应可解析");
        assert_eq!(std.standard_id, "gb_iec");
        assert_eq!(std.ampacity.len(), 12, "应为 2 材料 × 2 绝缘 × 3 敷设 = 12 张表");
        assert_eq!(std.sizes.len(), 15);
        assert!(std.breaker_sizes.contains(&16.0) && std.breaker_sizes.contains(&400.0));

        // 基准值：IEC 方法 C 铜 PVC 2.5 mm² = 24 A（三根载流导体表）
        let t = std
            .ampacity
            .iter()
            .find(|t| t.method == "C" && t.material == "cu" && t.insulation == "pvc")
            .expect("应存在 C/cu/pvc 表");
        let p = t
            .points
            .iter()
            .find(|p| (p.mm2 - 2.5).abs() < 1e-9)
            .expect("应存在 2.5 mm² 数据点");
        assert!((p.amps - 24.0).abs() < 1e-9);

        // 基准值：方法 E 铜 XLPE 10 mm² = 75 A
        let t2 = std
            .ampacity
            .iter()
            .find(|t| t.method == "E" && t.material == "cu" && t.insulation == "xlpe")
            .unwrap();
        let p2 = t2.points.iter().find(|p| (p.mm2 - 10.0).abs() < 1e-9).unwrap();
        assert!((p2.amps - 75.0).abs() < 1e-9);
    }

    #[test]
    fn ul_json_parses_and_baseline_values() {
        let std = load_standard(UL_JSON).expect("ul.json 应可解析");
        assert_eq!(std.standard_id, "ul");
        assert_eq!(std.ampacity.len(), 6, "应为 2 材料 × 3 温度列 = 6 张表");

        // 基准值：NEC 310.16 铜 75 °C：12 AWG=25 A、10 AWG=35 A、8 AWG=50 A
        let t = std
            .ampacity
            .iter()
            .find(|t| t.material == "cu" && t.insulation == "75c")
            .unwrap();
        let get = |lbl: &str| -> f64 {
            t.points
                .iter()
                .find(|p| p.size == lbl)
                .unwrap_or_else(|| panic!("缺少 {lbl}"))
                .amps
        };
        assert!((get("12 AWG") - 25.0).abs() < 1e-9);
        assert!((get("10 AWG") - 35.0).abs() < 1e-9);
        assert!((get("8 AWG") - 50.0).abs() < 1e-9);

        // 温度校正：90 °C 列 50 °C 时 0.82；多回路：4 回路 → 0.50
        let tc = std.temp_correction.iter().find(|t| t.insulation == "90c").unwrap();
        let (k, _) = interp(&tc.points, 50.0, "环境温度").unwrap();
        assert!((k - 0.82).abs() < 1e-9);
        let (kg, _) = interp(&std.grouping_correction[0].points, 4.0, "并排回路数").unwrap();
        assert!((kg - 0.50).abs() < 1e-9);
    }

    #[test]
    fn gb_three_phase_selection() {
        let std = load_standard(GB_JSON).unwrap();
        let r = calculate(&std, &input_gb()).expect("10 kW/380 V 三相应可计算");
        // Ib = 10000 / (√3 × 380 × 0.85) ≈ 17.9 A
        assert!((r.load_current_a - 17.87).abs() < 0.1, "Ib={}", r.load_current_a);
        assert!((r.derate_temp - 1.0).abs() < 1e-9);
        assert!((r.derate_group - 1.0).abs() < 1e-9);

        let sel = r.selected.clone().expect("应选出截面");
        // B1 铜 PVC：1.5 mm² = 15.5 A < 17.9 A，2.5 mm² = 21 A ≥ 17.9 A
        assert!((sel.mm2 - 2.5).abs() < 1e-9, "选中截面应为 2.5 mm²");
        assert!((sel.ampacity_corrected - 21.0).abs() < 1e-9);
        // 断路器：≥17.9 A 且 ≤21 A 的最小档 = 20 A
        assert!((sel.breaker_a - 20.0).abs() < 1e-9);
        assert!(sel.vd_pct < 5.0);
        assert!(r.warnings.is_empty(), "不应有警告：{:?}", r.warnings);
        assert!(!r.notes.is_empty());
        assert!(!r.suggested_types.is_empty());
        assert!(r.alternatives.len() >= 1);
    }

    #[test]
    fn gb_single_phase_temp_derate() {
        let std = load_standard(GB_JSON).unwrap();
        let input = CalcInput {
            material: "cu".to_string(),
            insulation: "pvc".to_string(),
            method: "C".to_string(),
            phases: 1,
            voltage: 220.0,
            load_kw: 3.0,
            power_factor: 0.9,
            length_m: 20.0,
            ambient_c: 35.0,
            grouped_circuits: 1.0,
            vd_limit_pct: 5.0,
            load_is_motor: true,
        };
        let r = calculate(&std, &input).expect("单相应可计算");
        // Ib = 3000 / (220 × 0.9) ≈ 15.15 A；35 °C 时 PVC 系数 0.94
        assert!((r.load_current_a - 15.15).abs() < 0.05);
        assert!((r.derate_temp - 0.94).abs() < 1e-9);
        let sel = r.selected.unwrap();
        // C 铜 PVC 1.5 mm² = 17.5 × 0.94 = 16.45 A ≥ 15.15 A
        assert!((sel.mm2 - 1.5).abs() < 1e-9, "选中截面应为 1.5 mm²");
        assert!((sel.breaker_a - 16.0).abs() < 1e-9);
        // 电动机提示应出现在 notes 中
        assert!(r.notes.iter().any(|n| n.contains("电动机")), "应提示电动机启动事项");
    }

    #[test]
    fn gb_grouping_and_vd_enlarge() {
        let std = load_standard(GB_JSON).unwrap();
        let mut input = input_gb();
        input.load_kw = 35.0;
        input.power_factor = 0.8;
        input.length_m = 300.0;
        let r = calculate(&std, &input).expect("长线路应可计算");
        // Ib = 35000 / (√3 × 380 × 0.8) ≈ 66.5 A；B1 铜 PVC 16 mm² = 68 A ≥ 66.5 A
        assert!((r.load_current_a - 66.5).abs() < 0.2);
        let sel = r.selected.clone().unwrap();
        // 16 mm² 压降 ≈ 31.9 V（8.4%）> 19 V 限值 → 逐档放大至 35 mm²（≈15.5 V，4.1%）
        assert!((sel.mm2 - 35.0).abs() < 1e-9, "应因压降放大到 35 mm²，实得 {}", sel.mm2);
        assert!(sel.vd_pct <= 5.0);
        assert!(sel.vd_pct > 3.5);
        assert!(r.notes.iter().any(|n| n.contains("逐档放大")), "应记录压降放大过程");
        // 备选：更大 1~3 档 → 50 / 70 / 95 mm²
        assert_eq!(r.alternatives.len(), 3);
        assert!((r.alternatives[0].mm2 - 50.0).abs() < 1e-9);

        // 多回路校正：3 回路成束 k = 0.70
        let mut input2 = input_gb();
        input2.grouped_circuits = 3.0;
        let r2 = calculate(&std, &input2).unwrap();
        assert!((r2.derate_group - 0.70).abs() < 1e-9);
        // 2.5 mm² 校正后 21×0.7=14.7 < 17.87 → 4 mm²（28×0.7=19.6 ≥ 17.87）
        assert!((r2.selected.unwrap().mm2 - 4.0).abs() < 1e-9);
    }

    #[test]
    fn ul_three_phase_selection() {
        let std = load_standard(UL_JSON).unwrap();
        let input = CalcInput {
            material: "cu".to_string(),
            insulation: "75c".to_string(),
            method: "raceway".to_string(),
            phases: 3,
            voltage: 480.0,
            load_kw: 30.0,
            power_factor: 0.9,
            length_m: 50.0,
            ambient_c: 30.0,
            grouped_circuits: 1.0,
            vd_limit_pct: 3.0,
            load_is_motor: false,
        };
        let r = calculate(&std, &input).expect("UL 工况应可计算");
        // Ib = 30000 / (√3 × 480 × 0.9) ≈ 40.1 A
        assert!((r.load_current_a - 40.09).abs() < 0.1);
        let sel = r.selected.unwrap();
        // 75 °C 铜：10 AWG=35 < 40.1，8 AWG=50 ≥ 40.1 → 8 AWG；断路器 45 A
        assert!((sel.mm2 - 8.367).abs() < 0.05, "应为 8 AWG（≈8.37 mm²），实得 {}", sel.mm2);
        assert!((sel.breaker_a - 45.0).abs() < 1e-9);
        assert!(sel.vd_pct < 3.0);
    }

    #[test]
    fn errors_are_reported() {
        // 非法 JSON
        assert!(load_standard("not json").is_err());
        // 字段缺失（空对象）
        assert!(load_standard("{}").is_err());

        let std = load_standard(GB_JSON).unwrap();
        // 未知敷设方式
        let mut input = input_gb();
        input.method = "D".to_string();
        assert!(calculate(&std, &input).is_err());
        // 未知绝缘
        let mut input2 = input_gb();
        input2.insulation = "epr".to_string();
        assert!(calculate(&std, &input2).is_err());
        // 超出表内最大截面
        let mut input3 = input_gb();
        input3.load_kw = 1000.0;
        let err = calculate(&std, &input3).unwrap_err();
        assert!(err.contains("超出"), "错误信息应说明超出范围：{err}");
        // 非法参数
        let mut input4 = input_gb();
        input4.power_factor = 0.0;
        assert!(calculate(&std, &input4).is_err());
        let mut input5 = input_gb();
        input5.phases = 2;
        assert!(calculate(&std, &input5).is_err());
    }
}
