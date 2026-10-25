//! 报警库数据层：读取由 `tools/convert_alarms.py` 生成的紧凑二进制（postcard 格式）。
//! 数据来源：原安卓版 App 的 alarms.sqlite（汇川 / 西门子 / 高创，13k+ 条报警代码）。

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AlarmDb {
    pub catalogs: Vec<Catalog>,
    pub alarms: Vec<Alarm>,
    /// 按 alarm 升序排序
    pub details: Vec<Detail>,
    /// 按 detail 升序排序
    pub provenance: Vec<Provenance>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Catalog {
    pub brand: String,       // 品牌：汇川 / 西门子 / 高创
    pub category: String,    // 类别：伺服 / 变频器
    pub series: String,      // 系列
    pub source_path: String, // 原始来源（如 高创伺服/BD3E系列.xlsx）
    pub models: Vec<String>, // 系列内型号记号
    pub alarm_count: u32,
    pub sheets: Vec<SheetInfo>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SheetInfo {
    pub name: String,
    /// 原始表头（如 品牌名称/设备分类/型号/标题/可能原因/确认方法/处理措施/文档地址）
    pub headers: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Alarm {
    pub catalog: u16,    // Catalog 下标
    pub code: String,    // 原始报警码
    pub code_key: String,// 规范化码（去空格、小写，用于精确/宽松匹配）
    pub name: String,    // 简称
    pub title: String,   // 标题（码 + 名称）
    /// 预生成的小写检索文本（品牌/类别/型号/码/名称/原因/措施 拼接）
    pub search: String,
    pub detail_count: u16,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Detail {
    pub alarm: u32,        // Alarm 下标
    pub cause: String,     // 可能原因
    pub verification: String, // 确认方法
    pub action: String,    // 处理措施
    pub doc_ref: String,   // 文档地址
    pub model: String,     // 型号
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Provenance {
    pub detail: u32,       // Detail 下标
    pub sheet: u8,         // 所属 Catalog.sheets 下标
    pub row: u32,          // 原始行号
    pub cells: Vec<String>,// 原始单元格值（与表头对齐）
}

impl AlarmDb {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("读取报警库失败 {}: {e}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    /// 从内存字节解析（也用于 exe 内嵌的兜底数据）
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        postcard::from_bytes(bytes).map_err(|e| format!("解析报警库失败: {e}"))
    }

    pub fn catalog(&self, a: &Alarm) -> &Catalog {
        &self.catalogs[a.catalog as usize]
    }

    /// 搜索：空格分词（全部命中）；brand / category / series 精确过滤（None = 不限）。
    /// 「码式查询」（无空格、含数字、仅由码字符构成，如 234 / er.234 / h00-00）只按
    /// 报警码与名称匹配，避免原因文字中的跨码引用造成误报；若无结果则退回全文检索。
    pub fn search(
        &self,
        q: &str,
        brand: Option<&str>,
        category: Option<&str>,
        series: Option<&str>,
    ) -> Vec<u32> {
        let ql = q.trim().to_lowercase();
        let code_like = !ql.is_empty()
            && !ql.chars().any(char::is_whitespace)
            && ql.chars().any(|c| c.is_ascii_digit())
            && ql.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/'));
        let mut out = Vec::new();
        let mut broad = false;
        loop {
            out.clear();
            for (i, a) in self.alarms.iter().enumerate() {
                let cat = &self.catalogs[a.catalog as usize];
                if let Some(b) = brand {
                    if cat.brand != b {
                        continue;
                    }
                }
                if let Some(c) = category {
                    if cat.category != c {
                        continue;
                    }
                }
                if let Some(sr) = series {
                    if cat.series != sr {
                        continue;
                    }
                }
                let hit = if ql.is_empty() {
                    true
                } else if code_like && !broad {
                    a.code_key.contains(&ql) || a.name.to_lowercase().contains(&ql)
                } else {
                    ql.split_whitespace()
                        .all(|t| a.search.contains(t) || a.code_key.contains(t))
                };
                if hit {
                    out.push(i as u32);
                }
            }
            // 码式查询无结果 → 退回全文检索（可搜到如 h00-00 这类参数名）
            if code_like && !broad && out.is_empty() {
                broad = true;
                continue;
            }
            break;
        }
        out
    }

    /// 某报警的详情区间（details 已按 alarm 排序）
    pub fn details_range(&self, alarm_idx: u32) -> std::ops::Range<usize> {
        let lo = self.details.partition_point(|d| d.alarm < alarm_idx);
        let hi = self.details.partition_point(|d| d.alarm <= alarm_idx);
        lo..hi
    }

    #[allow(dead_code)]
    pub fn details_of(&self, alarm_idx: u32) -> &[Detail] {
        let r = self.details_range(alarm_idx);
        &self.details[r]
    }

    /// 某详情的来源行（provenance 已按 detail 排序）
    pub fn provenance_of(&self, detail_idx: u32) -> &[Provenance] {
        let lo = self.provenance.partition_point(|p| p.detail < detail_idx);
        let hi = self.provenance.partition_point(|p| p.detail <= detail_idx);
        &self.provenance[lo..hi]
    }

    /// 品牌列表（去重，保持出现顺序）
    pub fn brands(&self) -> Vec<String> {
        dedup(self.catalogs.iter().map(|c| c.brand.clone()).collect())
    }

    /// 类别列表（去重）
    pub fn categories(&self) -> Vec<String> {
        dedup(self.catalogs.iter().map(|c| c.category.clone()).collect())
    }

    /// 按品牌/类别过滤后的 (catalog 下标, 序列名)，保持目录顺序、按序列名去重；空序列跳过。
    pub fn series_entries(
        &self,
        brand: Option<&str>,
        category: Option<&str>,
    ) -> Vec<(usize, String)> {
        let mut out: Vec<(usize, String)> = Vec::new();
        for (i, c) in self.catalogs.iter().enumerate() {
            if let Some(b) = brand {
                if c.brand != b {
                    continue;
                }
            }
            if let Some(k) = category {
                if c.category != k {
                    continue;
                }
            }
            if !c.series.is_empty() && !out.iter().any(|(_, s)| s == &c.series) {
                out.push((i, c.series.clone()));
            }
        }
        out
    }

    /// 系列列表（按当前品牌 / 类别过滤；保持目录顺序、去重）
    pub fn series_list(&self, brand: Option<&str>, category: Option<&str>) -> Vec<String> {
        self.series_entries(brand, category)
            .into_iter()
            .map(|(_, s)| s)
            .collect()
    }
}

/// q 需已 trim + 小写；空串匹配全部。
/// 匹配 Catalog 的 series、任一 models、source_path；不含 brand/category。
pub fn series_matches(c: &Catalog, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    c.series.to_lowercase().contains(q)
        || c.models.iter().any(|m| m.to_lowercase().contains(q))
        || c.source_path.to_lowercase().contains(q)
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for x in v {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(brand: &str, category: &str, series: &str, source_path: &str, models: &[&str]) -> Catalog {
        Catalog {
            brand: brand.to_string(),
            category: category.to_string(),
            series: series.to_string(),
            source_path: source_path.to_string(),
            models: models.iter().map(|m| m.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn series_matches_hits_series_models_and_source_path_case_insensitively() {
        let c = cat("汇川", "伺服", "DDR系列", "汇川伺服/DDR.xlsx", &["SV660N", "SV680"]);
        assert!(series_matches(&c, ""), "空查询应匹配全部");
        assert!(series_matches(&c, "ddr"), "应命中 series 中的 ddr");
        assert!(series_matches(&c, "sv660n"), "应命中 models 中的型号");
        assert!(series_matches(&c, "ddr.xlsx"), "应命中 source_path");
        assert!(!series_matches(&c, "不存在的词"), "不相关查询不应命中");
    }

    #[test]
    fn series_matches_ignores_brand_and_category() {
        let c = cat("汇川", "伺服", "IS620系列", "docs/is620.xlsx", &["IS620"]);
        assert!(!series_matches(&c, "汇川"), "brand 不参与匹配");
        assert!(!series_matches(&c, "伺服"), "category 不参与匹配");
    }

    #[test]
    fn series_entries_filters_dedups_skips_empty_and_keeps_order() {
        let db = AlarmDb {
            catalogs: vec![
                cat("汇川", "伺服", "DDR系列", "a.xlsx", &[]), // 0
                cat("汇川", "伺服", "DDR系列", "b.xlsx", &[]), // 1 同名序列
                cat("汇川", "伺服", "", "c.xlsx", &[]),        // 2 空序列
                cat("西门子", "伺服", "S120", "d.xlsx", &[]),  // 3
                cat("汇川", "变频器", "MD500", "e.xlsx", &[]), // 4
                cat("西门子", "变频器", "G120", "f.xlsx", &[]), // 5
            ],
            ..Default::default()
        };
        assert_eq!(
            db.series_entries(None, None),
            vec![
                (0, "DDR系列".to_string()),
                (3, "S120".to_string()),
                (4, "MD500".to_string()),
                (5, "G120".to_string()),
            ],
            "全量应保持目录顺序、按序列名去重、跳过空序列"
        );
        assert_eq!(
            db.series_entries(Some("汇川"), None),
            vec![(0, "DDR系列".to_string()), (4, "MD500".to_string())],
            "按品牌过滤应只保留汇川"
        );
        assert_eq!(
            db.series_entries(None, Some("变频器")),
            vec![(4, "MD500".to_string()), (5, "G120".to_string())],
            "按类别过滤应只保留变频器"
        );
        assert_eq!(
            db.series_entries(Some("西门子"), Some("伺服")),
            vec![(3, "S120".to_string())],
            "品牌与类别应同时生效"
        );
        assert!(
            db.series_entries(Some("高创"), None).is_empty(),
            "无匹配品牌时应为空"
        );
        assert_eq!(
            db.series_list(None, None),
            db.series_entries(None, None)
                .into_iter()
                .map(|(_, s)| s)
                .collect::<Vec<_>>(),
            "series_list 名称序列应与 series_entries 一致"
        );
    }

    #[test]
    fn series_entries_and_matches_real_db_if_present() {
        let p = std::path::Path::new("data/alarms.bin");
        if !p.exists() {
            eprintln!("data/alarms.bin 不存在，跳过");
            return;
        }
        let db = AlarmDb::load(p).expect("alarms.bin 加载失败");
        let entries = db.series_entries(None, None);
        let list = db.series_list(None, None);
        assert_eq!(
            entries.len(),
            list.len(),
            "series_entries 与 series_list 长度应一致"
        );
        assert_eq!(
            entries.iter().map(|(_, s)| s.clone()).collect::<Vec<_>>(),
            list,
            "series_entries 与 series_list 顺序应一致"
        );
        let (idx, _) = entries
            .iter()
            .find(|(_, s)| s == "DDR系列")
            .expect("系列列表中应存在 DDR系列");
        assert!(
            series_matches(&db.catalogs[*idx], "ddr"),
            "DDR系列 catalog 应以 ddr 命中"
        );
    }

    /// 若 data/alarms.bin 存在则做一次真实数据加载校验（由 tools/convert_alarms.py 生成）
    #[test]
    fn load_real_db_if_present() {
        let p = std::path::Path::new("data/alarms.bin");
        if !p.exists() {
            eprintln!("data/alarms.bin 不存在，跳过");
            return;
        }
        let db = AlarmDb::load(p).expect("alarms.bin 加载失败");
        assert_eq!(db.catalogs.len(), 129, "catalogs 数量不符");
        assert_eq!(db.alarms.len(), 13765, "alarms 数量不符");
        assert_eq!(db.details.len(), 14167, "details 数量不符");
        assert_eq!(db.provenance.len(), 14221, "provenance 数量不符");

        // 回归：多码拆分（原表 "Er.220：内部故障\nEr.234：飞车" 合并行）
        // 1) 检索 234 不应命中任何含 220 的码；应含 Er.234
        let h234 = db.search("234", None, None, None);
        assert!(!h234.is_empty(), "搜索 234 应有结果");
        assert!(
            h234.iter().any(|&i| db.alarms[i as usize].code_key.contains("234")),
            "搜索 234 应包含 Er.234 记录"
        );
        assert!(
            h234.iter().all(|&i| !db.alarms[i as usize].code_key.contains("220")),
            "搜索 234 不应命中 220 记录"
        );
        // 2) Er.220 名称已清洗（不带第二码、无换行）
        let h220 = db.search("er.220", None, None, None);
        let a220s: Vec<_> = h220
            .iter()
            .map(|&i| &db.alarms[i as usize])
            .filter(|a| a.code_key == "er.220")
            .collect();
        assert!(!a220s.is_empty(), "搜索 er.220 应有结果");
        assert!(
            a220s.iter().any(|a| a.name == "内部故障"),
            "应存在清洗后的 Er.220 内部故障"
        );
        assert!(
            a220s.iter().all(|a| !a.name.contains("234") && !a.name.contains('\n')),
            "Er.220 名称不应再包含 234 / 换行"
        );

        // 抽样：搜索 & 数据链路
        let hits = db.search("sto", None, None, None);
        assert!(!hits.is_empty(), "搜索 'sto' 应有结果");

        // 系列过滤：DDR系列 + 234
        let h_ddr = db.search("234", None, None, Some("DDR系列"));
        assert!(!h_ddr.is_empty(), "DDR 系列下搜索 234 应有结果");
        assert!(
            h_ddr
                .iter()
                .all(|&i| db.catalog(&db.alarms[i as usize]).series == "DDR系列"),
            "系列过滤后应全部为 DDR系列"
        );
        // 系列列表联动
        let all_series = db.series_list(None, None);
        assert!(all_series.contains(&"DDR系列".to_string()));
        let hc_series = db.series_list(Some("汇川"), None);
        assert!(hc_series.contains(&"DDR系列".to_string()));
        assert!(hc_series.len() <= all_series.len());
        let idx = hits[0];
        let alarm = &db.alarms[idx as usize];
        let cat = db.catalog(alarm);
        assert!(!cat.brand.is_empty(), "品牌不应为空");
        let r = db.details_range(idx);
        if !r.is_empty() {
            let prov = db.provenance_of(r.start as u32);
            let _ = prov;
        }
        println!(
            "加载 OK：{} 报警 / {} 详情 / {} 来源行",
            db.alarms.len(),
            db.details.len(),
            db.provenance.len()
        );
    }
}
