#!/usr/bin/env bash
# 打包 Windows v1.4.2 发布版：交叉编译 release → deliver/ 生成 zip（exe + 使用说明）
set -eu
export PATH="$HOME/.cargo/bin:$PATH"

APP_DIR=/home/li/worktoolbox-win/app
EXE=/var/tmp/worktoolbox-target/x86_64-pc-windows-gnu/release/worktoolbox-win.exe
DELIVER=/home/li/worktoolbox-win/deliver
STAGE="$DELIVER/_stage"

cd "$APP_DIR"
echo "== 交叉编译 release =="
cargo build --release --target x86_64-pc-windows-gnu 2>&1 | tail -4

echo "== 组装 =="
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$EXE" "$STAGE/工作工具箱.exe"
cp "$DELIVER/使用说明.txt" "$STAGE/使用说明.txt"
cp "$DELIVER/试试-Vulkan启动.bat" "$STAGE/试试-Vulkan启动.bat"
cp "$DELIVER/试试-OpenGL启动.bat" "$STAGE/试试-OpenGL启动.bat"

cd "$STAGE"
rm -f "$DELIVER/工作工具箱-v1.4.2-Windows.zip"
zip -r -9 "$DELIVER/工作工具箱-v1.4.2-Windows.zip" . >/dev/null
cd "$DELIVER"
rm -rf "$STAGE"

echo "== 完成 =="
ls -la "$DELIVER/工作工具箱-v1.4.2-Windows.zip"
file "$EXE"
sha256sum "$EXE"
