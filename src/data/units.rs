//! 单位换算数据表与换算逻辑（工程 / 电气语境）。
//!
//! 换算模型：`base = value * factor + offset`，即先把原单位的数值换算到本类别的
//! 基准单位，再从基准单位换算到目标单位。除温度类外，全部单位的 offset 均为 0，
//! 只用 factor 表示比例关系。
//!
//! 主要系数取值依据（尽量采用国际精确定义值 / 工程习惯值）：
//! - 长度：1 in = 25.4 mm、1 ft = 12 in、1 yd = 3 ft、1 mi = 1760 yd、1 nmi = 1852 m；
//! - 质量：1 lb = 0.45359237 kg、1 oz = 1/16 lb、1 格令 = 64.79891 mg；
//! - 力：标准重力加速度 g0 = 9.80665 m/s²，故 1 kgf = 9.80665 N、
//!   1 lbf = 1 lb × g0 = 4.4482216152605 N；
//! - 压力：1 bar = 100000 Pa、1 psi = 6894.757293 Pa、1 kgf/cm² = 98066.5 Pa、
//!   1 atm = 101325 Pa、1 Torr = atm / 760、1 毫米汞柱 ≈ 133.322387415 Pa；
//! - 功能：1 卡 = 4.1868 J（国际蒸汽表卡）、1 BTU = 1055.05585262 J；
//! - 马力：公制马力 1 PS = 735.49875 W、英制马力 1 hp = 745.699872 W（分开收录）；
//! - 线规：AWG 各档按标准线规表取标称截面积（mm²），如 AWG10 = 5.261 mm²。
//!
//! 本文件只提供数据与纯函数，不依赖界面层。

/// 单个单位定义。
pub struct Unit {
    pub id: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    /// 换算到基准单位：base = value * factor + offset
    pub factor: f64,
    pub offset: f64,
}

/// 一个换算类别（长度 / 压力 / 功率 / …）。
pub struct Category {
    #[allow(dead_code)]
    pub id: &'static str,
    pub name: &'static str,
    pub units: Vec<Unit>,
}

// —— 常用精确常数（国际定义值或工程习惯值） ——

/// 1 英寸 = 0.0254 米（精确定义）。
const INCH_M: f64 = 0.0254;
/// 1 英尺 = 0.3048 米。
const FOOT_M: f64 = 0.3048;
/// 1 码 = 0.9144 米。
const YARD_M: f64 = 0.9144;
/// 1 英里 = 1609.344 米。
const MILE_M: f64 = 1609.344;
/// 1 海里 = 1852 米。
const NMILE_M: f64 = 1852.0;
/// 1 磅 = 0.45359237 千克（精确定义）。
const LB_KG: f64 = 0.45359237;
/// 1 格令 = 64.79891 毫克。
const GRAIN_KG: f64 = 0.00006479891;
/// 标准重力加速度 g0 = 9.80665 m/s²。
const G0: f64 = 9.80665;
/// 1 磅力 = 1 lb × g0 = 4.4482216152605 牛。
const LBF_N: f64 = LB_KG * G0;
/// 1 巴 = 100000 帕。
const BAR_PA: f64 = 1.0e5;
/// 1 磅力/平方英寸 ≈ 6894.757293 帕。
const PSI_PA: f64 = 6894.757293;
/// 1 公斤力/平方厘米 = 98066.5 帕（即工程大气压）。
const KGF_CM2_PA: f64 = 98066.5;
/// 标准大气压 = 101325 帕。
const ATM_PA: f64 = 101325.0;
/// 1 毫米汞柱 ≈ 133.322387415 帕。
const MMHG_PA: f64 = 133.322387415;
/// 1 卡 = 4.1868 焦（国际蒸汽表卡）。
const CAL_J: f64 = 4.1868;
/// 1 英热单位 = 1055.05585262 焦。
const BTU_J: f64 = 1055.05585262;
/// 公制马力 = 735.49875 瓦。
const PS_W: f64 = 735.49875;
/// 英制（机械）马力 = 745.699872 瓦。
const HP_W: f64 = 745.699872;
/// 1 弧度 = 180/π ≈ 57.29577951308232 度。
const DEG_PER_RAD: f64 = 57.29577951308232;

/// 构造“纯比例单位”（offset = 0，用于除温度外的全部类别）。
fn u(id: &'static str, name: &'static str, symbol: &'static str, factor: f64) -> Unit {
    Unit { id, name, symbol, factor, offset: 0.0 }
}

/// 构造带偏移的单位（仅温度类使用，例如 °F、K）。
fn uo(id: &'static str, name: &'static str, symbol: &'static str, factor: f64, offset: f64) -> Unit {
    Unit { id, name, symbol, factor, offset }
}

