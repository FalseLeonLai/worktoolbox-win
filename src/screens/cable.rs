use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    super::placeholder(
        ui,
        "线缆选型计算",
        "载流量选型 · 电压降校验 · 空开配合 · 电缆型号建议",
        &[
            "多标准表库：国标 GB / 欧标 IEC·EN（CE）/ 美标 UL·NEC",
            "按敷设方式、环境温度、并排回路数做校正",
            "输入负载与距离，直接给出 截面 + 空开 组合建议",
        ],
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("规划于 S2 阶段（先完成原功能复刻）")
            .color(crate::theme::MUTED)
            .size(12.5),
    );
}
