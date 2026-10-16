# tools — 开发辅助脚本

本目录是从原始开发机复制过来的脚本，供参考 / 复现用（部分脚本内含开发机的绝对路径与目录约定，使用时按需调整；最近同步：2026-10-16 随 v1.3.1）：

| 脚本 | 用途 |
| --- | --- |
| `convert_alarms.py` | 把源 `alarms.sqlite` 转换为 `data/alarms.bin`（postcard 格式，与 `src/data/alarm.rs` 结构一一对应）。**v2 含数据清洗**：多码合并行拆分 / 名称换行合并 / 空码推导 / 检索文本重建 |
| `gen_standards.py` | 生成 `data/standards/gb_iec.json` 与 `ul.json`（线缆标准表） |
| `resize_assets.sh` | 把元件原图转成两档 WebP（thumbs 300px / full 700px，需要 ImageMagick） |
| `screenshot.sh` | Xvfb 无头渲染截图（Linux 开发机上验证界面用） |
| `package.sh` | 一键交叉编译 + 打包发布 zip（需先装 mingw-w64 交叉工具链） |
