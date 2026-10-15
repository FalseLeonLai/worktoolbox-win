// 构建脚本：为 Windows 目标嵌入 exe 文件图标（资源编译器由 mingw 提供）。
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").map(|v| v == "windows").unwrap_or(false);
    if target_os {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=嵌入 Windows 图标失败: {e}");
        }
    }
    println!("cargo:rerun-if-changed=assets/app.ico");
}
