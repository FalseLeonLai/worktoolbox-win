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

fn screen_name(s: Screen) -> &'static str {
    match s {
        Screen::Home => "",
        Screen::Alarm => "报警码查询",
        Screen::Electrical => "电气元件",
        Screen::Units => "单位换算",
        Screen::Comms => "通信参考",
        Screen::Cable => "线缆选型",
        Screen::About => "关于",
    }
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
            cable: cable::CableScreen::new(),
        }
    }
}

impl eframe::App for App {
    // eframe 0.36 新 API：主渲染入口是 ui()；面板（Panel / CentralPanel）在 Ui 中嵌套
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // —— 顶栏 ——
        let bar = egui::Panel::top("top_bar")
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 11)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.screen != Screen::Home {
                        if crate::ui::ghost(ui, "← 返回") {
                            self.screen = Screen::Home;
                        }
                        ui.add_space(8.0);
                    }
                    crate::ui::logo_mark(ui, 26.0);
                    ui.add_space(9.0);
                    ui.label(
                        crate::theme::b("工作工具箱")
                            .size(15.0)
                            .color(crate::theme::TEXT),
                    );
                    let name = screen_name(self.screen);
                    if !name.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("/")
                                .color(crate::theme::FAINT_TEXT)
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(name)
                                .color(crate::theme::MUTED)
                                .size(13.0),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v1.1")
                                .color(crate::theme::FAINT_TEXT)
                                .size(12.0),
                        );
                    });
                });
            });
        // 顶栏底部 1px 软分隔线
        let r = bar.response.rect;
        ui.painter().hline(
            r.left()..=r.right(),
            r.bottom() - 0.5,
            egui::Stroke::new(1.0, egui::Color32::from_white_alpha(14)),
        );

        // —— 主内容区 ——
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG)
                    .inner_margin(egui::Margin::same(16)),
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
