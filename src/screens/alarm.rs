//! 报警码查询：左侧搜索 + 筛选 + 列表（虚拟滚动），右侧详情（原因/确认/措施/来源行）。

use crate::data::alarm::{Alarm, AlarmDb};
use crate::theme::*;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AlarmScreen {
    slot: Arc<Mutex<Option<Result<AlarmDb, String>>>>,
    pub db: Option<AlarmDb>,
    pub load_err: Option<String>,
    pub q: String,
    pub brand: Option<String>,
    pub category: Option<String>,
    results: Vec<u32>,
    last_key: Option<(String, Option<String>, Option<String>)>,
    pub selected: Option<u32>,
    pub show_source: bool,
}

impl AlarmScreen {
    pub fn new(ctx: &egui::Context) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let s2 = Arc::clone(&slot);
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let r = load_db();
            *s2.lock().unwrap() = Some(r);
            ctx2.request_repaint();
        });
        Self {
            slot,
            db: None,
            load_err: None,
            q: String::new(),
            brand: None,
            category: None,
            results: Vec::new(),
            last_key: None,
            selected: None,
            show_source: false,
        }
    }
}

/// 数据文件候选路径：优先程序目录（发布版），其次开发目录
fn data_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("data/alarms.bin"));
        }
    }
    v.push(PathBuf::from("data/alarms.bin"));
    v.push(PathBuf::from("app/data/alarms.bin"));
    v
}

fn load_db() -> Result<AlarmDb, String> {
    for p in data_candidates() {
        if p.exists() {
            return AlarmDb::load(&p);
        }
    }
    // 兜底：使用编译期嵌入 exe 的报警库（单文件运行模式）
    AlarmDb::from_bytes(include_bytes!("../../data/alarms.bin"))
        .map_err(|e| format!("{}（也未找到外置数据文件）", e))
}

pub fn show(s: &mut AlarmScreen, ui: &mut egui::Ui) {
    // 加载线程结果收取
    if s.db.is_none() && s.load_err.is_none() {
        let got = s.slot.lock().unwrap().take();
        if let Some(r) = got {
            match r {
                Ok(db) => s.db = Some(db),
                Err(e) => s.load_err = Some(e),
            }
        } else {
            ui.ctx().request_repaint();
        }
    }

    if let Some(err) = &s.load_err {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("⚠ 报警库加载失败")
                .color(AMBER)
                .size(16.0)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new(err).color(MUTED));
        return;
    }
    let Some(db) = &s.db else {
        ui.add_space(20.0);
        ui.label(egui::RichText::new("报警库加载中…").color(MUTED).size(15.0));
        return;
    };

    // —— 搜索与筛选 ——
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("搜索").color(MUTED).size(13.0));
        ui.add(
            egui::TextEdit::singleline(&mut s.q)
                .hint_text("报警码 / 名称 / 型号 / 原因…")
                .desired_width(240.0),
        );
        ui.add_space(10.0);
        ui.label(egui::RichText::new("品牌").color(MUTED).size(13.0));
        if chip(ui, s.brand.is_none(), "全部") {
            s.brand = None;
        }
        for b in db.brands() {
            if chip(ui, s.brand.as_deref() == Some(b.as_str()), &b) {
                s.brand = Some(b.clone());
            }
        }
        ui.add_space(10.0);
        ui.label(egui::RichText::new("类别").color(MUTED).size(13.0));
        if chip(ui, s.category.is_none(), "全部") {
            s.category = None;
        }
        for c in db.categories() {
            if chip(ui, s.category.as_deref() == Some(c.as_str()), &c) {
                s.category = Some(c.clone());
            }
        }
    });

    // 条件变化时重算
    let key = (s.q.clone(), s.brand.clone(), s.category.clone());
    if s.last_key.as_ref() != Some(&key) {
        s.results = db.search(&s.q, s.brand.as_deref(), s.category.as_deref());
        s.last_key = Some(key);
        if s.selected.map(|i| !s.results.contains(&i)).unwrap_or(true) {
            s.selected = s.results.first().copied();
        }
    }

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(format!("共 {} 条", s.results.len()))
            .color(FAINT_TEXT)
            .size(12.0),
    );
    ui.add_space(4.0);

    // —— 左列表 | 右详情 ——
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(400.0);
            egui::ScrollArea::vertical()
                .id_salt("alarm_list")
                .auto_shrink([false, false])
                .show_rows(ui, 46.0, s.results.len(), |ui, range| {
                    for i in range {
                        let idx = s.results[i];
                        let a = &db.alarms[idx as usize];
                        let cat = db.catalog(a);
                        let tag = format!("{}·{}", cat.brand, cat.category);
                        let selected = s.selected == Some(idx);
                        let resp = row(ui, selected, &a.code, &a.name, &tag);
                        if resp.clicked() {
                            s.selected = Some(idx);
                            s.show_source = false;
                        }
                    }
                });
        });
        ui.separator();
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("alarm_detail")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(idx) = s.selected {
                        detail(ui, db, idx, &mut s.show_source);
                    } else {
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new("← 从左侧选择一条报警查看详情").color(MUTED),
                        );
                    }
                });
        });
    });
}

