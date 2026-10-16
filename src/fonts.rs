//! 字体管线 v2：内置 IBM Plex Sans（拉丁/数字）+ JetBrains Mono（等宽），
//! 中文由系统字体补齐（Windows 微软雅黑 / Linux Noto CJK）。
//! 家族：Proportional = [Plex, CJK]，Monospace = [JetBrainsMono, CJK]，
//!       bold / medium 家族供标题与强调使用。

use eframe::egui;
use std::sync::Arc;

// —— 内置拉丁字体（OFL 许可，编译期嵌入）——
const PLEX_REGULAR: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf");
const PLEX_MEDIUM: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf");
const PLEX_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf");
const PLEX_BOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Bold.ttf");
const JB_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");
const JB_MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

/// 中文正文字体候选（Windows 优先，本机 Linux 测试用 Noto CJK 兜底）
const CJK_REGULAR: &[&str] = &[
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\msyh.ttf",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
    r"C:\Windows\Fonts\Deng.ttf",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
];

/// 中文粗体候选
const CJK_BOLD: &[&str] = &[
    r"C:\Windows\Fonts\msyhbd.ttc",
    r"C:\Windows\Fonts\msyhbd.ttf",
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

    // —— 内置拉丁/等宽字体 ——
    let embed: [(&str, &[u8]); 6] = [
        ("plex", PLEX_REGULAR),
        ("plex_med", PLEX_MEDIUM),
        ("plex_sb", PLEX_SEMIBOLD),
        ("plex_bold", PLEX_BOLD),
        ("jbmono", JB_MONO),
        ("jbmono_bold", JB_MONO_BOLD),
    ];
    for (name, bytes) in embed {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(egui::FontData::from_static(bytes)));
    }

    // —— 系统中文 ——
    let cjk = load_first(CJK_REGULAR, "中文正文");
    let cjk_bold = load_first(CJK_BOLD, "中文粗体");
    if let Some(data) = cjk.clone() {
        fonts.font_data.insert("cjk".to_owned(), data);
    }
    let cjk_bold_used = if let Some(data) = cjk_bold {
        fonts.font_data.insert("cjk_bold".to_owned(), data);
        true
    } else {
        false
    };

    // —— 家族编排 ——
    use egui::FontFamily;
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "plex".to_owned());
    if cjk.is_some() {
        prop.insert(1, "cjk".to_owned());
    }

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "jbmono".to_owned());
    if cjk.is_some() {
        mono.insert(1, "cjk".to_owned());
    }

    // bold：Plex Bold + 中文粗体（缺省回退中文正文）
    let bold = fonts.families.entry(FontFamily::Name("bold".into())).or_default();
    bold.insert(0, "plex_bold".to_owned());
    if cjk_bold_used {
        bold.insert(1, "cjk_bold".to_owned());
    }
    if cjk.is_some() {
        bold.push("cjk".to_owned());
    }

    // medium：Plex Medium（中文以正文承担）
    let medium = fonts.families.entry(FontFamily::Name("medium".into())).or_default();
    medium.insert(0, "plex_med".to_owned());
    if cjk.is_some() {
        medium.insert(1, "cjk".to_owned());
    }

    // semibold：Plex SemiBold
    let sb = fonts.families.entry(FontFamily::Name("semibold".into())).or_default();
    sb.insert(0, "plex_sb".to_owned());
    if cjk_bold_used {
        sb.insert(1, "cjk_bold".to_owned());
    }
    if cjk.is_some() {
        sb.push("cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}
