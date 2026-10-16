//! 设计系统 v2：明/暗双主题（对齐 ui-ux-pro-max 生成的 Swiss Minimalism 设计系统）。
//! Light：藏青 #1E3A5F + 祖母绿 #047857 + 石板灰中性色（专业工具/企业级方向）
//! Dark ：#0F172A 深蓝黑 + 亮绿 #22C55E。
//! 所有颜色经由存取函数读取，运行时一键切换（顶栏按钮）。

use eframe::egui::{self, Color32};
use std::sync::atomic::{AtomicBool, Ordering};

// ——————————————————————————— 调色板 ———————————————————————————

pub struct Pal {
    pub bg: Color32,           // 主背景
    pub panel: Color32,        // 顶栏 / 面板
    pub card: Color32,         // 卡片
    pub card_hover: Color32,   // 卡片悬停
    pub well: Color32,         // 输入井 / 分段容器
    pub pressed: Color32,      // 按下态
    pub text: Color32,         // 主文字
    pub muted: Color32,        // 次级文字
    pub faint: Color32,        // 弱化文字
    #[allow(dead_code)]
    pub strong: Color32,       // 标题（浅色=藏青）
    pub accent: Color32,       // 强调色（填充 / 文字两用）
    pub accent_hover: Color32, // 强调悬停
    pub on_accent: Color32,    // 强调底上的文字
    pub amber: Color32,        // 警告（语义·文字级对比度）
    pub ok: Color32,           // 成功
    pub danger: Color32,       // 危险
}

pub const LIGHT: Pal = Pal {
    bg: Color32::from_rgb(248, 250, 252),          // #F8FAFC
    panel: Color32::from_rgb(255, 255, 255),       // #FFFFFF
    card: Color32::from_rgb(255, 255, 255),        // #FFFFFF
    card_hover: Color32::from_rgb(241, 245, 249),  // #F1F5F9
    well: Color32::from_rgb(241, 243, 245),        // #F1F3F5
    pressed: Color32::from_rgb(226, 232, 240),     // #E2E8F0
    text: Color32::from_rgb(15, 23, 42),           // #0F172A
    muted: Color32::from_rgb(71, 85, 105),         // #475569
    faint: Color32::from_rgb(148, 163, 184),       // #94A3B8
    strong: Color32::from_rgb(30, 58, 95),         // #1E3A5F 藏青
    accent: Color32::from_rgb(4, 120, 87),         // #047857 祖母绿（白字对比 5.3:1）
    accent_hover: Color32::from_rgb(6, 95, 70),    // #065F46
    on_accent: Color32::from_rgb(255, 255, 255),
    amber: Color32::from_rgb(180, 83, 9),          // #B45309
    ok: Color32::from_rgb(21, 128, 61),            // #15803D
    danger: Color32::from_rgb(185, 28, 28),        // #B91C1C
};

pub const DARK: Pal = Pal {
    bg: Color32::from_rgb(10, 11, 14),             // #0A0B0E 主黑
    panel: Color32::from_rgb(16, 17, 22),          // #101116 侧栏
    card: Color32::from_rgb(20, 22, 27),           // #14161B
    card_hover: Color32::from_rgb(27, 30, 36),     // #1B1E24
    well: Color32::from_rgb(13, 14, 18),           // #0D0E12
    pressed: Color32::from_rgb(35, 38, 46),        // #23262E
    text: Color32::from_rgb(244, 244, 245),        // #F4F4F5
    muted: Color32::from_rgb(161, 161, 170),       // #A1A1AA
    faint: Color32::from_rgb(113, 113, 122),       // #71717A
    strong: Color32::from_rgb(250, 250, 250),      // #FAFAFA
    accent: Color32::from_rgb(52, 211, 153),       // #34D399
    accent_hover: Color32::from_rgb(16, 185, 129), // #10B981
    on_accent: Color32::from_rgb(5, 46, 22),       // #052E16
    amber: Color32::from_rgb(251, 191, 36),        // #FBBF24
    ok: Color32::from_rgb(74, 222, 128),           // #4ADE80
    danger: Color32::from_rgb(248, 113, 113),      // #F87171
};

// ——————————————————————————— 模式切换 ———————————————————————————

static IS_DARK: AtomicBool = AtomicBool::new(true);

pub fn is_dark() -> bool {
    IS_DARK.load(Ordering::Relaxed)
}

pub fn set_dark(ctx: &egui::Context, dark: bool) {
    IS_DARK.store(dark, Ordering::Relaxed);
    install(ctx);
    ctx.request_repaint();
}

pub fn toggle(ctx: &egui::Context) {
    set_dark(ctx, !is_dark());
}

fn p() -> &'static Pal {
    if is_dark() {
        &DARK
    } else {
        &LIGHT
    }
}

// ——————————————————————————— 颜色存取 ———————————————————————————

