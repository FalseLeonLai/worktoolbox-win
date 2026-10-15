pub mod about;
pub mod alarm;
pub mod cable;
pub mod comms;
pub mod electrical;
pub mod home;
pub mod units;

use eframe::egui;

/// 未完成模块的占位页面（复刻阶段用，逐步替换为真实功能）
pub fn placeholder(ui: &mut egui::Ui, title: &str, desc: &str, features: &[&str]) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(title)
            .color(crate::theme::TEXT)
            .size(22.0)
            .strong(),
    );
    ui.add_space(6.0);
    ui.label(egui::RichText::new(desc).color(crate::theme::MUTED).size(13.5));
    ui.add_space(16.0);
    for f in features {
        ui.label(
            egui::RichText::new(format!("·  {f}"))
                .color(crate::theme::TEXT)
                .size(14.0),
        );
        ui.add_space(4.0);
    }
    ui.add_space(20.0);
    ui.label(
        egui::RichText::new("⏳ 该模块正在开发中（复刻阶段 S1）")
            .color(crate::theme::AMBER)
            .size(13.5),
    );
}
