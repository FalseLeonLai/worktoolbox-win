use eframe::egui;
use std::sync::Arc;

/// 正文字体候选（Windows 优先，本机 Linux 测试用 Noto CJK 兜底）
const REGULAR: &[&str] = &[
    // Windows
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\msyh.ttf",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
    r"C:\Windows\Fonts\Deng.ttf",
    // Linux（开发机测试）
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
];

/// 粗体字体候选（标题 / 大数字用；缺失时回退正文）
const BOLD: &[&str] = &[
    // Windows
    r"C:\Windows\Fonts\msyhbd.ttc",
    r"C:\Windows\Fonts\msyhbd.ttf",
    // Linux（开发机测试）
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Bold.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Medium.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Bold.ttc",
];

fn load_first(candidates: &[&str], tag: &str) -> Option<Arc<egui::FontData>> {
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            eprintln!("[fonts] {tag}: {path}");
            return Some(Arc::new(egui::FontData::from_owned(bytes)));
        }
    }
    eprintln!("[fonts] {tag}: 未找到可用字体文件");
    None
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let regular = load_first(REGULAR, "正文字体");
    let bold = load_first(BOLD, "粗体字体");

    if let Some(data) = regular.clone() {
        fonts.font_data.insert("cjk".to_owned(), data);
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
    }

    // 粗体家族：粗体文件优先，正文兜底（永远注册家族名，避免 fallback 混乱）
    let bold_family = egui::FontFamily::Name("bold".into());
    let list = fonts.families.entry(bold_family).or_default();
    if let Some(data) = bold {
        fonts.font_data.insert("cjk_bold".to_owned(), data);
        list.insert(0, "cjk_bold".to_owned());
    }
    if regular.is_some() {
        list.push("cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}
