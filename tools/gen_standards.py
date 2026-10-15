#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生成 data/standards/gb_iec.json 与 ul.json —— 数值均来自已核对来源，见 README.md。"""
import json, os, math

OUT = "/home/li/worktoolbox-win/app/data/standards"
os.makedirs(OUT, exist_ok=True)


def p(size_label, mm2, amps):
    return {"size": size_label, "mm2": mm2, "amps": amps}


def table(method, material, insulation, sizes, amps_list):
    assert len(sizes) == len(amps_list), (method, material, insulation)
    pts = [p(lbl, mm2, float(a)) for (lbl, mm2), a in zip(sizes, amps_list)]
    return {"method": method, "material": material, "insulation": insulation, "points": pts}


# ---------------- GB / IEC ----------------
gb_sizes = [(f"{s} mm²", float(s)) for s in [1.5, 2.5, 4, 6, 10, 16, 25, 35, 50, 70, 95, 120, 150, 185, 240]]
gb_al_sizes = gb_sizes[1:]  # 铝从 2.5 mm² 起

# IEC 60364-5-52:2009 附录 B，三根载流导体（铜/铝 × PVC/XLPE × B1/C/E）
B1_cu_pvc = [15.5, 21, 28, 36, 50, 68, 89, 110, 134, 171, 207, 239, 262, 296, 346]
C_cu_pvc = [17.5, 24, 32, 41, 57, 76, 96, 119, 144, 184, 223, 259, 299, 341, 403]
E_cu_pvc = [18.5, 25, 34, 43, 60, 80, 101, 126, 153, 198, 238, 278, 319, 364, 430]
B1_cu_xlpe = [20, 28, 37, 48, 66, 88, 117, 144, 175, 222, 269, 312, 342, 384, 450]
C_cu_xlpe = [22, 30, 40, 52, 71, 96, 119, 147, 179, 229, 278, 322, 371, 424, 500]
E_cu_xlpe = [23, 32, 42, 54, 75, 100, 127, 158, 192, 246, 298, 346, 399, 456, 538]
B1_al_pvc = [16.5, 22, 28, 39, 53, 70, 86, 104, 133, 161, 186, 204, 230, 269]
C_al_pvc = [18.5, 25, 32, 44, 59, 73, 90, 110, 140, 170, 197, 227, 259, 305]
E_al_pvc = [19.5, 26, 33, 46, 61, 78, 96, 117, 150, 183, 212, 245, 280, 330]
B1_al_xlpe = [22, 29, 38, 52, 71, 93, 116, 140, 179, 217, 251, 267, 300, 351]
C_al_xlpe = [24, 32, 41, 57, 76, 90, 112, 136, 174, 211, 245, 283, 323, 382]
E_al_xlpe = [24, 32, 41, 58, 77, 97, 120, 146, 187, 227, 263, 304, 347, 409]

