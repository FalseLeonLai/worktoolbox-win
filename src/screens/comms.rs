//! 通信参考：数据类型表 + 常用协议要点（左侧列表 / 右侧详情，两个选项卡）。

use crate::data::comms::{data_types, protocols, DataType, Protocol};
use crate::theme::*;
use eframe::egui;

pub struct CommsScreen {
    types: Vec<DataType>,
    protos: Vec<Protocol>,
    tab: Tab,
    sel_type: usize,
    sel_proto: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Types,
    Protocols,
}

impl CommsScreen {
    pub fn new() -> Self {
        // 调试用：WTB_COMMS=proto 默认打开协议页（保留无害）
        let tab = if std::env::var("WTB_COMMS")
            .map(|v| v == "proto")
            .unwrap_or(false)
        {
            Tab::Protocols
        } else {
            Tab::Types
        };
        Self {
            types: data_types(),
            protos: protocols(),
            tab,
            sel_type: 0,
            sel_proto: 0,
        }
    }
}

pub fn show(s: &mut CommsScreen, ui: &mut egui::Ui) {
    if let Some(i) = crate::ui::segmented(
        ui,
        &["数据类型", "通信协议"],
        if s.tab == Tab::Types { 0 } else { 1 },
    ) {
        s.tab = if i == 0 { Tab::Types } else { Tab::Protocols };
    }
    ui.add_space(10.0);

    ui.horizontal_top(|ui| {
        // —— 左：列表 ——
        ui.vertical(|ui| {
            ui.set_width(250.0);
            egui::ScrollArea::vertical()
                .id_salt("comms_list")
                .auto_shrink([false, false])
                .show(ui, |ui| match s.tab {
                    Tab::Types => {
                        for (i, t) in s.types.iter().enumerate() {
                            let sel = i == s.sel_type;
                            if ui
                                .selectable_label(sel, egui::RichText::new(t.name).size(14.5))
                                .clicked()
                            {
                                s.sel_type = i;
                            }
                        }
                    }
                    Tab::Protocols => {
                        for (i, p) in s.protos.iter().enumerate() {
                            let sel = i == s.sel_proto;
                            if ui
                                .selectable_label(sel, egui::RichText::new(p.name).size(14.5))
                                .clicked()
                            {
                                s.sel_proto = i;
                            }
                        }
                    }
                });
        });
        ui.separator();
        // —— 右：详情 ——
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("comms_detail")
                .auto_shrink([false, false])
                .show(ui, |ui| match s.tab {
                    Tab::Types => type_detail(ui, s.types.get(s.sel_type)),
                    Tab::Protocols => proto_detail(ui, s.protos.get(s.sel_proto)),
                });
        });
    });
}

fn kv(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{k}："))
                .color(faint_text())
                .size(12.5),
        );
        ui.label(egui::RichText::new(v).color(text()).size(14.0));
    });
    ui.add_space(2.0);
}

fn type_detail(ui: &mut egui::Ui, t: Option<&DataType>) {
    let Some(t) = t else {
        ui.label(egui::RichText::new("（无数据）").color(muted()));
        return;
    };
    ui.label(crate::theme::b(t.name).size(24.0).color(text()));
    ui.add_space(10.0);
    kv(ui, "位宽", t.size);
    kv(ui, "取值范围", t.range);
    ui.add_space(8.0);
    crate::ui::section(ui, "工程备注");
    ui.add_space(3.0);
    ui.label(egui::RichText::new(t.note).size(14.0).color(text()));
}

fn proto_detail(ui: &mut egui::Ui, p: Option<&Protocol>) {
    let Some(p) = p else {
        ui.label(egui::RichText::new("（无数据）").color(muted()));
        return;
    };
    ui.label(crate::theme::b(p.name).size(24.0).color(text()));
    ui.add_space(6.0);
    ui.label(egui::RichText::new(p.summary).size(14.5).color(text()));
    ui.add_space(12.0);
    crate::ui::section(ui, "工程要点");
    for (n, pt) in p.points.iter().enumerate() {
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new(format!("{}. {}", n + 1, pt))
                .size(14.0)
                .color(text()),
        );
    }
}