/// 全部换算类别（长度 / 面积 / 压力 / 温度 / 功率 / …）。
pub fn categories() -> Vec<Category> {
    vec![
        // —— 长度（基准单位：米 m） ——
        Category {
            id: "length",
            name: "长度",
            units: vec![
                u("m", "米", "m", 1.0),
                u("km", "千米（公里）", "km", 1000.0),
                u("cm", "厘米", "cm", 0.01),
                u("mm", "毫米", "mm", 0.001),
                u("um", "微米", "μm", 1.0e-6),
                u("silk", "丝（道，1 丝 = 0.01 mm）", "丝", 1.0e-5),
                u("mil", "密耳（千分之一英寸）", "mil", INCH_M / 1000.0),
                u("in", "英寸", "in", INCH_M),
                u("ft", "英尺", "ft", FOOT_M),
                u("yd", "码", "yd", YARD_M),
                u("mi", "英里", "mi", MILE_M),
                u("nmi", "海里", "nmi", NMILE_M),
                u("chi", "市尺（1 米 = 3 市尺）", "尺", 1.0 / 3.0),
                u("cun", "市寸（1 市尺 = 10 市寸）", "寸", 1.0 / 30.0),
            ],
        },
        // —— 面积（基准单位：平方米 m²） ——
        Category {
            id: "area",
            name: "面积",
            units: vec![
                u("m2", "平方米", "m²", 1.0),
                u("km2", "平方千米", "km²", 1.0e6),
                u("cm2", "平方厘米", "cm²", 1.0e-4),
                u("mm2", "平方毫米", "mm²", 1.0e-6),
                u("ha", "公顷", "ha", 1.0e4),
                u("mu", "亩（1 公顷 = 15 亩）", "亩", 10000.0 / 15.0),
                u("mi2", "平方英里", "mi²", MILE_M * MILE_M),
                u("acre", "英亩", "acre", 4046.8564224),
                u("yd2", "平方码", "yd²", YARD_M * YARD_M),
                u("ft2", "平方英尺", "ft²", FOOT_M * FOOT_M),
                u("in2", "平方英寸", "in²", INCH_M * INCH_M),
            ],
        },
        // —— 体积 / 容积（基准单位：立方米 m³） ——
        Category {
            id: "volume",
            name: "体积（容积）",
            units: vec![
                u("m3", "立方米", "m³", 1.0),
                u("l", "升", "L", 0.001),
                u("ml", "毫升", "mL", 1.0e-6),
                u("cm3", "立方厘米", "cm³", 1.0e-6),
                u("yd3", "立方码", "yd³", YARD_M * YARD_M * YARD_M),
                u("ft3", "立方英尺", "ft³", FOOT_M * FOOT_M * FOOT_M),
                u("in3", "立方英寸", "in³", INCH_M * INCH_M * INCH_M),
                u("gal_us", "加仑（美制）", "gal(US)", 0.003785411784),
                u("gal_uk", "加仑（英制）", "gal(UK)", 0.00454609),
                u("qt_us", "夸脱（美制）", "qt(US)", 0.000946352946),
                u("pt_us", "品脱（美制）", "pt(US)", 0.000473176473),
                u("bbl", "石油桶（1 bbl = 42 gal(US)）", "bbl", 0.158987294928),
            ],
        },
        // —— 质量（基准单位：千克 kg） ——
        Category {
            id: "mass",
            name: "质量",
            units: vec![
                u("kg", "千克（公斤）", "kg", 1.0),
                u("g", "克", "g", 0.001),
                u("mg", "毫克", "mg", 1.0e-6),
                u("t", "吨", "t", 1000.0),
                u("lb", "磅", "lb", LB_KG),
                u("oz", "盎司", "oz", LB_KG / 16.0),
                u("stone", "英石", "st", LB_KG * 14.0),
                u("lton", "长吨（英制，2240 lb）", "long tn", LB_KG * 2240.0),
                u("ston", "短吨（美制，2000 lb）", "short tn", LB_KG * 2000.0),
                u("grain", "格令", "gr", GRAIN_KG),
                u("jin", "市斤", "斤", 0.5),
                u("liang", "市两", "两", 0.05),
            ],
        },
        // —— 温度（基准单位：摄氏度 °C；唯一使用 offset 的类别） ——
        //
        // 换算到基准：°C = value × factor + offset
        //   °F  → °C：°C = °F × 5/9 − 17.7777777778
        //   K   → °C：°C = K − 273.15
        //   °R  → °C：°C = °R × 5/9 − 273.15
        //   °Ré → °C：°C = °Ré × 1.25
        Category {
            id: "temperature",
            name: "温度",
            units: vec![
                uo("c", "摄氏度", "°C", 1.0, 0.0),
                uo("f", "华氏度", "°F", 5.0 / 9.0, -17.7777777778),
                uo("k", "开尔文", "K", 1.0, -273.15),
                uo("r", "兰氏度", "°R", 5.0 / 9.0, -273.15),
                uo("re", "列氏度", "°Ré", 1.25, 0.0),
            ],
        },
        // —— 压力（基准单位：帕斯卡 Pa） ——
        Category {
            id: "pressure",
            name: "压力",
            units: vec![
                u("pa", "帕斯卡", "Pa", 1.0),
                u("kpa", "千帕", "kPa", 1000.0),
                u("mpa", "兆帕", "MPa", 1.0e6),
                u("bar", "巴", "bar", BAR_PA),
                u("mbar", "毫巴", "mbar", 100.0),
                u("psi", "磅力每平方英寸", "psi", PSI_PA),
                u("kgf_cm2", "公斤力每平方厘米（工程大气压）", "kgf/cm²", KGF_CM2_PA),
                u("atm", "标准大气压", "atm", ATM_PA),
                u("mmhg", "毫米汞柱", "mmHg", MMHG_PA),
                u("inhg", "英寸汞柱", "inHg", MMHG_PA * 25.4),
                u("mmh2o", "毫米水柱", "mmH2O", G0),
                u("torr", "托", "Torr", ATM_PA / 760.0),
            ],
        },
        // —— 力（基准单位：牛顿 N） ——
        Category {
            id: "force",
            name: "力",
            units: vec![
                u("n", "牛顿", "N", 1.0),
                u("kn", "千牛", "kN", 1000.0),
                u("kgf", "公斤力", "kgf", G0),
                u("tf", "吨力", "tf", G0 * 1000.0),
                u("lbf", "磅力", "lbf", LBF_N),
                u("ozf", "盎司力", "ozf", LBF_N / 16.0),
                u("dyn", "达因", "dyn", 1.0e-5),
                u("pdl", "磅达", "pdl", 0.138254954376),
            ],
        },
        // —— 力矩 / 扭矩（基准单位：牛顿米 N·m） ——
        Category {
            id: "torque",
            name: "力矩",
            units: vec![
                u("n_m", "牛顿米", "N·m", 1.0),
                u("kn_m", "千牛米", "kN·m", 1000.0),
                u("kgf_m", "公斤力米", "kgf·m", G0),
                u("kgf_cm", "公斤力厘米", "kgf·cm", G0 / 100.0),
                u("lbf_ft", "磅力英尺", "lbf·ft", LBF_N * FOOT_M),
                u("lbf_in", "磅力英寸", "lbf·in", LBF_N * INCH_M),
                u("ozf_in", "盎司力英寸", "ozf·in", LBF_N / 16.0 * INCH_M),
                u("dyn_cm", "达因厘米", "dyn·cm", 1.0e-7),
            ],
        },
        // —— 功率（基准单位：瓦特 W） ——
        Category {
            id: "power",
            name: "功率",
            units: vec![
                u("w", "瓦特", "W", 1.0),
                u("kw", "千瓦", "kW", 1000.0),
                u("mw", "兆瓦", "MW", 1.0e6),
                u("ps", "公制马力", "PS", PS_W),
                u("hp", "英制马力", "hp", HP_W),
                u("cal_s", "卡每秒", "cal/s", CAL_J),
                u("kcal_h", "千卡每小时", "kcal/h", CAL_J * 1000.0 / 3600.0),
                u("ftlb_s", "英尺磅每秒", "ft·lb/s", LBF_N * FOOT_M),
                u("btu_h", "英热单位每小时", "BTU/h", BTU_J / 3600.0),
            ],
        },
        // —— 能量 / 功（基准单位：焦耳 J） ——
        Category {
            id: "energy",
            name: "能量",
            units: vec![
                u("j", "焦耳", "J", 1.0),
                u("kj", "千焦", "kJ", 1000.0),
                u("mj", "兆焦", "MJ", 1.0e6),
                u("wh", "瓦时", "W·h", 3600.0),
                u("kwh", "千瓦时（度）", "kW·h", 3.6e6),
                u("psh", "公制马力时", "PS·h", PS_W * 3600.0),
                u("cal", "卡（国际蒸汽表卡）", "cal", CAL_J),
                u("kcal", "千卡（大卡）", "kcal", CAL_J * 1000.0),
                u("btu", "英热单位", "BTU", BTU_J),
                u("ftlb", "英尺磅", "ft·lb", LBF_N * FOOT_M),
                u("erg", "尔格", "erg", 1.0e-7),
                u("ev", "电子伏", "eV", 1.602176634e-19),
            ],
        },
        // —— 电流（基准单位：安培 A） ——
        Category {
            id: "current",
            name: "电流",
            units: vec![
                u("a", "安培", "A", 1.0),
                u("ma", "毫安", "mA", 0.001),
                u("ua", "微安", "μA", 1.0e-6),
                u("na", "纳安", "nA", 1.0e-9),
                u("ka", "千安", "kA", 1000.0),
            ],
        },
        // —— 电压（基准单位：伏特 V） ——
        Category {
            id: "voltage",
            name: "电压",
            units: vec![
                u("v", "伏特", "V", 1.0),
                u("mv", "毫伏", "mV", 0.001),
                u("uv", "微伏", "μV", 1.0e-6),
                u("kv", "千伏", "kV", 1000.0),
                u("mega_v", "兆伏", "MV", 1.0e6),
            ],
        },
        // —— 电阻（基准单位：欧姆 Ω） ——
        Category {
            id: "resistance",
            name: "电阻",
            units: vec![
                u("ohm", "欧姆", "Ω", 1.0),
                u("mohm", "毫欧", "mΩ", 0.001),
                u("uohm", "微欧", "μΩ", 1.0e-6),
                u("kohm", "千欧", "kΩ", 1000.0),
                u("mega_ohm", "兆欧", "MΩ", 1.0e6),
            ],
        },
        // —— 电容（基准单位：法拉 F） ——
        Category {
            id: "capacitance",
            name: "电容",
            units: vec![
                u("f", "法拉", "F", 1.0),
                u("mf", "毫法", "mF", 0.001),
                u("uf", "微法", "μF", 1.0e-6),
                u("nf", "纳法", "nF", 1.0e-9),
                u("pf", "皮法", "pF", 1.0e-12),
            ],
        },
        // —— 电感（基准单位：亨利 H） ——
        Category {
            id: "inductance",
            name: "电感",
            units: vec![
                u("h", "亨利", "H", 1.0),
                u("mh", "毫亨", "mH", 0.001),
                u("uh", "微亨", "μH", 1.0e-6),
                u("nh", "纳亨", "nH", 1.0e-9),
                u("ph", "皮亨", "pH", 1.0e-12),
            ],
        },
        // —— 频率（基准单位：赫兹 Hz） ——
        Category {
            id: "frequency",
            name: "频率",
            units: vec![
                u("hz", "赫兹", "Hz", 1.0),
                u("khz", "千赫", "kHz", 1000.0),
                u("mhz", "兆赫", "MHz", 1.0e6),
                u("ghz", "吉赫", "GHz", 1.0e9),
                u("thz", "太赫", "THz", 1.0e12),
            ],
        },
        // —— 转速（基准单位：转每分 r/min） ——
        Category {
            id: "rotational_speed",
            name: "转速",
            units: vec![
                u("rpm", "转每分", "r/min", 1.0),
                u("rps", "转每秒", "r/s", 60.0),
                u("rad_s", "弧度每秒", "rad/s", 60.0 / (2.0 * std::f64::consts::PI)),
                u("deg_s", "度每秒", "°/s", 1.0 / 6.0),
                u("rph", "转每时", "r/h", 1.0 / 60.0),
            ],
        },
        // —— 速度（基准单位：米每秒 m/s） ——
        Category {
            id: "speed",
            name: "速度",
            units: vec![
                u("m_s", "米每秒", "m/s", 1.0),
                u("km_h", "千米每小时", "km/h", 1.0 / 3.6),
                u("mph", "英里每小时", "mph", MILE_M / 3600.0),
                u("ft_s", "英尺每秒", "ft/s", FOOT_M),
                u("kn", "节", "kn", NMILE_M / 3600.0),
                u("m_min", "米每分", "m/min", 1.0 / 60.0),
                u("km_min", "千米每分", "km/min", 1000.0 / 60.0),
            ],
        },
        // —— 流量（基准单位：立方米每小时 m³/h） ——
        Category {
            id: "flow",
            name: "流量",
            units: vec![
                u("m3_h", "立方米每小时", "m³/h", 1.0),
                u("m3_min", "立方米每分", "m³/min", 60.0),
                u("m3_s", "立方米每秒", "m³/s", 3600.0),
                u("l_s", "升每秒", "L/s", 3.6),
                u("l_min", "升每分", "L/min", 0.06),
                u("l_h", "升每时", "L/h", 0.001),
                u("gpm", "加仑（美制）每分", "gal(US)/min", 0.003785411784 * 60.0),
                u("cfm", "立方英尺每分", "ft³/min", FOOT_M * FOOT_M * FOOT_M * 60.0),
            ],
        },
        // —— 时间（基准单位：秒 s） ——
        Category {
            id: "time",
            name: "时间",
            units: vec![
                u("s", "秒", "s", 1.0),
                u("ms", "毫秒", "ms", 0.001),
                u("us", "微秒", "μs", 1.0e-6),
                u("ns", "纳秒", "ns", 1.0e-9),
                u("min", "分钟", "min", 60.0),
                u("h", "小时", "h", 3600.0),
                u("d", "天", "d", 86400.0),
            ],
        },
        // —— 角度（基准单位：度 °） ——
        Category {
            id: "angle",
            name: "角度",
            units: vec![
                u("deg", "度", "°", 1.0),
                u("rad", "弧度", "rad", DEG_PER_RAD),
                u("arcmin", "角分", "′", 1.0 / 60.0),
                u("arcsec", "角秒", "″", 1.0 / 3600.0),
                u("gon", "百分度（梯度）", "gon", 0.9),
                u("rev", "转（圈）", "r", 360.0),
            ],
        },
        // —— 线规 / 导线截面积（基准单位：平方毫米 mm²） ——
        //
        // 每个 AWG 档位的 factor 即该号线的标准标称截面积（mm²），
        // 因此“1 AWG12”表示一根 12 号线（截面积 3.309 mm²）；
        // 由 mm² 反向换算到某个 AWG 单位时，得到的是“折合成该号线的根数”。
        Category {
            id: "awg",
            name: "线规（AWG 截面）",
            units: vec![
                u("mm2", "平方毫米", "mm²", 1.0),
                u("cm2", "平方厘米", "cm²", 100.0),
                u("awg_4_0", "AWG 4/0 号线", "AWG4/0", 107.2),
                u("awg_2_0", "AWG 2/0 号线", "AWG2/0", 67.43),
                u("awg_1_0", "AWG 1/0 号线", "AWG1/0", 53.49),
                u("awg_1", "AWG 1 号线", "AWG1", 42.41),
                u("awg_2", "AWG 2 号线", "AWG2", 33.62),
                u("awg_4", "AWG 4 号线", "AWG4", 21.15),
                u("awg_6", "AWG 6 号线", "AWG6", 13.30),
                u("awg_8", "AWG 8 号线", "AWG8", 8.367),
                u("awg_10", "AWG 10 号线", "AWG10", 5.261),
                u("awg_12", "AWG 12 号线", "AWG12", 3.309),
                u("awg_14", "AWG 14 号线", "AWG14", 2.081),
                u("awg_16", "AWG 16 号线", "AWG16", 1.309),
                u("awg_18", "AWG 18 号线", "AWG18", 0.823),
            ],
        },
    ]
}

