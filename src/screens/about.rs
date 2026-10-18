use crate::app::App;
use crate::theme::*;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("about")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.label(crate::theme::b("工作工具箱 · Windows 版").size(23.0).color(text()));
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION"), " · Rust + egui 重制 · 可离线启动"))
                    .color(muted())
                    .size(13.5),
            );
            ui.add_space(16.0);

            section(ui, "功能");
            bullet(
                ui,
                "报警码查询：13765 条（汇川 / 西门子 / 高创 · 伺服 / 变频器），关键词搜索、品牌与类别筛选，含可能原因、确认方法、处理措施、适用型号与原始表格来源。",
            );
            bullet(ui, "电气元件：30 个常用元件的渲染图、用途原理与工程要点。");
            bullet(ui, "单位换算：22 个类别（含线规 AWG），支持单位对调与全类别对照。");
            bullet(ui, "通信参考：26 个 PLC 数据类型 + 8 类通信协议工程要点速查。");
            bullet(ui, "网络与串口调试：TCP 客户端/服务端、UDP、串口、Modbus TCP/RTU 和经典 Siemens S7；HEX/文本收发与地址读写。");
            bullet(
                ui,
                "线缆计算：三套标准体系（国标 GB / 欧标 IEC·EN（CE） / 美标 UL·NEC），载流量选型、温度与并排校正、电压降校验、空开配合、型号建议。",
            );

            ui.add_space(12.0);
            section(ui, "数据来源");
            bullet(
                ui,
                "报警库与元件资料：整理自原安卓版「工作工具箱」App（v0.1.13）的离线数据。",
            );
            bullet(
                ui,
                "线缆标准数据：节选自 IEC 60364-5-52 / GB-T 16895.15、NEC 310.16 等公开资料（各标准页内有来源说明）。",
            );

            ui.add_space(12.0);
            section(ui, "免责声明");
            bullet(
                ui,
                "全部数据与计算结果供工程参考，正式设计、选型与施工请以现行标准原文及厂家资料为准。",
            );

            ui.add_space(18.0);
            ui.label(
                egui::RichText::new("—— 2026-10 · 单文件离线运行，不联网、不上传任何数据")
                    .color(muted())
                    .size(12.0),
            );
        });
}

fn section(ui: &mut egui::Ui, t: &str) {
    ui.label(crate::theme::b(t).size(13.0).color(muted()));
    ui.add_space(4.0);
}

fn bullet(ui: &mut egui::Ui, t: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("·").color(accent()).size(14.0));
        ui.label(egui::RichText::new(t).color(text()).size(14.0));
    });
    ui.add_space(5.0);
}
