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
    pub series: Option<String>,
    /// 系列弹层内的搜索文本
    series_query: String,
    /// 按品牌/类别过滤后的完整系列列表（缓存，用于校验与弹层筛选）
    series_cache: Vec<(usize, String)>,
    /// 缓存失效键 (brand, category)
    series_cache_key: Option<(Option<String>, Option<String>)>,
    results: Vec<u32>,
    last_key: Option<(String, Option<String>, Option<String>, Option<String>)>,
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
            // 调试/截图用：WTB_QUERY 预填搜索词
            q: std::env::var("WTB_QUERY").unwrap_or_default(),
            brand: None,
            category: None,
            // 调试/截图用：WTB_SERIES 预填系列筛选
            series: std::env::var("WTB_SERIES").ok().filter(|v| !v.is_empty()),
            series_query: String::new(),
            series_cache: Vec::new(),
            series_cache_key: None,
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
                .color(amber())
                .size(16.0)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new(err).color(muted()));
        return;
    }
    let Some(db) = &s.db else {
        ui.add_space(20.0);
        ui.label(egui::RichText::new("报警库加载中…").color(muted()).size(15.0));
        return;
    };

    // —— 搜索与筛选 ——
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("搜索").color(muted()).size(13.0));
        ui.add(
            egui::TextEdit::singleline(&mut s.q)
                .hint_text("报警码 / 名称 / 型号 / 原因…")
                .desired_width(240.0),
        );
        ui.add_space(10.0);
        ui.label(egui::RichText::new("品牌").color(muted()).size(13.0));
        if chip(ui, s.brand.is_none(), "全部") {
            s.brand = None;
        }
        for b in db.brands() {
            if chip(ui, s.brand.as_deref() == Some(b.as_str()), &b) {
                s.brand = Some(b.clone());
            }
        }
        ui.add_space(10.0);
        ui.label(egui::RichText::new("类别").color(muted()).size(13.0));
        if chip(ui, s.category.is_none(), "全部") {
            s.category = None;
        }
        for c in db.categories() {
            if chip(ui, s.category.as_deref() == Some(c.as_str()), &c) {
                s.category = Some(c.clone());
            }
        }
    });

    // 系列缓存（随品牌 / 类别联动；键变化时才重建）
    let cache_key = (s.brand.clone(), s.category.clone());
    if s.series_cache_key.as_ref() != Some(&cache_key) {
        s.series_cache = db.series_entries(s.brand.as_deref(), s.category.as_deref());
        s.series_cache_key = Some(cache_key);
    }
    // 当前选择失效时自动清除（必须在缓存重建之后校验）
    if let Some(cur) = &s.series {
        if !s.series_cache.iter().any(|(_, name)| name == cur) {
            s.series = None;
        }
    }

    // 条件变化时重算
    let key = (
        s.q.clone(),
        s.brand.clone(),
        s.category.clone(),
        s.series.clone(),
    );
    if s.last_key.as_ref() != Some(&key) {
        s.results = db.search(
            &s.q,
            s.brand.as_deref(),
            s.category.as_deref(),
            s.series.as_deref(),
        );
        s.last_key = Some(key);
        if s.selected.map(|i| !s.results.contains(&i)).unwrap_or(true) {
            s.selected = s.results.first().copied();
        }
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("系列").color(muted()).size(13.0));
        let cur = s.series.clone().unwrap_or_else(|| "全部".to_string());
        let trigger = ui.add_sized(
            [190.0, 26.0],
            egui::Button::new(egui::RichText::new(format!("{cur} ▾")).size(13.0)),
        );
        ui.add_space(14.0);
        ui.label(
            egui::RichText::new(format!("共 {} 条", s.results.len()))
                .color(faint_text())
                .size(12.0),
        );

        // 系列选择弹层：顶部固定搜索框，下方可滚动列表；Esc / 点击外部由 Popup 关闭
        let popup_id = egui::Popup::default_response_id(&trigger);
        let opening = trigger.clicked() && !egui::Popup::is_id_open(ui.ctx(), popup_id);
        if opening {
            s.series_query.clear();
        }
        let series = &mut s.series;
        let query = &mut s.series_query;
        let cache = &s.series_cache;
        egui::Popup::from_toggle_button_response(&trigger)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width(240.0)
            .show(|ui| {
                let search = ui.add(
                    egui::TextEdit::singleline(&mut *query)
                        .id_salt("series_search")
                        .hint_text("搜索系列 / 型号 / 来源…")
                        .desired_width(f32::INFINITY),
                );
                // 首帧 request_focus 可能不生效：弹层打开期间搜索框未获焦时补请求
                if !search.has_focus() {
                    search.request_focus();
                }
                ui.separator();
                let q = query.trim().to_lowercase();
                egui::ScrollArea::vertical()
                    .id_salt("series_popup")
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if ui.selectable_label(series.is_none(), "全部").clicked() {
                            *series = None;
                            ui.close();
                        }
                        let mut matched = false;
                        for (ci, name) in cache {
                            if !crate::data::alarm::series_matches(&db.catalogs[*ci], &q) {
                                continue;
                            }
                            matched = true;
                            let sel = series.as_deref() == Some(name.as_str());
                            if ui.selectable_label(sel, name).clicked() {
                                *series = Some(name.clone());
                                ui.close();
                            }
                        }
                        if !matched {
                            ui.label(
                                egui::RichText::new("无匹配系列")
                                    .color(faint_text())
                                    .size(12.5),
                            );
                        }
                    });
            });
    });
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
                            egui::RichText::new("← 从左侧选择一条报警查看详情").color(muted()),
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
        row_hover(t)
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
        accent(),
    );
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, 26.0),
        egui::Align2::LEFT_TOP,
        trunc(name, 24),
        egui::FontId::proportional(13.0),
        if selected { egui::Color32::WHITE } else { text() },
    );
    ui.painter().text(
        rect.right_top() + egui::vec2(-12.0, 8.0),
        egui::Align2::RIGHT_TOP,
        trunc(tag, 10),
        egui::FontId::proportional(11.0),
        faint_text(),
    );
    resp
}

/// 右侧详情：标题 + 元信息 + 各条详情 + 来源行（可展开）
fn detail(ui: &mut egui::Ui, db: &AlarmDb, idx: u32, show_source: &mut bool) {
    let a: &Alarm = &db.alarms[idx as usize];
    let cat = db.catalog(a);

    ui.label(crate::theme::b(&a.title).size(19.0).color(text()));
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
                    .color(muted()),
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
                        .color(muted())
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
                                ui.label(egui::RichText::new(h).color(muted()).size(12.0));
                                ui.label(egui::RichText::new(c).color(text()).size(12.5));
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
    ui.label(egui::RichText::new(format!("{k}：")).color(faint_text()).size(12.5));
    ui.label(egui::RichText::new(v).color(muted()).size(12.5));
    ui.add_space(12.0);
}

fn section(ui: &mut egui::Ui, title: &str, body: &str) {
    let b = body.trim();
    if b.is_empty() || b == "/" {
        return;
    }
    ui.label(crate::theme::b(title).size(13.0).color(muted()));
    ui.add_space(3.0);
    ui.label(egui::RichText::new(b).color(text()).size(14.0));
    ui.add_space(9.0);
}