gb = {
    "standard_id": "gb_iec",
    "label": "国标 GB/T 16895.15（等效 IEC 60364-5-52 / CE）",
    "source_note": (
        "节选自 GB/T 16895.15《建筑物的电气装置 布线系统载流量》（等同采用 IEC 60364-5-52）附录 B："
        "载流量表 B.52.4（PVC 70 ℃，三根载流导体）与 B.52.5（XLPE/EPR 90 ℃，三根载流导体）之 B1/C 列；"
        "敷设方式 E 取 B.52.10–B.52.13（三根载流导体列）；环境温度校正 B.52.14；多回路（成束）校正 B.52.17 项1。"
        "基准条件：空气 30 ℃、单回路、三根载流导体。数值以 IEC 60364-5-52:2009 版为准"
        "（GB/T 16895.15-2002 基于 1999 年版，个别数值与 2009 版略有差异）。"
    ),
    "materials": [
        {"id": "cu", "label": "铜"},
        {"id": "al", "label": "铝"},
    ],
    "insulations": [
        {"id": "pvc", "label": "PVC（70 ℃）", "max_temp_c": 70.0},
        {"id": "xlpe", "label": "XLPE / EPR（90 ℃）", "max_temp_c": 90.0},
    ],
    "methods": [
        {"id": "B1", "label": "B1 穿管敷设（单芯绝缘线穿管固定于墙面）"},
        {"id": "C", "label": "C 明敷/贴墙（电缆直接固定于墙面或非穿孔桥架）"},
        {"id": "E", "label": "E 桥架/自由空气（多芯电缆敷设于桥架/托盘）"},
    ],
    "sizes": [{"label": lbl, "mm2": mm2} for lbl, mm2 in gb_sizes],
    "ampacity": [
        table("B1", "cu", "pvc", gb_sizes, B1_cu_pvc),
        table("C", "cu", "pvc", gb_sizes, C_cu_pvc),
        table("E", "cu", "pvc", gb_sizes, E_cu_pvc),
        table("B1", "cu", "xlpe", gb_sizes, B1_cu_xlpe),
        table("C", "cu", "xlpe", gb_sizes, C_cu_xlpe),
        table("E", "cu", "xlpe", gb_sizes, E_cu_xlpe),
        table("B1", "al", "pvc", gb_al_sizes, B1_al_pvc),
        table("C", "al", "pvc", gb_al_sizes, C_al_pvc),
        table("E", "al", "pvc", gb_al_sizes, E_al_pvc),
        table("B1", "al", "xlpe", gb_al_sizes, B1_al_xlpe),
        table("C", "al", "xlpe", gb_al_sizes, C_al_xlpe),
        table("E", "al", "xlpe", gb_al_sizes, E_al_xlpe),
    ],
    "temp_correction": [
        {"insulation": "pvc", "points": [[10, 1.22], [15, 1.17], [20, 1.12], [25, 1.06], [30, 1.00],
                                         [35, 0.94], [40, 0.87], [45, 0.79], [50, 0.71], [55, 0.61], [60, 0.50]]},
        {"insulation": "xlpe", "points": [[10, 1.15], [15, 1.12], [20, 1.08], [25, 1.04], [30, 1.00],
                                          [35, 0.96], [40, 0.91], [45, 0.87], [50, 0.82], [55, 0.76],
                                          [60, 0.71], [65, 0.65], [70, 0.58], [75, 0.50], [80, 0.41]]},
    ],
    "grouping_correction": [
        # IEC 60364-5-52 表 B.52.17 项1（成束于空气中/表面/封闭），保守取法
        {"points": [[1, 1.00], [2, 0.80], [3, 0.70], [4, 0.65], [5, 0.60], [6, 0.57], [7, 0.54],
                    [8, 0.52], [9, 0.50], [12, 0.45], [16, 0.41], [20, 0.38]]},
    ],
    "breaker_sizes": [6, 10, 13, 16, 20, 25, 32, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400],
    "cable_types": [
        {"insulation": "pvc", "method": "B1", "example_types": "BV / BVR / BLV / ZR-BV",
         "note": "铜芯（或铝芯 BLV）PVC 绝缘单芯线穿管敷设，穿 PVC 管/钢管"},
        {"insulation": "pvc", "method": "C", "example_types": "BV / BVV / VV / VLV",
         "note": "明敷/贴墙：单芯线卡钉敷设或 PVC 护套电缆直接固定"},
        {"insulation": "pvc", "method": "E", "example_types": "VV / VLV / BVV",
         "note": "PVC 绝缘电缆在桥架/托盘/自由空气中敷设，注意多回路成束校正"},
        {"insulation": "xlpe", "method": "B1", "example_types": "YJV / YJV22 / WDZ-YJY",
         "note": "交联聚乙烯 90 ℃，同截面载流量高于 PVC；穿管时注意弯曲半径与管内填充率"},
        {"insulation": "xlpe", "method": "C", "example_types": "YJV / YJV22 / YJLV / ZR-YJV",
         "note": "明敷/贴墙或非穿孔桥架；铝芯为 YJLV"},
        {"insulation": "xlpe", "method": "E", "example_types": "YJV / YJV22 / YJLV / WDZ-YJY",
         "note": "托盘/桥架/自由空气敷设；多层桥架或并排多回路须做成束校正"},
    ],
    "vd_limits": {
        "general_pct": 5.0,
        "lighting_pct": 3.0,
        "note": "国内工程常用限值：照明线路 ≤3%，动力与一般线路 ≤5%（可按项目电压偏差要求调整）。",
    },
}

