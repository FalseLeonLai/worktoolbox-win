use crate::app::{App, Screen};
use crate::theme::*;
use eframe::egui;

struct Item {
    title: &'static str,
    sub: &'static str,
    tag: &'static str,
    color: egui::Color32,
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
            color: ACCENT,
            screen: Screen::Alarm,
        },
        Item {
            title: "电气元件",
            sub: "常用元件图库与参考（30 个）",
            tag: "",
            color: egui::Color32::from_rgb(125, 211, 252),
            screen: Screen::Electrical,
        },
        Item {
            title: "单位换算",
            sub: "工程量 / 电气单位换算（22 类）",
            tag: "",
            color: egui::Color32::from_rgb(74, 222, 128),
            screen: Screen::Units,
        },
        Item {
            title: "通信参考",
            sub: "PLC 数据类型 / 协议速查",
            tag: "",
            color: egui::Color32::from_rgb(167, 139, 250),
            screen: Screen::Comms,
        },
        Item {
            title: "线缆选型计算",
            sub: "载流量 / 压降 / 保护配合（GB · IEC · UL）",
            tag: "",
            color: egui::Color32::from_rgb(251, 191, 36),
            screen: Screen::Cable,
        },
        Item {
            title: "关于",
            sub: "版本信息与说明",
            tag: "",
            color: egui::Color32::from_rgb(148, 163, 184),
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
    let spacing = 12.0_f32;
    let card_w = (avail - spacing * (cols as f32 - 1.0)) / cols as f32;
    let card_h = 96.0_f32;

    for (row, chunk) in items.chunks(cols).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            for item in chunk {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                let hovered = resp.hovered();
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(10),
                    if hovered { CARD_HOVER } else { CARD },
                );
                // 左缘色条（悬停时加宽一点，作为悬停效果）
                let bar_w = if hovered { 5.0 } else { 4.0 };
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        rect.left_top() + egui::vec2(0.0, 12.0),
                        egui::vec2(bar_w, card_h - 24.0),
                    ),
                    egui::CornerRadius::same(2),
                    item.color,
                );
                ui.painter().text(
                    rect.left_top() + egui::vec2(18.0, 16.0),
                    egui::Align2::LEFT_TOP,
                    item.title,
                    egui::FontId::proportional(16.5),
                    TEXT,
                );
                ui.painter().text(
                    rect.left_top() + egui::vec2(18.0, 46.0),
                    egui::Align2::LEFT_TOP,
                    trunc(item.sub, 30),
                    egui::FontId::proportional(12.5),
                    MUTED,
                );
                if !item.tag.is_empty() {
                    ui.painter().text(
                        rect.right_top() + egui::vec2(-12.0, 16.0),
                        egui::Align2::RIGHT_TOP,
                        item.tag,
                        egui::FontId::proportional(11.0),
                        AMBER,
                    );
                }
                ui.painter().text(
                    rect.right_bottom() + egui::vec2(-14.0, -10.0),
                    egui::Align2::RIGHT_BOTTOM,
                    "›",
                    egui::FontId::proportional(18.0),
                    if hovered { ACCENT } else { MUTED },
                );
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    app.screen = item.screen;
                }
            }
        });
        if row + 1 < (items.len() + cols - 1) / cols {
            ui.add_space(spacing);
        }
    }
}

fn trunc(t: &str, n: usize) -> String {
    let mut s: String = t.chars().take(n).collect();
    if t.chars().count() > n {
        s.push('…');
    }
    s
}
