use eframe::egui::{self, Color32};

// —— 配色（深色工程风）——
pub const BG: Color32 = Color32::from_rgb(10, 16, 32);
pub const PANEL: Color32 = Color32::from_rgb(12, 20, 36);
pub const CARD: Color32 = Color32::from_rgb(18, 28, 48);
pub const CARD_HOVER: Color32 = Color32::from_rgb(24, 38, 62);
pub const ACCENT: Color32 = Color32::from_rgb(34, 180, 214);
pub const TEXT: Color32 = Color32::from_rgb(238, 243, 249);
pub const MUTED: Color32 = Color32::from_rgb(131, 148, 169);
pub const AMBER: Color32 = Color32::from_rgb(245, 158, 11);
pub const OK_GREEN: Color32 = Color32::from_rgb(74, 222, 128);

/// 应用整体视觉风格（深色 + 青色点缀）
pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.visuals = egui::Visuals::dark();

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = PANEL;
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = ACCENT;

        v.widgets.noninteractive.bg_fill = CARD;
        v.widgets.noninteractive.weak_bg_fill = CARD;
        v.widgets.inactive.bg_fill = CARD;
        v.widgets.inactive.weak_bg_fill = CARD;
        v.widgets.hovered.bg_fill = CARD_HOVER;
        v.widgets.hovered.weak_bg_fill = CARD_HOVER;
        v.widgets.active.bg_fill = CARD_HOVER;
        v.widgets.active.weak_bg_fill = CARD_HOVER;

        let radius = egui::CornerRadius::same(8);
        v.widgets.noninteractive.corner_radius = radius;
        v.widgets.inactive.corner_radius = radius;
        v.widgets.hovered.corner_radius = radius;
        v.widgets.active.corner_radius = radius;

        v.selection.bg_fill = ACCENT.linear_multiply(0.35);
        v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    });

    ctx.all_styles_mut(|style| {
        use egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(21.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(15.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
    });
}
