use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("关于 · 工作工具箱（Windows 版）")
            .color(crate::theme::TEXT)
            .size(22.0)
            .strong(),
    );
    ui.add_space(10.0);
    ui.label(egui::RichText::new("Rust + egui 重制 · v0.1.0（开发中）").color(crate::theme::MUTED));
    ui.add_space(14.0);
    ui.label(
        egui::RichText::new(
            "用途：电气 / 工控工程师的离线工具箱。\n\
             规划功能：报警码查询、电气元件图库、单位换算、通信参考、\n\
             线缆选型计算（国标 GB / 欧标 IEC·EN / 美标 UL·NEC）。",
        )
        .color(crate::theme::TEXT)
        .size(14.0),
    );
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new("数据整理自公开手册与厂商资料，供工程参考。")
            .color(crate::theme::MUTED)
            .size(12.5),
    );
}
