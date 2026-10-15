use eframe::egui;

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
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            screen: Screen::Home,
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
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui, |ui| {
                let screen = self.screen;
                match screen {
                    Screen::Home => crate::screens::home::show(self, ui),
                    Screen::Alarm => crate::screens::alarm::show(self, ui),
                    Screen::Electrical => crate::screens::electrical::show(self, ui),
                    Screen::Units => crate::screens::units::show(self, ui),
                    Screen::Comms => crate::screens::comms::show(self, ui),
                    Screen::Cable => crate::screens::cable::show(self, ui),
                    Screen::About => crate::screens::about::show(self, ui),
                }
            });
    }
}