/// 由原单位数值换算到类别基准单位。
pub fn to_base(u: &Unit, v: f64) -> f64 {
    v * u.factor + u.offset
}

/// 由类别基准单位数值换算到目标单位。
pub fn from_base(u: &Unit, b: f64) -> f64 {
    (b - u.offset) / u.factor
}

/// 两个单位之间直接换算。
pub fn convert(from: &Unit, to: &Unit, v: f64) -> f64 {
    from_base(to, to_base(from, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在给定类别中按 id 查找单位。
    fn find_unit<'a>(cats: &'a [Category], cat_id: &str, unit_id: &str) -> &'a Unit {
        let cat = cats
            .iter()
            .find(|c| c.id == cat_id)
            .unwrap_or_else(|| panic!("测试找不到类别：{cat_id}"));
        cat.units
            .iter()
            .find(|x| x.id == unit_id)
            .unwrap_or_else(|| panic!("类别 {cat_id} 中找不到单位：{unit_id}"))
    }

    /// 按类别与单位 id 直接换算：把 v 从 from 单位换算到 to 单位。
    fn cv(cats: &[Category], cat_id: &str, from: &str, to: &str, v: f64) -> f64 {
        convert(find_unit(cats, cat_id, from), find_unit(cats, cat_id, to), v)
    }

    /// 近似断言：|实际 − 期望| ≤ eps × max(|期望|, 1)。
    fn assert_close(actual: f64, expected: f64, eps: f64) {
        let diff = (actual - expected).abs();
        let limit = eps * expected.abs().max(1.0);
        assert!(
            diff <= limit,
            "期望 = {expected}（容差 {eps}），实际 = {actual}，偏差 = {diff}"
        );
    }

    #[test]
    fn test_catalog_structure() {
        let cats = categories();
        // 至少 16 个类别
        assert!(cats.len() >= 16, "类别数量不足：{}", cats.len());
        // 必须包含的工程 / 电气类别
        for required in [
            "length",
            "area",
            "volume",
            "mass",
            "temperature",
            "pressure",
            "force",
            "torque",
            "power",
            "energy",
            "current",
            "voltage",
            "resistance",
            "capacitance",
            "inductance",
            "frequency",
            "rotational_speed",
            "speed",
            "flow",
            "time",
            "angle",
            "awg",
        ] {
            assert!(cats.iter().any(|c| c.id == required), "缺少类别：{required}");
        }
        for (i, c) in cats.iter().enumerate() {
            // 类别 id 全局唯一
            assert!(!cats[..i].iter().any(|o| o.id == c.id), "类别 id 重复：{}", c.id);
            assert!(!c.name.is_empty(), "类别 {} 名称为空", c.id);
            // 每个类别 5~15 个单位
            assert!(
                (5..=15).contains(&c.units.len()),
                "类别 {} 的单位数 {} 超出 5~15",
                c.id,
                c.units.len()
            );
            for (j, unit) in c.units.iter().enumerate() {
                // 单位 id 在类别内唯一
                assert!(
                    !c.units[..j].iter().any(|o| o.id == unit.id),
                    "类别 {} 内单位 id 重复：{}",
                    c.id,
                    unit.id
                );
                assert!(!unit.name.is_empty() && !unit.symbol.is_empty());
                assert!(unit.factor.is_finite() && unit.factor != 0.0);
            }
        }
    }

    #[test]
    fn test_offsets_only_in_temperature() {
        let cats = categories();
        for c in &cats {
            for unit in &c.units {
                if c.id == "temperature" {
                    continue;
                }
                assert_eq!(unit.offset, 0.0, "非温度单位不应有偏移：{}/{}", c.id, unit.id);
            }
        }
        // 温度类严格按 offset 模型定义
        let c = find_unit(&cats, "temperature", "c");
        assert_eq!((c.factor, c.offset), (1.0, 0.0));
        let f = find_unit(&cats, "temperature", "f");
        assert_eq!(f.factor, 5.0 / 9.0);
        assert_eq!(f.offset, -17.7777777778);
        let k = find_unit(&cats, "temperature", "k");
        assert_eq!(k.factor, 1.0);
        assert_eq!(k.offset, -273.15);
    }

    #[test]
    fn test_length() {
        let cats = categories();
        // 1 英寸 = 25.4 毫米（精确定义）
        assert_close(cv(&cats, "length", "in", "mm", 1.0), 25.4, 1e-12);
        // 1 英尺 = 304.8 毫米
        assert_close(cv(&cats, "length", "ft", "mm", 1.0), 304.8, 1e-12);
        // 1 英里 = 1.609344 千米
        assert_close(cv(&cats, "length", "mi", "km", 1.0), 1.609344, 1e-12);
        // 1 市尺 = 1/3 米 ≈ 333.3333 毫米
        assert_close(cv(&cats, "length", "chi", "mm", 1.0), 333.3333333333333, 1e-9);
        // 1 毫米 = 1000 微米
        assert_close(cv(&cats, "length", "mm", "um", 1.0), 1000.0, 1e-12);
        // 1 海里 = 1852 米
        assert_close(cv(&cats, "length", "nmi", "m", 1.0), 1852.0, 1e-12);
    }

    #[test]
    fn test_mass() {
        let cats = categories();
        // 1 磅 = 0.45359237 千克
        assert_close(cv(&cats, "mass", "lb", "kg", 1.0), 0.45359237, 1e-12);
        // 1 千克 = 2 市斤
        assert_close(cv(&cats, "mass", "kg", "jin", 1.0), 2.0, 1e-12);
        // 1 短吨 = 2000 磅
        assert_close(cv(&cats, "mass", "ston", "lb", 1.0), 2000.0, 1e-9);
        // 1 盎司 = 28.349523125 克
        assert_close(cv(&cats, "mass", "oz", "g", 1.0), 28.349523125, 1e-12);
        // 1 格令 = 64.79891 毫克
        assert_close(cv(&cats, "mass", "grain", "mg", 1.0), 64.79891, 1e-9);
    }

    #[test]
    fn test_temperature_basic() {
        let cats = categories();
        // 100 °C = 212 °F
        assert_close(cv(&cats, "temperature", "c", "f", 100.0), 212.0, 1e-9);
        // 0 °C = 32 °F
        assert_close(cv(&cats, "temperature", "c", "f", 0.0), 32.0, 1e-9);
        // 32 °F = 0 °C
        assert_close(cv(&cats, "temperature", "f", "c", 32.0), 0.0, 1e-9);
        // −40 °C = −40 °F
        assert_close(cv(&cats, "temperature", "c", "f", -40.0), -40.0, 1e-9);
        // 0 °C = 273.15 K
        assert_close(cv(&cats, "temperature", "c", "k", 0.0), 273.15, 1e-12);
        // 273.15 K = 0 °C
        assert_close(cv(&cats, "temperature", "k", "c", 273.15), 0.0, 1e-12);
        // 100 °C = 373.15 K
        assert_close(cv(&cats, "temperature", "c", "k", 100.0), 373.15, 1e-12);
        // 491.67 °R = 0 °C
        assert_close(cv(&cats, "temperature", "r", "c", 491.67), 0.0, 1e-9);
    }

    #[test]
    fn test_temperature_round_trip() {
        let cats = categories();
        // °C → °F → K → °C 往返换算应还原原值
        for v in [-40.0, 0.0, 25.0, 37.0, 100.0, 233.0] {
            let f = cv(&cats, "temperature", "c", "f", v);
            let k = cv(&cats, "temperature", "f", "k", f);
            let back = cv(&cats, "temperature", "k", "c", k);
            assert_close(back, v, 1e-9);
        }
    }

    #[test]
    fn test_pressure() {
        let cats = categories();
        // 1 bar = 100000 Pa
        assert_close(cv(&cats, "pressure", "bar", "pa", 1.0), 100000.0, 1e-12);
        // 1 MPa = 10 bar
        assert_close(cv(&cats, "pressure", "mpa", "bar", 1.0), 10.0, 1e-12);
        // 1 psi ≈ 6894.757293 Pa
        assert_close(cv(&cats, "pressure", "psi", "pa", 1.0), 6894.757293, 1e-12);
        // 1 kgf/cm² = 98066.5 Pa
        assert_close(cv(&cats, "pressure", "kgf_cm2", "pa", 1.0), 98066.5, 1e-12);
        // 1 atm = 101.325 kPa
        assert_close(cv(&cats, "pressure", "atm", "kpa", 1.0), 101.325, 1e-12);
        // 1 psi ≈ 0.06894757293 bar
        assert_close(cv(&cats, "pressure", "psi", "bar", 1.0), 0.06894757293, 1e-9);
        // 1 kgf/cm² ≈ 0.980665 bar
        assert_close(cv(&cats, "pressure", "kgf_cm2", "bar", 1.0), 0.980665, 1e-9);
    }

    #[test]
    fn test_force_and_torque() {
        let cats = categories();
        // 1 kgf = 9.80665 N
        assert_close(cv(&cats, "force", "kgf", "n", 1.0), 9.80665, 1e-12);
        // 1 lbf = 4.4482216152605 N
        assert_close(cv(&cats, "force", "lbf", "n", 1.0), 4.4482216152605, 1e-12);
        // 1 吨力 = 1000 公斤力
        assert_close(cv(&cats, "force", "tf", "kgf", 1.0), 1000.0, 1e-9);
        // 1 kgf·m = 9.80665 N·m
        assert_close(cv(&cats, "torque", "kgf_m", "n_m", 1.0), 9.80665, 1e-12);
        // 1 lbf·ft ≈ 1.3558179483314004 N·m
        assert_close(cv(&cats, "torque", "lbf_ft", "n_m", 1.0), 1.3558179483314004, 1e-12);
        // 1 kgf·m = 100 kgf·cm
        assert_close(cv(&cats, "torque", "kgf_m", "kgf_cm", 1.0), 100.0, 1e-9);
    }

    #[test]
    fn test_power_and_energy() {
        let cats = categories();
        // 公制马力 = 735.49875 W
        assert_close(cv(&cats, "power", "ps", "w", 1.0), 735.49875, 1e-12);
        // 英制马力 = 745.699872 W
        assert_close(cv(&cats, "power", "hp", "w", 1.0), 745.699872, 1e-12);
        // 1 千瓦 = 1000 W
        assert_close(cv(&cats, "power", "kw", "w", 1.0), 1000.0, 1e-12);
        // 公制 ↔ 英制马力往返应还原
        let p = cv(&cats, "power", "ps", "hp", 1.0);
        assert_close(cv(&cats, "power", "hp", "ps", p), 1.0, 1e-12);
        // 1 kW·h（度）= 3600000 J
        assert_close(cv(&cats, "energy", "kwh", "j", 1.0), 3600000.0, 1e-12);
        // 1 kW·h = 3600 kJ
        assert_close(cv(&cats, "energy", "kwh", "kj", 1.0), 3600.0, 1e-9);
        // 1 kcal = 4186.8 J
        assert_close(cv(&cats, "energy", "kcal", "j", 1.0), 4186.8, 1e-12);
        // 1 kW·h ≈ 859.845227859 kcal
        assert_close(cv(&cats, "energy", "kwh", "kcal", 1.0), 859.845227858985, 1e-9);
        // 1 公制马力时 = 2647795.5 J
        assert_close(cv(&cats, "energy", "psh", "j", 1.0), 2647795.5, 1e-9);
    }

    #[test]
    fn test_wire_gauge() {
        let cats = categories();
        // 标准 AWG 表标称截面积（mm²）
        assert_close(cv(&cats, "awg", "awg_8", "mm2", 1.0), 8.367, 1e-12);
        assert_close(cv(&cats, "awg", "awg_10", "mm2", 1.0), 5.261, 1e-12);
        assert_close(cv(&cats, "awg", "awg_12", "mm2", 1.0), 3.309, 1e-12);
        assert_close(cv(&cats, "awg", "awg_14", "mm2", 1.0), 2.081, 1e-12);
        assert_close(cv(&cats, "awg", "awg_16", "mm2", 1.0), 1.309, 1e-12);
        assert_close(cv(&cats, "awg", "awg_18", "mm2", 1.0), 0.823, 1e-12);
        // 反向：2.081 mm² 对应 1 根 AWG 14 号线
        assert_close(cv(&cats, "awg", "mm2", "awg_14", 2.081), 1.0, 1e-12);
        // 1 cm² = 100 mm²
        assert_close(cv(&cats, "awg", "cm2", "mm2", 1.0), 100.0, 1e-12);
        // 2.5 mm² 折合 2.5 / 3.309 根 AWG 12 号线
        assert_close(cv(&cats, "awg", "mm2", "awg_12", 2.5), 2.5 / 3.309, 1e-12);
    }

    #[test]
    fn test_rotation_speed_angle_frequency() {
        let cats = categories();
        // 1 转/秒 = 60 转/分
        assert_close(cv(&cats, "rotational_speed", "rps", "rpm", 1.0), 60.0, 1e-12);
        // 1 弧度/秒 ≈ 9.5492965855 转/分
        assert_close(
            cv(&cats, "rotational_speed", "rad_s", "rpm", 1.0),
            60.0 / (2.0 * std::f64::consts::PI),
            1e-12,
        );
        // 1 弧度/秒 ≈ 57.2957795131 度/秒
        assert_close(cv(&cats, "rotational_speed", "rad_s", "deg_s", 1.0), DEG_PER_RAD, 1e-9);
        // 180 度 = π 弧度
        assert_close(cv(&cats, "angle", "deg", "rad", 180.0), std::f64::consts::PI, 1e-12);
        // 1 度 = 60 角分 = 3600 角秒
        assert_close(cv(&cats, "angle", "deg", "arcmin", 1.0), 60.0, 1e-12);
        assert_close(cv(&cats, "angle", "deg", "arcsec", 1.0), 3600.0, 1e-9);
        // 1 转（圈）= 360 度
        assert_close(cv(&cats, "angle", "rev", "deg", 1.0), 360.0, 1e-12);
        // 1 GHz = 1000 MHz
        assert_close(cv(&cats, "frequency", "ghz", "mhz", 1.0), 1000.0, 1e-12);
        // 1 MHz = 1e6 Hz
        assert_close(cv(&cats, "frequency", "mhz", "hz", 1.0), 1.0e6, 1e-12);
    }

    #[test]
    fn test_speed_and_flow() {
        let cats = categories();
        // 1 m/s = 3.6 km/h
        assert_close(cv(&cats, "speed", "m_s", "km_h", 1.0), 3.6, 1e-9);
        // 1 km/h ≈ 0.2778 m/s
        assert_close(cv(&cats, "speed", "km_h", "m_s", 1.0), 1.0 / 3.6, 1e-12);
        // 1 节 = 1.852 km/h
        assert_close(cv(&cats, "speed", "kn", "km_h", 1.0), 1.852, 1e-9);
        // 1 mph = 1.609344 km/h
        assert_close(cv(&cats, "speed", "mph", "km_h", 1.0), 1.609344, 1e-9);
        // 1 m³/h ≈ 0.2778 L/s
        assert_close(cv(&cats, "flow", "m3_h", "l_s", 1.0), 1.0 / 3.6, 1e-12);
        // 1 L/min = 60 L/h
        assert_close(cv(&cats, "flow", "l_min", "l_h", 1.0), 60.0, 1e-9);
        // 1 gal(US)/min = 3.785411784 L/min
        assert_close(cv(&cats, "flow", "gpm", "l_min", 1.0), 3.785411784, 1e-9);
        // 1 ft³/min ≈ 0.4719474432 L/s
        assert_close(cv(&cats, "flow", "cfm", "l_s", 1.0), 0.4719474432, 1e-9);
    }

    #[test]
    fn test_convert_identity_and_inverse() {
        let cats = categories();
        // 同一单位自身换算应还原原值（含温度类的 offset 单位）
        for c in &cats {
            for unit in &c.units {
                assert_close(convert(unit, unit, 12.3456), 12.3456, 1e-12);
            }
        }
        // from → to → from 往返应还原原值
        let cats2 = categories();
        let u_in = find_unit(&cats2, "length", "in");
        let u_mm = find_unit(&cats2, "length", "mm");
        let mm = convert(u_in, u_mm, 2.5);
        assert_close(convert(u_mm, u_in, mm), 2.5, 1e-12);
    }
}
