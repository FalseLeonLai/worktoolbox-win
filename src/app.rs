use eframe::egui;

use crate::screens::{about, alarm, cable, comms, electrical, units};
use crate::theme;
use crate::ui;

/// 一级功能页（左侧菜单对应）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Alarm,
    Electrical,
    Units,
    Comms,
    Cable,
    About,
}

/// 一级菜单：页面 / 图标 / 名称
const MENU: [(Screen, &str, &str); 6] = [
    (Screen::Alarm, "triangle-alert", "报警码查询"),
    (Screen::Electrical, "circuit-board", "电气元件"),
    (Screen::Units, "ruler", "单位换算"),
    (Screen::Comms, "radio-tower", "通信参考"),
    (Screen::Cable, "cable", "线缆选型"),
    (Screen::About, "info", "关于"),
];

pub struct App {
    pub screen: Screen,
    pub alarm: alarm::AlarmScreen,
    pub units: units::UnitsScreen,
    pub electrical: electrical::ElectricalScreen,
    pub comms: comms::CommsScreen,
    pub cable: cable::CableScreen,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 调试用：环境变量 WTB_START_SCREEN=alarm/units/... 可直接进入指定页面（保留无害）
        let screen = match std::env::var("WTB_START_SCREEN").unwrap_or_default().as_str() {
            "electrical" => Screen::Electrical,
            "units" => Screen::Units,
            "comms" => Screen::Comms,
            "cable" => Screen::Cable,
            "about" => Screen::About,
            _ => Screen::Alarm,
        };
        Self {
            screen,
            alarm: alarm::AlarmScreen::new(&cc.egui_ctx),
            units: units::UnitsScreen::new(),
            electrical: electrical::ElectricalScreen::new(),
            comms: comms::CommsScreen::new(),
            cable: cable::CableScreen::new(),
        }
    }
}

impl eframe::App for App {
    // eframe 0.36 新 API：主渲染入口是 ui()；面板（Panel / CentralPanel）在 Ui 中嵌套
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // —— 左侧一级菜单（侧栏）——
        let side = egui::Panel::left("sidebar")
            .exact_size(216.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::panel())
                    .inner_margin(egui::Margin::symmetric(12, 14)),
            )
            .show(ui, |ui| {
                // 品牌
                ui.horizontal(|ui| {
                    ui::logo_mark(ui, 28.0);
                    ui.add_space(9.0);
                    ui.vertical(|ui| {
                        ui.add_space(1.0);
                        ui.label(theme::b("工作工具箱").size(14.5).color(theme::text()));
                        ui.label(
                            egui::RichText::new("电气工控 · 离线")
                                .size(10.5)
                                .color(theme::faint_text()),
                        );
                    });
                });
                ui.add_space(18.0);
                ui.label(
                    egui::RichText::new("工具")
                        .size(11.0)
                        .color(theme::faint_text()),
                );
                ui.add_space(6.0);
                for (scr, icon, label) in MENU {
                    if ui::menu_item(ui, icon, label, self.screen == scr) {
                        self.screen = scr;
                    }
                    ui.add_space(2.0);
                }
                // 底部：主题切换 + 版本
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("v1.4.0 · 单文件离线版")
                            .size(10.5)
                            .color(theme::faint_text()),
                    );
                    ui.add_space(8.0);
                    let (icon, label) = if theme::is_dark() {
                        ("sun", "浅色主题")
                    } else {
                        ("moon", "深色主题")
                    };
                    if ui::menu_item(ui, icon, label, false) {
                        theme::toggle(ui.ctx());
                    }
                });
            });
        // 侧栏右缘 1px 分隔线
        let r = side.response.rect;
        ui.painter()
            .vline(r.right() - 0.5, r.top()..=r.bottom(), theme::hairline());

        // —— 主内容区 ——
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::bg())
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |ui| {
                match self.screen {
                    Screen::Alarm => alarm::show(&mut self.alarm, ui),
                    Screen::Electrical => electrical::show(&mut self.electrical, ui),
                    Screen::Units => units::show(&mut self.units, ui),
                    Screen::Comms => comms::show(&mut self.comms, ui),
                    Screen::Cable => cable::show(&mut self.cable, ui),
                    Screen::About => about::show(self, ui),
                }
            });
    }
}
