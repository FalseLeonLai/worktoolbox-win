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

    /// 搜索：q 支持空格分词（全部命中）；brand / category 为精确过滤（None = 不限）
    pub fn search(&self, q: &str, brand: Option<&str>, category: Option<&str>) -> Vec<u32> {
        let terms: Vec<String> = q
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let mut out = Vec::new();
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
            if terms
                .iter()
                .all(|t| a.search.contains(t.as_str()) || a.code_key.contains(t.as_str()))
            {
                out.push(i as u32);
            }
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
        assert_eq!(db.alarms.len(), 13754, "alarms 数量不符");
        assert_eq!(db.details.len(), 14156, "details 数量不符");
        assert_eq!(db.provenance.len(), 14210, "provenance 数量不符");

        // 抽样：搜索 & 数据链路
        let hits = db.search("sto", None, None);
        assert!(!hits.is_empty(), "搜索 'sto' 应有结果");
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
