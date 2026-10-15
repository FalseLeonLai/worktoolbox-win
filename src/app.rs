use eframe::egui;

use crate::screens::{about, alarm, cable, comms, electrical, home, units};

/// 主界面所在的"页面"
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Alarm,
    Electrical,
    Units,
    Comms,
    Cable,
    About,
}

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
            "alarm" => Screen::Alarm,
            "electrical" => Screen::Electrical,
            "units" => Screen::Units,
            "comms" => Screen::Comms,
            "cable" => Screen::Cable,
            "about" => Screen::About,
            _ => Screen::Home,
        };
        Self {
            screen,
            alarm: alarm::AlarmScreen::new(&cc.egui_ctx),
            units: units::UnitsScreen::new(),
            electrical: electrical::ElectricalScreen::new(),
            comms: comms::CommsScreen::new(),
            cable: Default::default(),
        }
    }
}

impl eframe::App for App {
    // eframe 0.36 新 API：主渲染入口是 ui()；面板（Panel / CentralPanel）在 Ui 中嵌套
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // —— 顶栏 ——
        egui::Panel::top("top_bar")
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.screen != Screen::Home && ui.button("← 返回主页").clicked() {
                        self.screen = Screen::Home;
                    }
                    ui.label(
                        egui::RichText::new("工作工具箱")
                            .color(crate::theme::ACCENT)
                            .size(18.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v0.1.0 · Windows 版（开发中）")
                                .color(crate::theme::MUTED)
                                .size(12.0),
                        );
                    });
                });
            });

        // —— 主内容区 ——
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui, |ui| {
                let screen = self.screen;
                match screen {
                    Screen::Home => home::show(self, ui),
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
