// 工作工具箱 · Windows 版（Rust + egui）
// 电气/工控离线工具箱 —— 报警码查询 / 元件图库 / 单位换算 / 通信参考 / 线缆选型

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod fonts;
mod screens;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("工作工具箱")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "工作工具箱",
        options,
        Box::new(|cc| {
            fonts::install(&cc.egui_ctx);
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}
