# 线缆选型标准数据（GB/IEC 与 UL）

本目录存放「线缆选型计算」功能的标准数据，供 `src/data/cable_engine.rs` 的
`load_standard()` 解析使用。JSON 字段与引擎中的 serde 结构一一对应，
**修改数值或字段结构前请先阅读本文件**。

| 文件 | 标准 | 内容 |
| --- | --- | --- |
| `gb_iec.json` | 国标 GB/T 16895.15（等效 IEC 60364-5-52）/ CE | 载流量表 + 温度校正 + 成束校正 + 断路器档 + 型号提示 |
| `ul.json` | 美标 UL / NEC（NFPA 70） | 同上，按 60/75/90 ℃ 三列组织 |

引擎的 `#[cfg(test)]` 测试通过 `include_str!` 直接编译本目录文件，
因此 **JSON 数字错误会直接导致 `cargo test` 失败**，这是刻意设计的回归保护。

---

## 一、GB/IEC（`gb_iec.json`）

### 节选范围
- 载流量：**三根载流导体**（三相回路）基准，截面 1.5–240 mm²（铝 2.5–240 mm²）。
- 敷设方式：**B1**（单芯线穿管于墙面，列 4）、**C**（明敷/贴墙，列 6）、
  **E**（多芯电缆自由空气/桥架，B.52.10–B.52.13 的“三根载流导体”列）。
- 基准条件：空气 30 ℃、单回路、三根载流导体；铜/铝 × PVC(70 ℃)/XLPE·EPR(90 ℃)。
- 温度校正：`Table B.52.14`（PVC 与 XLPE 两列，10–60/80 ℃）。
- 多回路校正：`Table B.52.17` **项 1**（成束于空气中/表面/封闭，2~20 回路，保守取法；
  单层穿孔桥架等其它项未收录）。
- 断路器档位：国内常用序列 6,10,13,16,20,25,32,40,50,63,80,100,125,160,200,250,315,400 A。

### 数值来源（互相交叉核对）
1. **IEC 60364-5-52:2009 附录 B 原文**（FDIS 全文影印）：
   <https://pdfcoffee.com/iec-60364-5-52-2009-pdf-pdf-free.html>
   （B.52.2–B.52.5、B.52.14、B.52.17–B.52.19）
2. **TiSoft ElectricalDesign 帮助页**（逐表转载 B.52.4/5/10/11/12，与本数据完全一致）：
   <https://www.ti-soft.com/en/support/help/electricaldesign/standards/iec-60364-5-52/current-carrying-capacity/table_b_52_4>
   （将 URL 末段替换为 `table_b_52_5` / `table_b_52_10` / `table_b_52_11` / `table_b_52_12`；
   注：`table_b_52_13` 页面目前失效，其数值改由下述第 3、4 项核对）
3. **Top Cable《Methods of Installation and current-carrying capacities based on IEC 60364-5-52 Ed.3》**（表页为图片，逐页人工核对）：
   <https://www.topcable.com/topmatic/data/en.pdf>
4. **Dancables Alukaflex ECO 样本**（HD 60364-5-52 表 B.52.13 引用值，与第 3 项完全一致）：
   <https://assets.cdnlemu.dk/api/sitecore/asset/get?id=af5807d2-1323-4535-9450-da3b0ff4c921&size=content>
5. **ECalPro 校正系数页**（B.52.14/B.52.17）：
   <https://ecalpro.com/standards/iec-60364/correction-factors-b52>
6. **Schneider Electric《Electrical Installation Guide》**Fig. G16（B.52.17 分组系数）、
   Fig. G21（简化载流量表）：
   <https://www.electrical-installation.org/enwiki/Recommended_simplified_approach_for_cable_sizing>

### 已核对的基准值（回归测试锚点）
- 方法 C、铜、PVC：2.5 mm² = **24 A**、4 mm² = **32 A**、6 mm² = **41 A**；
- 方法 B1、铜、PVC：2.5 mm² = 21 A；方法 E、铜、XLPE：10 mm² = 75 A；
- 温度系数：40 ℃ 时 PVC = 0.87、XLPE = 0.91；
- 成束系数：3 回路 = 0.70、6 回路 = 0.57。

### 版本说明（重要）
- GB/T 16895.15-2002 等同采用 IEC 60364-5-523:1999，与 IEC 60364-5-52:2009 的
  附录 B 表在**个别截面**存在数值差异；本数据集以 **IEC 60364-5-52:2009（+AMD1:2024 未改动表值）** 为准。
- 数据集采用「三根载流导体」表（三相基准）；单相回路使用本表**偏保守**（引擎会在
  notes 中提示）。若需严格的单相（两根载流导体）载流量，应另附 B.52.2/B.52.3 表。

## 二、UL / NEC（`ul.json`）

### 节选范围
- 载流量：**NEC Table 310.16**（≤3 根载流导体，桥架/电缆/直埋，基准 30 ℃），
  铜 14 AWG–750 kcmil、铝 12 AWG–750 kcmil，60/75/90 ℃ 三列。
