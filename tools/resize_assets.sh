#!/usr/bin/env bash
# 把电气元件渲染图转成两档小尺寸 WebP：thumbs/ 300px（列表）、full/ 700px（详情）
# 依赖 ImageMagick（convert）。原 ref/ 目录保留原始大图；app/assets/electrical 旧大图删除。
set -u
SRC=/home/li/worktoolbox-win/ref/assets/electrical/webp
DST=/home/li/worktoolbox-win/app/assets/electrical

CONV=$(command -v magick || command -v convert)
echo "使用: $CONV"

mkdir -p "$DST/thumbs" "$DST/full"
n=0
for f in "$SRC"/*.webp; do
    b=$(basename "$f")
    "$CONV" "$f" -resize 300x300 -quality 82 "$DST/thumbs/$b" || { echo "!! 失败: $b (thumb)"; exit 1; }
    "$CONV" "$f" -resize 700x700 -quality 86 "$DST/full/$b" || { echo "!! 失败: $b (full)"; exit 1; }
    rm -f "$DST/$b"
    n=$((n + 1))
done

echo "转换 $n 张"
du -sh "$DST/thumbs" "$DST/full"
ls "$DST" | head -5
