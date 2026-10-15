#!/usr/bin/env bash
# 在 Xvfb 虚拟屏里启动 debug 版应用并截图。
# 用法: bash tools/screenshot.sh <页面> [输出路径]
#       页面: home / alarm / units / electrical / comms / cable / about
set -u
export PATH="$HOME/.cargo/bin:$PATH"

SCREEN_NAME="${1:-home}"
OUT="${2:-/home/li/worktoolbox-win/screenshots/${SCREEN_NAME}.png}"
APP=/var/tmp/worktoolbox-target/debug/worktoolbox-win
APP_DIR=/home/li/worktoolbox-win/app

cd "$APP_DIR"
cargo build 2>&1 | tail -3

pkill -f 'Xvfb :99' 2>/dev/null || true
sleep 0.5
Xvfb :99 -screen 0 1280x820x24 >/dev/null 2>&1 &
XVFB_PID=$!
sleep 1.5

unset WAYLAND_DISPLAY
DISPLAY=:99 WTB_START_SCREEN="$SCREEN_NAME" WTB_ELEC_KEY="${WTB_ELEC_KEY:-}" WTB_COMMS="${WTB_COMMS:-}" "$APP" >/tmp/wtb_run.log 2>&1 &
APP_PID=$!
sleep 20
if kill -0 "$APP_PID" 2>/dev/null; then echo "[诊断] 应用进程存活"; else echo "[诊断] 应用进程已退出（可能崩溃）"; fi
echo "[诊断] 进程状态:"
ps -o pid,pcpu,pmem,rss,etime,cmd -p "$APP_PID" 2>/dev/null || true
echo "[诊断] X 窗口树:"
DISPLAY=:99 xwininfo -root -tree 2>/dev/null | head -25 || true

mkdir -p "$(dirname "$OUT")"
DISPLAY=:99 import -window root "$OUT" || echo "!! 截图失败"

kill "$APP_PID" 2>/dev/null || true
kill "$XVFB_PID" 2>/dev/null || true

echo "== app log（全量） =="
cat /tmp/wtb_run.log
ls -la "$OUT" 2>/dev/null || true
