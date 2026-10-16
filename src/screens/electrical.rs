//! 电气元件图库：分组筛选 + 卡片网格 + 详情（图片 / 用途原理 / 工程要点）。
//! 图像方案：编译期嵌入 WebP → image crate 自解码 → egui 纹理缓存（懒加载），
//! 不依赖运行时文件与网络加载器。

use crate::data::electrical::{components, groups_order, Component};
use crate::theme::*;
use eframe::egui;
use std::collections::HashMap;

pub struct ElectricalScreen {
    items: Vec<Component>,
    groups: Vec<&'static str>,
    pub selected: Option<usize>,
    group: Option<&'static str>,
    thumbs: HashMap<&'static str, egui::TextureHandle>,
    fulls: HashMap<&'static str, egui::TextureHandle>,
}

impl ElectricalScreen {
    pub fn new() -> Self {
        let items = components();
        // 调试用：WTB_ELEC_KEY=breaker 可直接打开某元件详情页（保留无害）
        let selected = std::env::var("WTB_ELEC_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .and_then(|k| items.iter().position(|c| c.key == k));
        Self {
            items,
            groups: groups_order(),
            selected,
            group: None,
            thumbs: HashMap::new(),
            fulls: HashMap::new(),
        }
    }

    /// 缩略图纹理（懒加载 + 缓存）
    fn thumb(&mut self, ctx: &egui::Context, key: &'static str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.thumbs.get(key) {
            return Some(t.clone());
        }
        let ci = decode(thumb_bytes(key)?)?;
        let t = ctx.load_texture(format!("thumb-{key}"), ci, egui::TextureOptions::LINEAR);
        self.thumbs.insert(key, t.clone());
        Some(t)
    }

    /// 详情大图纹理（懒加载 + 缓存）
    fn full(&mut self, ctx: &egui::Context, key: &'static str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.fulls.get(key) {
            return Some(t.clone());
        }
        let ci = decode(full_bytes(key)?)?;
        let t = ctx.load_texture(format!("full-{key}"), ci, egui::TextureOptions::LINEAR);
        self.fulls.insert(key, t.clone());
        Some(t)
    }
}

fn decode(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// 生成 key → 嵌入字节 的查表函数（thumbs / full 两档）
macro_rules! webp_table {
    ($fn_name:ident, $dir:literal) => {
        fn $fn_name(key: &str) -> Option<&'static [u8]> {
            Some(match key {
                "breaker" => include_bytes!(concat!("../../assets/electrical/", $dir, "/breaker.webp")),
                "isolator" => include_bytes!(concat!("../../assets/electrical/", $dir, "/isolator.webp")),
                "fuse" => include_bytes!(concat!("../../assets/electrical/", $dir, "/fuse.webp")),
                "rcd" => include_bytes!(concat!("../../assets/electrical/", $dir, "/rcd.webp")),
                "thermal" => include_bytes!(concat!("../../assets/electrical/", $dir, "/thermal.webp")),
                "stop" => include_bytes!(concat!("../../assets/electrical/", $dir, "/stop.webp")),
                "contactor" => include_bytes!(concat!("../../assets/electrical/", $dir, "/contactor.webp")),
                "relay" => include_bytes!(concat!("../../assets/electrical/", $dir, "/relay.webp")),
                "timer" => include_bytes!(concat!("../../assets/electrical/", $dir, "/timer.webp")),
                "button" => include_bytes!(concat!("../../assets/electrical/", $dir, "/button.webp")),
                "selector" => include_bytes!(concat!("../../assets/electrical/", $dir, "/selector.webp")),
                "limit" => include_bytes!(concat!("../../assets/electrical/", $dir, "/limit.webp")),
                "proximity" => include_bytes!(concat!("../../assets/electrical/", $dir, "/proximity.webp")),
                "photoelectric" => include_bytes!(concat!("../../assets/electrical/", $dir, "/photoelectric.webp")),
                "solenoid" => include_bytes!(concat!("../../assets/electrical/", $dir, "/solenoid.webp")),
                "motor" => include_bytes!(concat!("../../assets/electrical/", $dir, "/motor.webp")),
                "vfd" => include_bytes!(concat!("../../assets/electrical/", $dir, "/vfd.webp")),
                "plc" => include_bytes!(concat!("../../assets/electrical/", $dir, "/plc.webp")),
                "supply" => include_bytes!(concat!("../../assets/electrical/", $dir, "/supply.webp")),
                "transformer" => include_bytes!(concat!("../../assets/electrical/", $dir, "/transformer.webp")),
                "ssr" => include_bytes!(concat!("../../assets/electrical/", $dir, "/ssr.webp")),
                "ct" => include_bytes!(concat!("../../assets/electrical/", $dir, "/ct.webp")),
                "terminal" => include_bytes!(concat!("../../assets/electrical/", $dir, "/terminal.webp")),
                "earth" => include_bytes!(concat!("../../assets/electrical/", $dir, "/earth.webp")),
                "resistor" => include_bytes!(concat!("../../assets/electrical/", $dir, "/resistor.webp")),
                "capacitor" => include_bytes!(concat!("../../assets/electrical/", $dir, "/capacitor.webp")),
                "inductor" => include_bytes!(concat!("../../assets/electrical/", $dir, "/inductor.webp")),
                "diode" => include_bytes!(concat!("../../assets/electrical/", $dir, "/diode.webp")),
                "potentiometer" => include_bytes!(concat!("../../assets/electrical/", $dir, "/potentiometer.webp")),
                "lamp" => include_bytes!(concat!("../../assets/electrical/", $dir, "/lamp.webp")),
                _ => return None,
            })
        }
    };
}
webp_table!(thumb_bytes, "thumbs");
webp_table!(full_bytes, "full");

