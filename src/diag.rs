//! 轻量诊断日志（v1.4.1）：启动环境 + 崩溃信息，写入程序目录（不可写时退到临时目录）。
//! 用途：定位「拖拽缩放崩溃」等问题时，让用户直接把日志文件发回来。

use std::io::Write;
use std::path::PathBuf;

fn candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("工作工具箱-诊断.log"));
            v.push(dir.join("worktoolbox-diag.log"));
        }
    }
    v.push(std::env::temp_dir().join("worktoolbox-diag.log"));
    v
}

/// 日志文件大小上限：超过后自动重置（正常使用每次启动仅 1 行，不会占用空间）
const LOG_MAX_BYTES: u64 = 256 * 1024;
/// 单条写入上限（防超大回溯一次写爆；按字符边界安全截断）
const LINE_MAX_BYTES: usize = 100 * 1024;

/// 追加一行诊断信息（自动选择可写位置；超过上限自动重置）
pub fn write_line(s: &str) {
    let s = if s.len() > LINE_MAX_BYTES {
        let mut end = LINE_MAX_BYTES;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    } else {
        s
    };
    for p in candidates() {
        // 文件超过上限 → 先删除重置
        let reset = std::fs::metadata(&p)
            .map(|m| m.len() > LOG_MAX_BYTES)
            .unwrap_or(false)
            && std::fs::remove_file(&p).is_ok();
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            if reset {
                let _ = writeln!(f, "---- 日志超过 256 KiB 上限，已自动重置 ----");
            }
            let _ = writeln!(f, "{s}");
            return;
        }
    }
}

/// 简单 UTC 时间戳（yyyy-MM-dd HH:MM:SS）
fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let r = secs.rem_euclid(86_400);
    let (h, m, s) = (r / 3600, (r % 3600) / 60, r % 60);
    // Hinnant 民历算法（epoch 天 → 年月日）
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

/// 记录一次启动（版本 + 渲染环境）
pub fn startup(extra: &str) {
    write_line(&format!(
        "=== 启动 v{} · {} · {}",
        env!("CARGO_PKG_VERSION"),
        timestamp(),
        extra
    ));
}

/// 安装 panic 钩子：panic 信息 + 位置 + 回溯写入日志（随后仍执行原钩子）
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "-".into());
        let bt = std::backtrace::Backtrace::force_capture();
        write_line(&format!(
            "[CRASH] {info}\n  位置: {loc}\n{bt}\n----------------------------"
        ));
        prev(info);
    }));
}
