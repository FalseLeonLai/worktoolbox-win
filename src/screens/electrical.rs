use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    super::placeholder(
        ui,
        "电气元件图库",
        "常用电气元件图示与说明（断路器 / 接触器 / 继电器 / 电机 / PLC / 电磁阀 …）",
        &["元件图片浏览与详情", "接线与选型参考", "与报警码、计算器交叉跳转"],
    );
}