fn chip(ui: &mut egui::Ui, active: bool, text: &str) -> bool {
    crate::ui::chip(ui, active, text)
}

fn trunc(t: &str, n: usize) -> String {
    let mut s: String = t.chars().take(n).collect();
    if t.chars().count() > n {
        s.push('…');
    }
    s
}

/// 单行列表项（自绘两行卡片，选中/悬停 = 半透明强调底，动效过渡）
fn row(ui: &mut egui::Ui, selected: bool, code: &str, name: &str, tag: &str) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 46.0), egui::Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.1);
    let fill = if selected {
        accent_soft(42)
    } else {
        egui::Color32::TRANSPARENT.lerp_to_gamma(egui::Color32::from_white_alpha(9), t)
    };
    if selected || t > 0.0 {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(8), fill);
    }
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, 8.0),
        egui::Align2::LEFT_TOP,
        trunc(code, 28),
        egui::FontId::monospace(12.5),
        ACCENT,
    );
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, 26.0),
        egui::Align2::LEFT_TOP,
        trunc(name, 24),
        egui::FontId::proportional(13.0),
        if selected { egui::Color32::WHITE } else { TEXT },
    );
    ui.painter().text(
        rect.right_top() + egui::vec2(-12.0, 8.0),
        egui::Align2::RIGHT_TOP,
        trunc(tag, 10),
        egui::FontId::proportional(11.0),
        FAINT_TEXT,
    );
    resp
}

/// 右侧详情：标题 + 元信息 + 各条详情 + 来源行（可展开）
fn detail(ui: &mut egui::Ui, db: &AlarmDb, idx: u32, show_source: &mut bool) {
    let a: &Alarm = &db.alarms[idx as usize];
    let cat = db.catalog(a);

    ui.label(crate::theme::b(&a.title).size(19.0).color(TEXT));
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        meta(ui, "品牌", &cat.brand);
        meta(ui, "类别", &cat.category);
        if !cat.series.is_empty() {
            meta(ui, "系列", &cat.series);
        }
        meta(ui, "来源", &cat.source_path);
    });
    ui.add_space(6.0);
    ui.separator();

    let range = db.details_range(idx);
    for di in range {
        let d = &db.details[di];
        ui.add_space(10.0);
        section(ui, "可能原因", &d.cause);
        section(ui, "确认方法", &d.verification);
        section(ui, "处理措施", &d.action);
        section(ui, "文档地址", &d.doc_ref);
        section(ui, "适用型号", &d.model);

        let provs = db.provenance_of(di as u32);
        if !provs.is_empty() {
            ui.add_space(2.0);
            ui.checkbox(
                show_source,
                egui::RichText::new("显示原始表格行")
                    .size(12.5)
                    .color(MUTED),
            );
            if *show_source {
                for p in provs {
                    let sheet = cat.sheets.get(p.sheet as usize);
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · 第 {} 行",
                            sheet.map(|x| x.name.as_str()).unwrap_or("-"),
                            p.row
                        ))
                        .color(MUTED)
                        .size(12.0),
                    );
                    let headers = sheet.map(|x| x.headers.as_slice()).unwrap_or_default();
                    egui::Grid::new(("prov", di, p.row))
                        .num_columns(2)
                        .spacing([10.0, 3.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (n, c) in p.cells.iter().enumerate() {
                                let h = headers.get(n).map(|s| s.as_str()).unwrap_or("");
                                ui.label(egui::RichText::new(h).color(MUTED).size(12.0));
                                ui.label(egui::RichText::new(c).color(TEXT).size(12.5));
                                ui.end_row();
                            }
                        });
                }
            }
        }
        ui.add_space(6.0);
        ui.separator();
    }
}

fn meta(ui: &mut egui::Ui, k: &str, v: &str) {
    if v.trim().is_empty() || v.trim() == "/" {
        return;
    }
    ui.label(egui::RichText::new(format!("{k}：")).color(FAINT_TEXT).size(12.5));
    ui.label(egui::RichText::new(v).color(MUTED).size(12.5));
    ui.add_space(12.0);
}

fn section(ui: &mut egui::Ui, title: &str, body: &str) {
    let b = body.trim();
    if b.is_empty() || b == "/" {
        return;
    }
    ui.label(crate::theme::b(title).size(13.0).color(MUTED));
    ui.add_space(3.0);
    ui.label(egui::RichText::new(b).color(TEXT).size(14.0));
    ui.add_space(9.0);
}
