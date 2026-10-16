use crate::app::{App, Screen};
use crate::theme::*;
use crate::ui;
use eframe::egui;

struct Item {
    title: &'static str,
    sub: &'static str,
    tag: &'static str,
    glyph: &'static str,
    color: egui::Color32,
    screen: Screen,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui::page_head(ui, "工作工具箱", "电气 / 工控 离线工具箱 · 全部功能离线可用");

    let items = [
        Item {
            title: "报警码查询",
            sub: "汇川 / 西门子 / 高创 · 伺服 · 变频器",
            tag: "13754 条",
            glyph: "报",
            color: ACCENT,
            screen: Screen::Alarm,
        },
        Item {
            title: "电气元件",
            sub: "30 个常用元件图库与参考",
            tag: "",
            glyph: "元",
            color: egui::Color32::from_rgb(138, 180, 248),
            screen: Screen::Electrical,
        },
        Item {
            title: "单位换算",
            sub: "22 类工程量 / 电气单位",
            tag: "",
            glyph: "换",
            color: egui::Color32::from_rgb(124, 217, 165),
            screen: Screen::Units,
        },
        Item {
            title: "通信参考",
            sub: "PLC 数据类型 / 协议速查",
            tag: "",
            glyph: "通",
            color: egui::Color32::from_rgb(180, 151, 245),
            screen: Screen::Comms,
        },
        Item {
            title: "线缆选型计算",
            sub: "载流量 / 压降 / 保护配合",
            tag: "GB · IEC · UL",
            glyph: "算",
            color: egui::Color32::from_rgb(232, 195, 106),
            screen: Screen::Cable,
        },
        Item {
            title: "关于",
            sub: "版本信息与说明",
            tag: "",
            glyph: "关",
            color: egui::Color32::from_rgb(139, 147, 167),
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
    let card_h = 118.0_f32;
    let sub_n = (((card_w - 150.0) / 13.5).floor() as usize).max(8);

    for chunk in items.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            for item in chunk {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                let t = ui
                    .ctx()
                    .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);

                // 卡片底 + 描边（悬停渐亮）
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(12),
                    CARD.lerp_to_gamma(CARD_HOVER, t),
                );
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(12),
                    border((20.0 + 22.0 * t) as u8),
                    egui::StrokeKind::Inside,
                );

                // 字形方块（柔和着色）
                let sq = egui::Rect::from_min_size(
                    rect.left_top() + egui::vec2(16.0, (card_h - 46.0) / 2.0),
                    egui::vec2(46.0, 46.0),
                );
                ui.painter().rect_filled(
                    sq,
                    egui::CornerRadius::same(12),
                    tinted(item.color, (28.0 + 18.0 * t) as u8),
                );
                ui.painter().text(
                    sq.center(),
                    egui::Align2::CENTER_CENTER,
                    item.glyph,
                    egui::FontId::new(20.0, b_font()),
                    item.color,
                );

                // 标题 / 副标题
                let tx = sq.right() + 14.0;
                ui.painter().text(
                    egui::pos2(tx, rect.center().y - 13.0),
                    egui::Align2::LEFT_CENTER,
                    item.title,
                    egui::FontId::new(16.0, b_font()),
                    TEXT,
                );
                ui.painter().text(
                    egui::pos2(tx, rect.center().y + 12.0),
                    egui::Align2::LEFT_CENTER,
                    trunc(item.sub, sub_n),
                    egui::FontId::proportional(12.5),
                    MUTED,
                );

                // 右上角标签 / 右侧箭头
                if !item.tag.is_empty() {
                    ui.painter().text(
                        rect.right_top() + egui::vec2(-14.0, 14.0),
                        egui::Align2::RIGHT_TOP,
                        item.tag,
                        egui::FontId::proportional(11.0),
                        FAINT_TEXT,
                    );
                }
                ui.painter().text(
                    rect.right_center() + egui::vec2(-18.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    "→",
                    egui::FontId::proportional(16.0),
                    MUTED.lerp_to_gamma(ACCENT, t),
                );

                if resp.clicked() {
                    app.screen = item.screen;
                }
            }
        });
        ui.add_space(spacing);
    }
}

fn trunc(t: &str, n: usize) -> String {
    let mut s: String = t.chars().take(n).collect();
    if t.chars().count() > n {
        s.push('…');
    }
    s
}