pub fn show(s: &mut ElectricalScreen, ui: &mut egui::Ui) {
    if s.items.is_empty() {
        ui.label(egui::RichText::new("元件数据缺失").color(AMBER));
        return;
    }
    match s.selected {
        None => list_view(s, ui),
        Some(i) => detail_view(s, ui, i),
    }
}

fn chip(ui: &mut egui::Ui, active: bool, text: &str) -> bool {
    crate::ui::chip(ui, active, text)
}

fn list_view(s: &mut ElectricalScreen, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if chip(ui, s.group.is_none(), "全部") {
            s.group = None;
        }
        for g in &s.groups {
            if chip(ui, s.group == Some(*g), g) {
                s.group = Some(*g);
            }
        }
    });
    ui.add_space(8.0);

    // 手排网格：按可用宽度算列数，自绘卡片（悬停高亮），避免自动换行布局的不确定性
    let list: Vec<usize> = (0..s.items.len())
        .filter(|&i| s.group.map(|g| s.items[i].group == g).unwrap_or(true))
        .collect();
    let card_w = 160.0_f32;
    let card_h = 170.0_f32;
    let gap = 12.0_f32;

    egui::ScrollArea::vertical()
        .id_salt("elec_grid")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let avail = ui.available_width();
            let cols = (((avail + gap) / (card_w + gap)).floor() as usize).max(1);
            let ctx = ui.ctx().clone();
            for chunk in list.chunks(cols) {
                ui.horizontal(|ui| {
                    for &i in chunk {
                        let (key, name) = {
                            let c = &s.items[i];
                            (c.key, c.name)
                        };
                        let tex = s.thumb(&ctx, key);
                        let (rect, resp) = ui
                            .allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                        let t = ui
                            .ctx()
                            .animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(12),
                            CARD.lerp_to_gamma(CARD_HOVER, t),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(12),
                            border((20.0 + 22.0 * t) as u8),
                            egui::StrokeKind::Inside,
                        );
                        if let Some(t) = &tex {
                            let img_rect = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.top() + 10.0 + 59.0),
                                egui::vec2(118.0, 118.0),
                            );
                            ui.painter().image(
                                t.id(),
                                img_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        }
                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.bottom() - 20.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            egui::FontId::proportional(14.5),
                            TEXT,
                        );
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            s.selected = Some(i);
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
}

fn detail_view(s: &mut ElectricalScreen, ui: &mut egui::Ui, i: usize) {
    let mut back = false;
    let (key, name, group, summary, tips) = {
        let c = &s.items[i];
        (c.key, c.name, c.group, c.summary, c.tips.clone())
    };
    let ctx = ui.ctx().clone();
    let tex = s.full(&ctx, key);

    ui.horizontal(|ui| {
        if ui.button("← 返回图库").clicked() {
            back = true;
        }
        ui.label(egui::RichText::new(group).color(ACCENT).size(13.0).strong());
    });
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt("elec_detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(crate::theme::b(name).size(24.0).color(TEXT));
            ui.add_space(10.0);
            ui.horizontal_top(|ui| {
                // 左：渲染图（浅底圆角框）
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(248, 249, 251))
                    .corner_radius(12)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        if let Some(t) = &tex {
                            let st = egui::load::SizedTexture::new(t.id(), t.size_vec2());
                            ui.add(
                                egui::Image::from_texture(st)
                                    .max_height(300.0)
                                    .corner_radius(6.0),
                            );
                        } else {
                            ui.label(egui::RichText::new("（图片不可用）").color(MUTED));
                        }
                    });
                ui.add_space(16.0);
                // 右：用途 + 要点
                ui.vertical(|ui| {
                    ui.set_max_width(560.0);
                    crate::ui::section(ui, "用途与原理");
                    ui.add_space(3.0);
                    ui.label(egui::RichText::new(summary).size(14.5).color(TEXT));
                    ui.add_space(12.0);
                    crate::ui::section(ui, "工程要点");
                    for (n, tip) in tips.iter().enumerate() {
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(format!("{}. {}", n + 1, tip))
                                .size(14.0)
                                .color(TEXT),
                        );
                    }
                });
            });
        });
    if back {
        s.selected = None;
    }
}