pub fn bg() -> Color32 { p().bg }
pub fn panel() -> Color32 { p().panel }
pub fn card() -> Color32 { p().card }
pub fn card_hover() -> Color32 { p().card_hover }
pub fn well() -> Color32 { p().well }
pub fn text() -> Color32 { p().text }
pub fn muted() -> Color32 { p().muted }
pub fn faint_text() -> Color32 { p().faint }
#[allow(dead_code)]
pub fn strong() -> Color32 { p().strong }
pub fn accent() -> Color32 { p().accent }
pub fn accent_hover() -> Color32 { p().accent_hover }
pub fn on_accent() -> Color32 { p().on_accent }
pub fn amber() -> Color32 { p().amber }
pub fn ok_green() -> Color32 { p().ok }
pub fn danger() -> Color32 { p().danger }

/// 1px 描边（暗色=半透明白，浅色=半透明黑；alpha 建议 20/38/48）
pub fn border(alpha: u8) -> egui::Stroke {
    if is_dark() {
        egui::Stroke::new(1.0, Color32::from_white_alpha(alpha))
    } else {
        egui::Stroke::new(1.0, Color32::from_black_alpha((alpha as f32 * 0.55) as u8))
    }
}

/// 分隔线 / 轮廓线（两模式都可辨识）
pub fn hairline() -> egui::Stroke {
    if is_dark() {
        egui::Stroke::new(1.0, Color32::from_white_alpha(16))
    } else {
        egui::Stroke::new(1.0, Color32::from_black_alpha(10))
    }
}

/// 列表行悬停浮层（动画友好）
pub fn row_hover(t: f32) -> Color32 {
    let base = if is_dark() {
        Color32::from_white_alpha(10)
    } else {
        Color32::from_black_alpha(6)
    };
    Color32::TRANSPARENT.lerp_to_gamma(base, t)
}

/// 强调色半透明底（选中态）
pub fn accent_soft(alpha: u8) -> Color32 {
    tinted(accent(), alpha)
}

/// 任意色半透明底
pub fn tinted(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

// ——————————————————————————— 字体助手 ———————————————————————————

/// 粗体家族（fonts.rs 注册；缺粗体文件时回退）
pub fn b_font() -> egui::FontFamily {
    egui::FontFamily::Name("bold".into())
}

/// 中等字重家族
#[allow(dead_code)]
pub fn medium_font() -> egui::FontFamily {
    egui::FontFamily::Name("medium".into())
}

/// 粗体文本助手：标题 / 大数字用
pub fn b(t: impl Into<String>) -> egui::RichText {
    egui::RichText::new(t).family(b_font())
}

/// 中等字重文本助手
#[allow(dead_code)]
pub fn med(t: impl Into<String>) -> egui::RichText {
    egui::RichText::new(t).family(medium_font())
}

/// 等宽文本助手：报警码 / 数值 / 型号（JetBrains Mono）
#[allow(dead_code)]
pub fn m(t: impl Into<String>) -> egui::RichText {
    egui::RichText::new(t).monospace()
}

// ——————————————————————————— 全局控件样式 ———————————————————————————

/// 应用整体视觉风格（切换主题时重新调用）
pub fn install(ctx: &egui::Context) {
    let dark = is_dark();
    ctx.all_styles_mut(|style| {
        style.animation_time = 0.16;

        // —— 间距（8dp 节奏）——
        let sp = &mut style.spacing;
        sp.item_spacing = egui::vec2(8.0, 8.0);
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
        sp.scroll.active_handle_opacity = if dark { 0.38 } else { 0.30 };
        sp.scroll.interact_handle_opacity = if dark { 0.85 } else { 0.65 };

        // —— 字号 ——
        use egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(21.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        style.drag_value_text_style = TextStyle::Body;

        // —— 视觉 ——
        let v = &mut style.visuals;
        v.dark_mode = dark;
        v.panel_fill = panel();
        v.window_fill = panel();
        v.window_stroke = border(24);
        v.window_corner_radius = egui::CornerRadius::same(12);
        v.menu_corner_radius = egui::CornerRadius::same(10);
        v.window_shadow = egui::epaint::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 120 } else { 36 }),
        };
        v.popup_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 140 } else { 48 }),
        };
        v.faint_bg_color = if dark { panel() } else { well() };
        v.extreme_bg_color = well();
        v.text_edit_bg_color = Some(card());
        v.code_bg_color = well();
        v.hyperlink_color = accent();
        v.warn_fg_color = amber();
        v.error_fg_color = danger();
        v.override_text_color = None;
        v.striped = false;
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        v.text_cursor.stroke = egui::Stroke::new(2.0, accent());
        v.text_cursor.preview = true;
        v.selection.bg_fill = accent_soft(if dark { 56 } else { 46 });
        v.selection.stroke = egui::Stroke::new(1.0, accent());

        // —— 控件四态（描边优先于阴影）——
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
        v.widgets.noninteractive = wv(panel(), 0, text());
        v.widgets.inactive = wv(card(), 22, text());
        v.widgets.hovered = wv(card_hover(), 40, text());
        v.widgets.active = wv(p().pressed, 50, text());
        v.widgets.open = v.widgets.hovered;
    });
}