# ---------------- UL / NEC ----------------
def awg_mm2(n):
    # n: 14, 12, ... 1；1/0 -> 0, 2/0 -> -1, 3/0 -> -2, 4/0 -> -3
    d_mm = 0.127 * 92 ** ((36 - n) / 39.0)
    return round(math.pi / 4 * d_mm * d_mm, 3)


ul_sizes = [
    ("14 AWG", awg_mm2(14)),
    ("12 AWG", awg_mm2(12)),
    ("10 AWG", awg_mm2(10)),
    ("8 AWG", awg_mm2(8)),
    ("6 AWG", awg_mm2(6)),
    ("4 AWG", awg_mm2(4)),
    ("3 AWG", awg_mm2(3)),
    ("2 AWG", awg_mm2(2)),
    ("1 AWG", awg_mm2(1)),
    ("1/0 AWG", awg_mm2(0)),
    ("2/0 AWG", awg_mm2(-1)),
    ("3/0 AWG", awg_mm2(-2)),
    ("4/0 AWG", awg_mm2(-3)),
    ("250 kcmil", round(250 * 0.506707, 1)),
    ("300 kcmil", round(300 * 0.506707, 1)),
    ("350 kcmil", round(350 * 0.506707, 1)),
    ("400 kcmil", round(400 * 0.506707, 1)),
    ("500 kcmil", round(500 * 0.506707, 1)),
    ("600 kcmil", round(600 * 0.506707, 1)),
    ("700 kcmil", round(700 * 0.506707, 1)),
    ("750 kcmil", round(750 * 0.506707, 1)),
]
ul_al_sizes = ul_sizes[1:]  # 铝/铜包铝从 12 AWG 起

cu60 = [15, 20, 30, 40, 55, 70, 85, 95, 110, 125, 145, 165, 195, 215, 240, 260, 280, 320, 350, 385, 400]
cu75 = [20, 25, 35, 50, 65, 85, 100, 115, 130, 150, 175, 200, 230, 255, 285, 310, 335, 380, 420, 460, 475]
cu90 = [25, 30, 40, 55, 75, 95, 115, 130, 145, 170, 195, 225, 260, 290, 320, 350, 380, 430, 475, 520, 535]
al60 = [15, 25, 35, 40, 55, 65, 75, 85, 100, 115, 130, 150, 170, 195, 210, 225, 260, 285, 315, 320]
al75 = [20, 30, 40, 50, 65, 75, 90, 100, 120, 135, 155, 180, 205, 230, 250, 270, 310, 340, 375, 385]
al90 = [25, 35, 45, 55, 75, 85, 100, 115, 135, 150, 175, 205, 230, 260, 280, 305, 350, 385, 425, 435]

