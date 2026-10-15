use eframe::egui;
use std::sync::Arc;

/// 中文字体候选（Windows 优先，本机 Linux 测试用 Noto CJK 兜底）
const CANDIDATES: &[&str] = &[
    // Windows
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\msyh.ttf",
    r"C:\Windows\Fonts\msyhbd.ttc",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
    r"C:\Windows\Fonts\Deng.ttf",
    // Linux（开发机测试）
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
];

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            eprintln!("[fonts] 已加载中文字体: {path}");
            let data = egui::FontData::from_owned(bytes);
            fonts.font_data.insert("cjk".to_owned(), Arc::new(data));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "cjk".to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}
