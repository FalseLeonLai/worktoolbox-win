use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    super::placeholder(
        ui,
        "单位换算",
        "常用工程量与电气单位换算",
        &[
            "长度 / 面积 / 质量 / 温度 / 压力",
            "功率 / 电流 / 电压 / 频率 / 转速",
            "工程量与工程记号速查",
        ],
    );
}
