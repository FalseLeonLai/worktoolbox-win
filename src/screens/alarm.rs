use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    super::placeholder(
        ui,
        "报警码查询",
        "汇川 / 西门子 / 高创 · 伺服与变频器 · 共 13754 条报警代码",
        &[
            "按品牌、类别筛选（伺服 / 变频器）",
            "关键词 / 代码搜索",
            "详情：名称、原因、处理方法、来源",
        ],
    );
}
