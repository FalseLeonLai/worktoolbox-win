//! 单位换算：左侧类别列表，右侧换算面板（数值 + 单位选择 + 本类别全部单位对照表）。

use crate::data::units::{categories, convert, Category};
use crate::theme::*;
use eframe::egui;

pub struct UnitsScreen {
    cats: Vec<Category>,
    cat_idx: usize,
    from_idx: usize,
    to_idx: usize,
    value: f64,
}

impl UnitsScreen {
    pub fn new() -> Self {
        Self {
            cats: categories(),
            cat_idx: 0,
            from_idx: 0,
            to_idx: 1,
            value: 1.0,
        }
    }
}

pub fn show(s: &mut UnitsScreen, ui: &mut egui::Ui) {
    if s.cats.is_empty() {
        ui.label(egui::RichText::new("单位数据缺失").color(amber()));
        return;
    }
    s.cat_idx = s.cat_idx.min(s.cats.len() - 1);

    ui.horizontal_top(|ui| {
        // —— 左：类别 ——
        ui.vertical(|ui| {
            ui.set_width(180.0);
            crate::ui::section(ui, "类别");
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("units_cats")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, c) in s.cats.iter().enumerate() {
                        let sel = i == s.cat_idx;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 30.0),
                            egui::Sense::click(),
                        );
                        let t = ui
                            .ctx()
                            .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.1);
                        if sel || t > 0.0 {
                            let fill = if sel {
                                accent_soft(42)
                            } else {
                                row_hover(t)
                            };
                            ui.painter()
                                .rect_filled(rect, egui::CornerRadius::same(8), fill);
                        }
                        ui.painter().text(
                            rect.left_center() + egui::vec2(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            c.name,
                            egui::FontId::proportional(13.5),
                            if sel { egui::Color32::WHITE } else { text() },
                        );
                        if resp.clicked() && !sel {
                            s.cat_idx = i;
                            s.from_idx = 0;
                            s.to_idx = if s.cats[i].units.len() > 1 { 1 } else { 0 };
                        }
                    }
                });
        });
        ui.separator();

        // —— 右：换算 ——
        ui.vertical(|ui| {
            let units: Vec<(&'static str, &'static str)> = s.cats[s.cat_idx]
                .units
                .iter()
                .map(|u| (u.name, u.symbol))
                .collect();
            if units.is_empty() {
                ui.label("该类别暂无单位");
                return;
            }
            s.from_idx = s.from_idx.min(units.len() - 1);
            s.to_idx = s.to_idx.min(units.len() - 1);

            ui.label(crate::theme::b(s.cats[s.cat_idx].name).size(20.0).color(text()));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("数值").color(muted()).size(14.0));
                ui.add(
                    egui::DragValue::new(&mut s.value)
                        .speed(1.0)
                        .range(-1e15..=1e15),
                );
                ui.add_space(14.0);
                if ui.button("⇄ 对调").clicked() {
                    std::mem::swap(&mut s.from_idx, &mut s.to_idx);
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("units_from")
                    .width(200.0)
                    .selected_text(format!("{}（{}）", units[s.from_idx].0, units[s.from_idx].1))
                    .show_ui(ui, |ui| {
                        for (i, (n, sym)) in units.iter().enumerate() {
                            ui.selectable_value(&mut s.from_idx, i, format!("{n}（{sym}）"));
                        }
                    });
                ui.label(egui::RichText::new("→").color(accent()).size(18.0));
                egui::ComboBox::from_id_salt("units_to")
                    .width(200.0)
                    .selected_text(format!("{}（{}）", units[s.to_idx].0, units[s.to_idx].1))
                    .show_ui(ui, |ui| {
                        for (i, (n, sym)) in units.iter().enumerate() {
                            ui.selectable_value(&mut s.to_idx, i, format!("{n}（{sym}）"));
                        }
                    });
            });

            ui.add_space(16.0);
            let from = &s.cats[s.cat_idx].units[s.from_idx];
            let to = &s.cats[s.cat_idx].units[s.to_idx];
            let out = convert(from, to, s.value);
            ui.label(
                egui::RichText::new(format!("{} {}", fmt_num(s.value), from.symbol))
                    .color(muted())
                    .size(16.0),
            );
            ui.add_space(2.0);
            ui.label(
                crate::theme::b(format!("= {} {}", fmt_num(out), to.symbol))
                    .color(accent())
                    .size(32.0),
            );

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(crate::theme::b("本类别全部单位（按当前输入换算）").size(13.0).color(muted()));
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("units_all")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let from = &s.cats[s.cat_idx].units[s.from_idx];
                    egui::Grid::new("units_table")
                        .num_columns(3)
                        .spacing([18.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("单位").color(muted()).size(12.5));
                            ui.label(egui::RichText::new("符号").color(muted()).size(12.5));
                            ui.label(egui::RichText::new("数值").color(muted()).size(12.5));
                            ui.end_row();
                            for u in &s.cats[s.cat_idx].units {
                                let v = convert(from, u, s.value);
                                ui.label(egui::RichText::new(u.name).size(13.5));
                                ui.label(egui::RichText::new(u.symbol).color(muted()).size(13.0));
                                ui.label(
                                    egui::RichText::new(fmt_num(v))
                                        .color(if u.id == s.cats[s.cat_idx].units[s.to_idx].id {
                                            accent()
                                        } else {
                                            text()
                                        })
                                        .size(13.5),
                                );
                                ui.end_row();
                            }
                        });
                });
        });
    });
}

/// 数值格式化：普通数值最多 6 位小数并去尾零；过大/过小用科学计数法
fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let a = v.abs();
    if a != 0.0 && (a >= 1e12 || a < 1e-6) {
        return format!("{:.6e}", v);
    }
    let mut s = format!("{:.6}", v);
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s.is_empty() || s == "-" {
        s = "0".into();
    }
    s
}
