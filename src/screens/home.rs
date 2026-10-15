use crate::app::{App, Screen};
use crate::theme::*;
use eframe::egui;

struct Item {
    title: &'static str,
    sub: &'static str,
    tag: &'static str,
    screen: Screen,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("电气 / 工控 离线工具箱 · 全部功能离线可用")
            .color(MUTED)
            .size(13.5),
    );
    ui.add_space(12.0);

    let items = [
        Item {
            title: "报警码查询",
            sub: "汇川 / 西门子 / 高创 · 伺服 · 变频器",
            tag: "13754 条",
            screen: Screen::Alarm,
        },
        Item {
            title: "电气元件",
            sub: "常用元件图库与参考",
            tag: "",
            screen: Screen::Electrical,
        },
        Item {
            title: "单位换算",
            sub: "工程量 / 电气单位换算",
            tag: "",
            screen: Screen::Units,
        },
        Item {
            title: "通信参考",
            sub: "PLC 数据类型 / 协议速查",
            tag: "",
            screen: Screen::Comms,
        },
        Item {
            title: "线缆选型计算",
            sub: "载流量 / 压降 / 保护配合（GB · IEC · UL）",
            tag: "开发中",
            screen: Screen::Cable,
        },
        Item {
            title: "关于",
            sub: "版本信息与说明",
            tag: "",
            screen: Screen::About,
        },
    ];

    let avail = ui.available_width();
    let cols: usize = if avail > 1000.0 {
        3
    } else if avail > 660.0 {
        2
    } else {
        1
    };
    let spacing = 12.0;
    let card_w = (avail - spacing * (cols as f32 - 1.0) - 8.0) / cols as f32;
    let card_h = 92.0;

    egui::Grid::new("home_grid")
        .num_columns(cols)
        .spacing([spacing, spacing])
        .show(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                let frame = egui::Frame::new()
                    .fill(CARD)
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(14));

                let resp = frame
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(card_w - 28.0, card_h - 28.0));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(item.title).color(TEXT).size(16.5).strong(),
                                );
                                if !item.tag.is_empty() {
                                    ui.label(
                                        egui::RichText::new(item.tag).color(AMBER).size(11.0),
                                    );
                                }
                            });
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(item.sub).color(MUTED).size(12.5),
                            );
                        });
                    })
                    .response
                    .interact(egui::Sense::click());

                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    app.screen = item.screen;
                }

                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
}