ul = {
    "standard_id": "ul",
    "label": "美标 UL / NEC（NFPA 70）",
    "source_note": (
        "节选自 NFPA 70（NEC）：Table 310.16（≤3 根载流导体，桥架/电缆/直埋，基准环境 30 ℃，60/75/90 ℃ 三列）、"
        "Table 310.15(B)(1)（环境温度校正，30 ℃ 基准）、310.15(C)(1)（>3 根载流导体调整系数，按每回路 3 根折算）、"
        "240.6(A)（标准过流保护档位）。60/75/90 ℃ 列分别对应 TW/UF、THW/THWN、THHN/THWN-2/XHHW-2 等绝缘类型；"
        "端接温度限制见 NEC 110.14(C)（本数据未自动降列，选型时注意）。"
    ),
    "materials": [
        {"id": "cu", "label": "铜（Copper）"},
        {"id": "al", "label": "铝/铜包铝（Aluminum / CCA）"},
    ],
    "insulations": [
        {"id": "60c", "label": "60 ℃ 级（TW / UF / NM-B）", "max_temp_c": 60.0},
        {"id": "75c", "label": "75 ℃ 级（THW / THWN / XHHW）", "max_temp_c": 75.0},
        {"id": "90c", "label": "90 ℃ 级（THHN / THWN-2 / XHHW-2）", "max_temp_c": 90.0},
    ],
    "methods": [
        {"id": "raceway", "label": "桥架/穿管/电缆（≤3 根载流导体，NEC 310.16）"},
    ],
    "sizes": [{"label": lbl, "mm2": mm2} for lbl, mm2 in ul_sizes],
    "ampacity": [
        table("raceway", "cu", "60c", ul_sizes, cu60),
        table("raceway", "cu", "75c", ul_sizes, cu75),
        table("raceway", "cu", "90c", ul_sizes, cu90),
        table("raceway", "al", "60c", ul_al_sizes, al60),
        table("raceway", "al", "75c", ul_al_sizes, al75),
        table("raceway", "al", "90c", ul_al_sizes, al90),
    ],
    "temp_correction": [
        # 2020/2023 NEC 310.15(B)(1)：阶梯值以双端点表示，配合线性内插即阶梯函数
        {"insulation": "60c", "points": [[10, 1.29], [11, 1.22], [15, 1.22], [16, 1.15], [20, 1.15], [21, 1.08],
                                         [25, 1.08], [26, 1.00], [30, 1.00], [31, 0.91], [35, 0.91], [36, 0.82],
                                         [40, 0.82], [41, 0.71], [45, 0.71], [46, 0.58], [50, 0.58], [51, 0.41], [55, 0.41]]},
        {"insulation": "75c", "points": [[10, 1.20], [11, 1.15], [15, 1.15], [16, 1.11], [20, 1.11], [21, 1.05],
                                         [25, 1.05], [26, 1.00], [30, 1.00], [31, 0.94], [35, 0.94], [36, 0.88],
                                         [40, 0.88], [41, 0.82], [45, 0.82], [46, 0.75], [50, 0.75], [51, 0.67],
                                         [55, 0.67], [56, 0.58], [60, 0.58], [61, 0.47], [65, 0.47], [66, 0.33], [70, 0.33]]},
        {"insulation": "90c", "points": [[10, 1.15], [11, 1.12], [15, 1.12], [16, 1.08], [20, 1.08], [21, 1.04],
                                         [25, 1.04], [26, 1.00], [30, 1.00], [31, 0.96], [35, 0.96], [36, 0.91],
                                         [40, 0.91], [41, 0.87], [45, 0.87], [46, 0.82], [50, 0.82], [51, 0.76],
                                         [55, 0.76], [56, 0.71], [60, 0.71], [61, 0.65], [65, 0.65], [66, 0.58],
                                         [70, 0.58], [71, 0.50], [75, 0.50], [76, 0.41], [80, 0.41], [81, 0.29], [85, 0.29]]},
    ],
    "grouping_correction": [
        # NEC 310.15(C)(1)，按每回路 3 根载流导体折算成"回路数"轴（整数回路取值为规范原值）
        {"points": [[1, 1.00], [2, 0.80], [3, 0.70], [4, 0.50], [6, 0.50], [7, 0.45], [10, 0.45],
                    [11, 0.40], [13, 0.40], [14, 0.35]]},
    ],
    "breaker_sizes": [15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100, 110, 125, 150, 175, 200,
                      225, 250, 300, 350, 400, 450, 500, 600, 700, 800, 1000, 1200],
    "cable_types": [
        {"insulation": "90c", "method": "raceway", "example_types": "THHN / THWN-2 / XHHW-2 / RHH / RHW-2",
         "note": "90 ℃ 列；端接（开关/端子）多为 60/75 ℃ 额定，按 NEC 110.14(C) 需取较低温度列"},
        {"insulation": "75c", "method": "raceway", "example_types": "THW / THWN / XHHW / RHW / USE",
         "note": "75 ℃ 列，常见端子额定温度档"},
        {"insulation": "60c", "method": "raceway", "example_types": "TW / UF / NM-B / SE",
         "note": "60 ℃ 列；NM-B（Romex）按 NEC 334.80 限制使用 60 ℃ 列"},
    ],
    "vd_limits": {
        "general_pct": 3.0,
        "lighting_pct": 3.0,
        "note": "NEC 210.19(A) 信息注释：分支线路压降 ≤3%；215.2(A)(1) 信息注释：馈线加分支总压降 ≤5%。此处按单段分支回路 3% 取值。",
    },
}

for name, obj in (("gb_iec.json", gb), ("ul.json", ul)):
    path = os.path.join(OUT, name)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=4)
        f.write("\n")
    n_tables = len(obj["ampacity"])
    n_points = sum(len(t["points"]) for t in obj["ampacity"])
    print(f"{name}: ampacity 表 {n_tables} 张 / 数据点 {n_points} 个 / sizes {len(obj['sizes'])}")
