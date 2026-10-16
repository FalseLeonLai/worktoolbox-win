//! Lucide 图标系统：白色 PNG（96px，编译期嵌入）→ 运行时按需解码 + 纹理缓存，
//! 任意着色（tint 相乘）、悬停动画按钮。图标族统一 Lucide（1.5px 描边风格）。

use eframe::egui;

/// 图标名 → 内嵌 PNG 字节
const ICONS: &[(&str, &[u8])] = &[
    ("arrow-left", include_bytes!("../assets/icons/arrow-left.png")),
    ("arrow-up-right", include_bytes!("../assets/icons/arrow-up-right.png")),
    ("boxes", include_bytes!("../assets/icons/boxes.png")),
    ("cable", include_bytes!("../assets/icons/cable.png")),
    ("chevron-right", include_bytes!("../assets/icons/chevron-right.png")),
    ("circuit-board", include_bytes!("../assets/icons/circuit-board.png")),
    ("gauge", include_bytes!("../assets/icons/gauge.png")),
    ("info", include_bytes!("../assets/icons/info.png")),
    ("moon", include_bytes!("../assets/icons/moon.png")),
    ("package-open", include_bytes!("../assets/icons/package-open.png")),
    ("radio-tower", include_bytes!("../assets/icons/radio-tower.png")),
    ("ruler", include_bytes!("../assets/icons/ruler.png")),
    ("search", include_bytes!("../assets/icons/search.png")),
    ("sun", include_bytes!("../assets/icons/sun.png")),
    ("triangle-alert", include_bytes!("../assets/icons/triangle-alert.png")),
    ("zap", include_bytes!("../assets/icons/zap.png")),
];

/// 取纹理（首用解码 + 缓存到 ctx 数据）
pub fn texture(ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("lucide-icon", name));
    if let Some(h) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(h);
    }
    let bytes = ICONS.iter().find(|(n, _)| *n == name)?.1;
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let handle = ctx.load_texture(format!("icon-{name}"), ci, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    Some(handle)
}

/// 在指定矩形内绘制图标（tint 相乘着色）
pub fn draw(
    painter: &egui::Painter,
    ctx: &egui::Context,
    name: &str,
    rect: egui::Rect,
    tint: egui::Color32,
) {
    if let Some(tex) = texture(ctx, name) {
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
}

/// 图标按钮：悬停变色 + 手型指针（返回 Response，可挂 .on_hover_text）
pub fn button(
    ui: &mut egui::Ui,
    name: &str,
    size: f32,
    base_tint: egui::Color32,
    hover_tint: egui::Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
    let tint = base_tint.lerp_to_gamma(hover_tint, t);
    let pad = size * 0.17;
    draw(
        ui.painter(),
        ui.ctx(),
        name,
        rect.shrink(pad),
        tint,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}
