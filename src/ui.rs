//! 共享 UI 组件（按设计令牌手搓，替代默认控件观感；带悬停动画）。

use crate::theme::*;
use eframe::egui;

/// 卡片容器（fill + 1px 描边 + 圆角 + 内边距）
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(border(20))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(14))
        .show(ui, add)
        .inner
}

/// 区块标题：13px 粗体 MUTED（不用彩色）
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(b(text).size(13.0).color(MUTED));
    ui.add_space(2.0);
}

/// 页头：标题 19px 粗体 + 副标题
pub fn page_head(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(b(title).size(19.0).color(TEXT));
    if !sub.is_empty() {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(sub).size(12.5).color(MUTED));
    }
    ui.add_space(10.0);
}

/// 软分隔线
pub fn sep(ui: &mut egui::Ui) {
    ui.add_space(7.0);
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.top(),
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(12)),
    );
    ui.add_space(7.0);
}

/// 自绘胶囊按钮基元：动画悬停 + 选中态
fn pill_base(
    ui: &mut egui::Ui,
    text: &str,
    selected: bool,
    extra_w: f32,
    height: f32,
) -> egui::Response {
    let font = egui::FontId::proportional(13.0);
    let text_w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let size = egui::vec2(text_w + extra_w, height);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
    let (base, hover, tc, hc, stroke_a, stroke_hover) = if selected {
        (
            accent_soft(40),
            accent_soft(64),
            egui::Color32::WHITE,
            egui::Color32::WHITE,
            96u8,
            128u8,
        )
    } else {
        (CARD, CARD_HOVER, MUTED, TEXT, 20u8, 38u8)
    };
    let fill = base.lerp_to_gamma(hover, t);
    let stroke = border(((stroke_a as f32) * (1.0 - t) + (stroke_hover as f32) * t) as u8);
    let radius = (height / 2.0) as u8;
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(radius), fill);
    ui.painter()
        .rect_stroke(rect, egui::CornerRadius::same(radius), stroke, egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        tc.lerp_to_gamma(hc, t),
    );
    resp
}

/// 胶囊筛选按钮（激活 = 强调软底 + 白字）
pub fn chip(ui: &mut egui::Ui, active: bool, text: &str) -> bool {
    pill_base(ui, text, active, 26.0, 28.0).clicked()
}

/// 幽灵胶囊（低强调：用于次级操作）
pub fn ghost(ui: &mut egui::Ui, text: &str) -> bool {
    let r = pill_base(ui, text, false, 22.0, 26.0);
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    r.clicked()
}

/// 分段控件（返回新选中的索引）
pub fn segmented(ui: &mut egui::Ui, items: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    egui::Frame::new()
        .fill(WELL)
        .stroke(border(20))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, t) in items.iter().enumerate() {
                let sel = i == selected;
                let font = egui::FontId::proportional(13.5);
                let tw = ui
                    .painter()
                    .layout_no_wrap((*t).to_owned(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(tw + 30.0, 26.0), egui::Sense::click());
                let ht = ui
                    .ctx()
                    .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
                let fill = if sel {
                    CARD_HOVER
                } else {
                    egui::Color32::TRANSPARENT.lerp_to_gamma(
                        egui::Color32::from_white_alpha(6),
                        ht,
                    )
                };
                let stroke = if sel {
                    border(36)
                } else {
                    egui::Stroke::NONE
                };
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(8), fill);
                if sel {
                    ui.painter().rect_stroke(
                        rect,
                        egui::CornerRadius::same(8),
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
                let tc = if sel {
                    egui::Color32::WHITE
                } else {
                    MUTED.lerp_to_gamma(TEXT, ht)
                };
                ui.painter()
                    .text(rect.center(), egui::Align2::CENTER_CENTER, *t, font, tc);
                if resp.clicked() && !sel {
                    clicked = Some(i);
                }
            }
            });
        });
    clicked
}

/// 状态胶囊（语义色软底 + 同色文字）
pub fn pill(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(tinted(color, 26))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(12.0).color(color));
        });
}

/// 大数字块：标签 12px FAINT + 数值 26px 粗体
pub fn stat(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).size(12.0).color(FAINT_TEXT));
        ui.label(b(value).size(26.0).color(color));
    });
}

/// 键值行
pub fn kv(ui: &mut egui::Ui, k: &str, v: &str, vcolor: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(format!("{k}：")).color(FAINT_TEXT).size(13.0));
        ui.label(egui::RichText::new(v).color(vcolor).size(13.5));
    });
}

/// 空态提示
pub fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(msg).color(FAINT_TEXT).size(13.5));
    });
}

/// 迷你 logo 方块（顶栏用）
pub fn logo_mark(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same((size * 0.28) as u8), accent_soft(70));
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same((size * 0.28) as u8),
        border(90),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "工",
        b_font_id(size * 0.58),
        ACCENT,
    );
}

fn b_font_id(size: f32) -> egui::FontId {
    egui::FontId::new(size, b_font())
}
