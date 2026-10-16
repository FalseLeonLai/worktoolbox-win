//! 设计系统：颜色 / 字号 / 间距令牌 + 全局控件样式 + 排版助手。
//! 风格方向：现代产品级暗色 UI——克制、精确、单强调色、靠层次而非装饰。

use eframe::egui::{self, Color32};

// —— 中性色阶 ——
pub const BG: Color32 = Color32::from_rgb(10, 12, 17); // 主背景
pub const PANEL: Color32 = Color32::from_rgb(14, 17, 24); // 顶栏 / 面板
pub const CARD: Color32 = Color32::from_rgb(18, 21, 29); // 卡片
pub const CARD_HOVER: Color32 = Color32::from_rgb(24, 28, 38); // 卡片悬停
pub const WELL: Color32 = Color32::from_rgb(9, 11, 15); // 输入井（比背景更沉）

// —— 文字三级 ——
pub const TEXT: Color32 = Color32::from_rgb(232, 234, 240);
pub const MUTED: Color32 = Color32::from_rgb(139, 147, 167);
pub const FAINT_TEXT: Color32 = Color32::from_rgb(90, 96, 114);

// —— 强调色与语义色 ——
pub const ACCENT: Color32 = Color32::from_rgb(59, 224, 230);
pub const AMBER: Color32 = Color32::from_rgb(240, 180, 41);
pub const OK_GREEN: Color32 = Color32::from_rgb(74, 222, 128);
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);

/// 1px 半透明白描边（随底色自适应，alpha 建议 20/38/48）
pub fn border(alpha: u8) -> egui::Stroke {
    egui::Stroke::new(1.0, Color32::from_white_alpha(alpha))
}

/// 强调色半透明底（选中态）
pub fn accent_soft(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(59, 224, 230, alpha)
}

/// 语义色半透明底
pub fn tinted(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// 粗体家族（fonts.rs 注册；缺粗体文件时回退 regular）
pub fn b_font() -> egui::FontFamily {
    egui::FontFamily::Name("bold".into())
}

/// 粗体文本助手：标题 / 大数字用
pub fn b(t: impl Into<String>) -> egui::RichText {
    egui::RichText::new(t).family(b_font())
}

/// 应用整体视觉风格
pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.animation_time = 0.16;

        // —— 间距 ——
        let sp = &mut style.spacing;
        sp.item_spacing = egui::vec2(8.0, 7.0);
        sp.button_padding = egui::vec2(12.0, 6.0);
        sp.window_margin = egui::Margin::same(12);
        sp.menu_margin = egui::Margin::same(6);
        sp.indent = 20.0;
        sp.interact_size = egui::vec2(40.0, 28.0);
        sp.slider_width = 140.0;
        sp.combo_width = 160.0;
        sp.text_edit_width = 220.0;
        sp.icon_width = 18.0;
        sp.icon_width_inner = 10.0;
        sp.icon_spacing = 6.0;
        // 浮动细滚动条（悬停才显形）
        sp.scroll = egui::style::ScrollStyle::floating();
        sp.scroll.floating_width = 5.0;
        sp.scroll.bar_width = 10.0;
        sp.scroll.handle_min_length = 28.0;
        sp.scroll.bar_inner_margin = 4.0;
        sp.scroll.floating_allocated_width = 0.0;
        sp.scroll.dormant_handle_opacity = 0.0;
        sp.scroll.active_handle_opacity = 0.38;
        sp.scroll.interact_handle_opacity = 0.85;

        // —— 字号 ——
        use egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(19.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        style.drag_value_text_style = TextStyle::Body;

        // —— 视觉 ——
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = PANEL;
        v.window_stroke = border(24);
        v.window_corner_radius = egui::CornerRadius::same(12);
        v.menu_corner_radius = egui::CornerRadius::same(10);
        v.window_shadow = egui::epaint::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(120),
        };
        v.popup_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(140),
        };
        v.faint_bg_color = PANEL;
        v.extreme_bg_color = WELL;
        v.text_edit_bg_color = Some(WELL);
        v.code_bg_color = CARD;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = AMBER;
        v.error_fg_color = DANGER;
        v.override_text_color = None;
        v.striped = false;
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        v.text_cursor.stroke = egui::Stroke::new(2.0, ACCENT);
        v.text_cursor.preview = true;
        v.selection.bg_fill = accent_soft(56);
        v.selection.stroke = egui::Stroke::new(1.0, ACCENT);

        // —— 控件四态（描边优先于阴影） ——
        fn wv(bg: Color32, bd: u8, fg: Color32) -> egui::style::WidgetVisuals {
            egui::style::WidgetVisuals {
                bg_fill: bg,
                weak_bg_fill: bg,
                bg_stroke: if bd == 0 {
                    egui::Stroke::NONE
                } else {
                    border(bd)
                },
                corner_radius: egui::CornerRadius::same(8),
                fg_stroke: egui::Stroke::new(1.0, fg),
                expansion: 0.0,
            }
        }
        v.widgets.noninteractive = wv(PANEL, 0, TEXT);
        v.widgets.inactive = wv(CARD, 20, TEXT);
        v.widgets.hovered = wv(CARD_HOVER, 38, Color32::from_rgb(246, 248, 252));
        v.widgets.active = wv(Color32::from_rgb(29, 34, 48), 48, Color32::WHITE);
        v.widgets.open = v.widgets.hovered;
    });
}
