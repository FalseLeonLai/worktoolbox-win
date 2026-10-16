use crate::app::{App, Screen};
use crate::icons;
use crate::theme::*;
use crate::ui;
use eframe::egui;

struct Item {
    title: &'static str,
    sub: &'static str,
    tag: &'static str,
    icon: &'static str,
    neutral: bool,
    screen: Screen,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui::page_head(ui, "工作工具箱", "电气 / 工控 离线工具箱 · 全部功能离线可用");

    let items = [
        Item {
            title: "报警码查询",
            sub: "汇川 / 西门子 / 高创 · 伺服 · 变频器",
            tag: "13754 条",
            icon: "triangle-alert",
            neutral: false,
            screen: Screen::Alarm,
        },
        Item {
            title: "电气元件",
            sub: "30 个常用元件图库与参考",
            tag: "",
            icon: "circuit-board",
            neutral: false,
            screen: Screen::Electrical,
        },
        Item {
            title: "单位换算",
            sub: "22 类工程量 / 电气单位",
            tag: "",
            icon: "ruler",
            neutral: false,
            screen: Screen::Units,
        },
        Item {
            title: "通信参考",
            sub: "PLC 数据类型 / 协议速查",
            tag: "",
            icon: "radio-tower",
            neutral: false,
            screen: Screen::Comms,
        },
        Item {
            title: "线缆选型计算",
            sub: "载流量 / 压降 / 保护配合",
            tag: "GB · IEC · UL",
            icon: "cable",
            neutral: false,
            screen: Screen::Cable,
        },
        Item {
            title: "关于",
            sub: "版本信息与说明",
            tag: "",
            icon: "info",
            neutral: true,
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
    let card_h = 120.0_f32;
    let sub_n = (((card_w - 158.0) / 13.5).floor() as usize).max(8);

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
                    card().lerp_to_gamma(card_hover(), t),
                );
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(12),
                    border((20.0 + 22.0 * t) as u8),
                    egui::StrokeKind::Inside,
                );

                // 图标方块（单强调色；「关于」用中性色）
                let tint = if item.neutral { faint_text() } else { accent() };
                let sq = egui::Rect::from_min_size(
                    rect.left_top() + egui::vec2(16.0, (card_h - 46.0) / 2.0),
                    egui::vec2(46.0, 46.0),
                );
                ui.painter().rect_filled(
                    sq,
                    egui::CornerRadius::same(12),
                    tinted(tint, (24.0 + 16.0 * t) as u8),
                );
                icons::draw(
                    ui.painter(),
                    ui.ctx(),
                    item.icon,
                    sq.shrink(11.0),
                    tint,
                );

                // 标题 / 副标题
                let tx = sq.right() + 14.0;
                ui.painter().text(
                    egui::pos2(tx, rect.center().y - 13.0),
                    egui::Align2::LEFT_CENTER,
                    item.title,
                    egui::FontId::new(16.5, b_font()),
                    text(),
                );
                ui.painter().text(
                    egui::pos2(tx, rect.center().y + 12.0),
                    egui::Align2::LEFT_CENTER,
                    trunc(item.sub, sub_n),
                    egui::FontId::proportional(13.0),
                    muted(),
                );

                // 右上角数据标签（软胶囊）
                if !item.tag.is_empty() {
                    let font = egui::FontId::proportional(11.0);
                    let galley =
                        ui.painter()
                            .layout_no_wrap(item.tag.to_owned(), font, muted());
                    let tw = galley.size().x;
                    let tag_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.right() - 14.0 - tw - 14.0, rect.top() + 13.0),
                        egui::vec2(tw + 14.0, 20.0),
                    );
                    ui.painter().rect_filled(
                        tag_rect,
                        egui::CornerRadius::same(10),
                        well(),
                    );
                    ui.painter().rect_stroke(
                        tag_rect,
                        egui::CornerRadius::same(10),
                        border(18),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().galley(
                        egui::pos2(tag_rect.left() + 7.0, tag_rect.top() + 3.0),
                        galley,
                        muted(),
                    );
                }

                // 右侧箭头（悬停渐变到强调色）
                icons::draw(
                    ui.painter(),
                    ui.ctx(),
                    "chevron-right",
                    egui::Rect::from_center_size(
                        rect.right_center() - egui::vec2(20.0, 0.0),
                        egui::vec2(16.0, 16.0),
                    ),
                    muted().lerp_to_gamma(accent(), t),
                );

                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
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