- 温度校正：**Table 310.15(B)(1)**（2020/2023 版，30 ℃ 基准；60/75/90 ℃ 三列各一表）。
  原表为 5 ℃ 阶梯，数据文件中以「双端点」表示（如 `[31,0.91],[35,0.91]`），
  配合引擎线性内插即还原为阶梯函数。
- 多回路校正：**310.15(C)(1)**（4–6 根 = 0.80、7–9 根 = 0.70、10–20 根 = 0.50、
  21–30 根 = 0.45、31–40 根 = 0.40、≥41 根 = 0.35）。数据文件按**每回路 3 根载流导体**
  折算为「回路数」轴；单相回路使用该折算结果偏保守。
- 断路器档位：**240.6(A)** 标准档位（15 A–1200 A）。
- 型号提示：THHN/THWN-2/XHHW-2（90 ℃）、THW/THWN/XHHW（75 ℃）、TW/UF/NM-B（60 ℃）。

### 数值来源（互相交叉核对）
1. NFPA 官方《Using the NEC Ampacity Charts》资料页（含 310.16 与 310.15(B)(1) 摘录）：
   <https://www.nfpa.org/-/media/project/storefront/catalog/files/code-or-topic-fact-sheets/NECAmpacityWorkflow.pdf>
2. HELUKABEL《Allowable Ampacity Tables》（NEC 2023 表 310.16 转载）：
   <https://www.helukabel.us/HELUKABEL/Publications/Technical-Documents/Allowable-Ampacity-Tables.pdf>
3. OmniCable《NEC Ampacity Data》：<https://www.omnicable.com/docs/default-source/technical-resources/nec-ampacity-data.pdf>
4. nectables.com 表 310.16 全文：<https://nectables.com/ampacity/table-310-16/>
5. 温度校正系数与公式 k = √((Tc−Ta)/(Tc−30)) 逐格交叉验证一致
   （即 NEC 2020 新表的 ≤10/11–15/16–20 ℃ 等行由此确认）。

### 已核对的基准值（回归测试锚点）
- 铜、75 ℃：12 AWG = **25 A**、10 AWG = **35 A**、8 AWG = **50 A**；
- 铜、90 ℃、8 AWG = 55 A；铝、75 ℃、250 kcmil = 205 A；
- 温度系数：90 ℃ 列 @50 ℃ = 0.82；多回路：4 回路（12 根载流导体）= 0.50。

## 三、未收录项（引擎遇到时返回“数据缺失”错误或按 README 说明处理）

- IEC：A1/A2/B2、D1/D2（埋地）、300 mm² 以上截面、两根载流导体表（B.52.2/3）、
  矿物绝缘表、土壤热阻 B.52.16、埋地温度 B.52.15、成束项 2–5、B.52.20/21、
  谐波降容（附录 E）。
- NEC：Table 310.17（单芯自由空气）、屋顶温度表 310.15(B)(2)、
  **240.4(D) 小截面过流保护限值**（14 AWG≤15 A、12 AWG≤20 A、10 AWG≤30 A，铜；
  引擎未强制，选型时须人工注意）、110.14(C) 端接温度降列规则、
  连续负载 125% 系数（210.19/215.2）、motors 430 系列。
- 两标准均未包含：并行电缆/多芯并联均流、短路热稳定校验、中性线谐波电流放大。

## 四、引擎计算假设（与 `cable_engine.rs` 对应）

1. 设计电流：三相 `Ib = P/(√3·U·cosφ)`，单相 `Ib = P/(U·cosφ)`（P = kW×1000）。
2. 校正系数 `k = k_temp × k_group`，均为**线性内插**；越界取端点值并在 notes 中注明。
3. 选截面：最小的 `表值 × k ≥ Ib`；再选 `breaker_sizes` 中 `≥ Ib 且 ≤ 表值 × k` 的
   最小档；不存在时给出警告（不阻断计算）。
4. 电压降：`R = ρ·1000/mm²`（Ω/km），ρ 铜 0.0175、铝 0.0282 Ω·mm²/m（20 ℃）；
   `X = 0.08 Ω/km`（默认假设，未区分敷设方式）；
   三相 `ΔU = √3·Ib·L(km)·(R·cosφ + X·sinφ)`，单相 `ΔU = 2·Ib·L(km)·(R·cosφ + X·sinφ)`。
   超限时逐档放大截面直到满足或达到表内最大截面（后者给出警告）。
5. 电阻按 20 ℃ 直流值估算，**未计入**运行温升（+20% 量级）、集肤/邻近效应与谐波；
   精确设计请用厂家数据或 IEC 60287。
6. `alternatives` 给出比选中大一档起 1~3 档截面（含各自载流量、断路器与压降）。

## 五、免责声明

本数据为工程选型**参考**用途，节选自公开标准文本与厂商资料并已做交叉核对，
但不保证与标准原文逐字一致；正式设计、审图与采购请以现行标准正版原文、
电缆厂家数据表为准。
