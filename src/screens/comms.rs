use crate::app::App;
use eframe::egui;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    super::placeholder(
        ui,
        "通信参考",
        "PLC 常用通信与数据类型速查",
        &[
            "常用数据类型（BOOL / INT / REAL / DWORD …）",
            "常用协议要点（Modbus / EtherCAT / PROFINET …）",
            "字节序与寄存器映射提示",
        ],
    );
}
