//! 共享 UI 组件 v2（Swiss 设计系统：令牌化 + 单强调色 + 悬停动画 + Lucide 图标）。

use crate::icons;
use crate::theme::*;
use eframe::egui;

/// 卡片容器（fill + 1px 描边 + 圆角 + 内边距）
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(crate::theme::card())
        .stroke(border(20))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add)
        .inner
}

/// 区块标题：13px 粗体 MUTED（不用彩色）
pub fn section(ui: &mut egui::Ui, label: &str) {
    ui.label(b(label).size(13.0).color(muted()));
    ui.add_space(3.0);
}

/// 页头：标题 21px 粗体（浅色=藏青）+ 副标题
pub fn page_head(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(b(title).size(21.0).color(strong()));
    if !sub.is_empty() {
        ui.add_space(3.0);
        ui.label(egui::RichText::new(sub).size(13.0).color(muted()));
    }
    ui.add_space(12.0);
}

/// 软分隔线
#[allow(dead_code)]
pub fn sep(ui: &mut egui::Ui) {
    ui.add_space(8.0);
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(rect.left()..=rect.right(), rect.top(), hairline());
    ui.add_space(8.0);
}

/// 自绘胶囊按钮基元：动画悬停 + 选中态（选中 = 强调实底 + 反色文字）
fn pill_base(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    extra_w: f32,
    height: f32,
) -> egui::Response {
    let font = egui::FontId::proportional(13.0);
    let text_w = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let size = egui::vec2(text_w + extra_w, height);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
    let (base, hover, tc, hc, stroke_a, stroke_hover) = if selected {
        (accent(), accent_hover(), on_accent(), on_accent(), 0u8, 0u8)
    } else {
        (
            crate::theme::card(),
            card_hover(),
            muted(),
            text(),
            22u8,
            40u8,
        )
    };
    let fill = base.lerp_to_gamma(hover, t);
    let radius = (height / 2.0) as u8;
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(radius), fill);
    if stroke_a > 0 || stroke_hover > 0 {
        let sa = ((stroke_a as f32) * (1.0 - t) + (stroke_hover as f32) * t) as u8;
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(radius),
            border(sa),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        tc.lerp_to_gamma(hc, t),
    );
    resp
}

/// 胶囊筛选按钮（激活 = 强调实底）
pub fn chip(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    pill_base(ui, label, active, 26.0, 28.0).clicked()
}

/// 幽灵胶囊（低强调：用于次级操作）
#[allow(dead_code)]
pub fn ghost(ui: &mut egui::Ui, label: &str) -> bool {
    let r = pill_base(ui, label, false, 22.0, 26.0);
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    r.clicked()
}

/// 分段控件（返回新选中的索引；选中项 = 强调实底）
pub fn segmented(ui: &mut egui::Ui, items: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    egui::Frame::new()
        .fill(well())
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
                    let (fill, tc) = if sel {
                        (accent().lerp_to_gamma(accent_hover(), ht), on_accent())
                    } else {
                        (
                            egui::Color32::TRANSPARENT.lerp_to_gamma(row_hover(1.0), ht),
                            muted().lerp_to_gamma(text(), ht),
                        )
                    };
                    ui.painter()
                        .rect_filled(rect, egui::CornerRadius::same(8), fill);
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
pub fn pill(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    let alpha = if is_dark() { 30 } else { 24 };
    egui::Frame::new()
        .fill(tinted(color, alpha))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(12.0).color(color));
        });
}

/// 大数字块：标签 12.5px FAINT + 数值 28px 粗体
pub fn stat(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).size(12.5).color(faint_text()));
        ui.label(b(value).size(28.0).color(color));
    });
}

/// 键值行
#[allow(dead_code)]
pub fn kv(ui: &mut egui::Ui, k: &str, v: &str, vcolor: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(format!("{k}：")).color(faint_text()).size(13.0));
        ui.label(egui::RichText::new(v).color(vcolor).size(13.5));
    });
}

/// 空态提示
#[allow(dead_code)]
pub fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(msg).color(faint_text()).size(13.5));
    });
}

/// 迷你 logo 方块（顶栏用）：强调实底 + 反色「工」
pub fn logo_mark(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let radius = egui::CornerRadius::same((size * 0.30) as u8);
    ui.painter().rect_filled(rect, radius, accent());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "工",
        b_font_id(size * 0.56),
        on_accent(),
    );
}

/// 顶栏图标按钮（悬停变色 + 手型指针）
pub fn icon_button(ui: &mut egui::Ui, name: &str, size: f32) -> egui::Response {
    icons::button(ui, name, size, muted(), text())
}

fn b_font_id(size: f32) -> egui::FontId {
    egui::FontId::new(size, b_font())
}
