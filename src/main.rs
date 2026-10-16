// 工作工具箱 · Windows 版（Rust + egui）
// 电气/工控离线工具箱 —— 报警码查询 / 元件图库 / 单位换算 / 通信参考 / 线缆选型

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod data;
mod diag;
mod fonts;
mod icons;
mod screens;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    diag::install_panic_hook();
    if std::env::var("WTB_PANIC_TEST").is_ok() {
        panic!("WTB_PANIC_TEST：模拟崩溃（用于验证诊断日志）");
    }
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("工作工具箱")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    // Windows：默认固定 DX12 后端（避免 Intel 等驱动在窗口快速缩放时掉设备 →
    // 上游 egui-wgpu 的「staging buffer」panic）；可用环境变量 WGPU_BACKEND=vulkan|gl 覆盖。
    #[cfg(target_os = "windows")]
    if std::env::var("WGPU_BACKEND").is_err() {
        let mut cfg = eframe::WgpuConfiguration::default();
        if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut cfg.wgpu_setup {
            setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
        }
        options.wgpu_options = cfg;
    }

    let result = eframe::run_native(
        "工作工具箱",
        options,
        Box::new(|cc| {
            diag::startup(&gpu_desc(cc));
            fonts::install(&cc.egui_ctx);
            theme::install(&cc.egui_ctx);
            // 调试/截图用：WTB_THEME=light|dark 指定初始主题（默认深色）
            match std::env::var("WTB_THEME").unwrap_or_default().as_str() {
                "light" => theme::set_dark(&cc.egui_ctx, false),
                "dark" => theme::set_dark(&cc.egui_ctx, true),
                _ => {}
            }
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc)))
        }),
    );
    if let Err(e) = &result {
        diag::write_line(&format!("[eframe 错误] {e:?}\n----------------------------"));
    }
    result
}

/// 记录 GPU / 渲染后端信息（启动诊断用）
fn gpu_desc(cc: &eframe::CreationContext<'_>) -> String {
    if let Some(rs) = &cc.wgpu_render_state {
        let i = rs.adapter.get_info();
        format!(
            "renderer=wgpu | gpu={} | backend={:?} | type={:?} | driver={} {}",
            i.name, i.backend, i.device_type, i.driver, i.driver_info
        )
    } else {
        "renderer=(非 wgpu)".to_string()
    }
}

/// 应用窗口图标（编译期嵌入 WebP，image crate 解码为 RGBA）
fn app_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.webp");
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            egui::IconData {
                rgba: rgba.as_raw().clone(),
                width: rgba.width(),
                height: rgba.height(),
            }
        }
        Err(_) => egui::IconData {
            rgba: Vec::new(),
            width: 0,
            height: 0,
        },
    }
}
