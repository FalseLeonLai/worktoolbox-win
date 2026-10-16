#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
将原 App 的 alarms.sqlite 转换为工作工具箱(Windows 版) 使用的紧凑数据文件。

输出：app/data/alarms.bin —— postcard 二进制格式，
与 app/src/data/alarm.rs 中的结构体（AlarmDb/Catalog/SheetInfo/Alarm/Detail/Provenance）
字段顺序一一对应。

v2 数据清洗（2026-10-16，修复「检索 234 出现 220」问题）：
- 原始表格存在「一条记录打包多个报警码」的行（如标题 "Er.220：内部故障\nEr.234：飞车"），
  拆分为多条独立报警记录（每个码单独成条、共享同一详情块），任一码检索都能精确命中；
- 名称/标题里不必要的换行合并为单行（如 "电子齿轮保\n护" → "电子齿轮保护"）；
- 空 code 记录从标题的「码：名称」结构推导（如 "E.740.0、…、E.740.8：编码器干扰"）；
- search 检索文本统一重建（品牌/类别/系列/码/名称/型号/原因/确认/措施/文档）。

用法：  python3 tools/convert_alarms.py
（源库：ref/assets/builtin/alarms.sqlite；重新生成数据时再跑一次即可）
"""
import json
import os
import re
import sqlite3
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "ref", "assets", "builtin", "alarms.sqlite")
OUT = os.path.join(ROOT, "app", "data", "alarms.bin")


# ---------------- postcard 基础编码 ----------------
def varint(n: int) -> bytes:
    out = bytearray()
    n = int(n)
    assert n >= 0
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def s(x: str) -> bytes:
    """postcard string：长度 varint + UTF-8 字节"""
    b = (x or "").encode("utf-8")
    return varint(len(b)) + b


def v(items) -> bytes:
    """postcard Vec<T>：元素个数 varint + 依次编码的元素"""
    return varint(len(items)) + b"".join(items)


def i(n) -> bytes:
    """postcard 整数（varint）"""
    return varint(int(n))


def load_json_list(text: str):
    try:
        x = json.loads(text or "[]")
        return x if isinstance(x, list) else []
    except Exception:
        return []


# ---------------- 数据清洗 ----------------
CODE_RE = re.compile(r"^[A-Za-z][A-Za-z0-9.．]*[0-9][A-Za-z0-9.．]*$")   # 单个报警码
CODES_LINE_RE = re.compile(r"^[A-Za-z0-9.．、,，~～\s]+$")               # 码列表（含 、 分隔）


def norm_code(c: str) -> str:
    """规范化报警码：小写、去空格、全角点转半角"""
    return (c or "").lower().replace(" ", "").replace("．", ".")


def _join_sep(a: str, b: str) -> str:
    """合并两行时的连接符：两边都是码字符且 a 未以分隔符结尾 → 补「、」"""
    if (
        a
        and b
        and (a[-1] not in "、，,：:；;-（( /")
        and re.search(r"[A-Za-z0-9.]$", a)
        and re.match(r"^[A-Za-z0-9.]", b)
    ):
        return "、"
    return ""


def _merge_lines(text: str) -> str:
    lines = [l.strip() for l in (text or "").replace("\r", "").split("\n") if l.strip()]
    if not lines:
        return ""
    out = lines[0]
    for ln in lines[1:]:
        out += _join_sep(out, ln) + ln
    return out


def clean_record(code: str, name: str, title: str):
    """清洗一条记录 → [(code, name), ...]（多码拆分时返回多条，首个为主码）"""
    src = (title or name or "").replace("\r", "")
    lines = [l.strip() for l in src.split("\n") if l.strip()]

    # ① 多码分行：每行形如「码：名称」→ 拆分为多条
    if len(lines) >= 2:
        pairs, ok = [], True
        for ln in lines:
            if "：" not in ln:
                ok = False
                break
            c, n = ln.split("：", 1)
            c, n = c.strip(), n.strip()
            if not c or not n or not CODE_RE.match(c):
                ok = False
                break
            pairs.append((c, n))
        if ok:
            seen, uniq = set(), []
            for c, n in pairs:
                if c not in seen:
                    seen.add(c)
                    uniq.append((c, n))
            if len(uniq) >= 2:
                return uniq

    # ② 合并不必要的换行（仅处理有换行或空码的记录；单行正常记录原样保留）
    if len(lines) <= 1 and "\n" not in (name or "") and "\n" not in (title or "") and (code or "").strip():
        return [((code or "").strip(), (name or "").strip())]
    joined = _merge_lines(src)
    n0 = _merge_lines(name)
    c0 = (code or "").strip()
    if "：" in joined:
        left, right = joined.split("：", 1)
        left, right = left.strip(), right.strip()
        # 空 code：若左侧是码列表则采用
        if not c0 and left and CODES_LINE_RE.match(left) and re.search(r"\d", left):
            c0 = left
        if right:
            n0 = right
    elif len(lines) > 1 and not n0:
        n0 = joined
    return [(c0, n0)]


def main() -> int:
    if not os.path.exists(SRC):
        print(f"源数据库不存在: {SRC}", file=sys.stderr)
        return 1
    con = sqlite3.connect(f"file:{SRC}?mode=ro", uri=True)
    cur = con.cursor()

    # ---------- catalogs（品牌/类别/系列 排序，保证确定性） ----------
    cat_rows = cur.execute(
        "SELECT id, brand, category, series, source_path, source_models, alarm_count "
        "FROM catalogs ORDER BY brand, category, series"
    ).fetchall()
    cat_index = {}
    catalogs = []
    sheet_name_idx = {}
    for idx, (cid, brand, category, series, source_path, models_json, alarm_count) in enumerate(cat_rows):
        cat_index[cid] = idx
        models = [str(m) for m in load_json_list(models_json)]
        sheets = []
        for name, headers_json in cur.execute(
            "SELECT name, headers_json FROM sheets WHERE catalog_id=? ORDER BY rowid", (cid,)
        ).fetchall():
            headers = [str(h) for h in load_json_list(headers_json)]
            sheets.append((name or "", headers))
        catalogs.append((brand or "", category or "", series or "", source_path or "",
                         models, int(alarm_count or 0), sheets))
        sheet_name_idx[idx] = {name: n for n, (name, _) in enumerate(sheets)}

    # ---------- 第一遍：现有码集合（用于拆分去重） + 原始行清单 ----------
    raw_rows = cur.execute(
        "SELECT id, catalog_id, code, code_key, name, title, search_text, detail_count FROM alarms"
    ).fetchall()
    existing_keys = {}   # cit -> set(norm_code)
    for _aid, cid, code, _ck, _name, _title, _st, _dc in raw_rows:
        ci = cat_index.get(cid)
        if ci is None:
            continue
        existing_keys.setdefault(ci, set()).add(norm_code(code))

    # ---------- 第二遍：清洗 + 拆分拼接 ----------
    tmp = []              # 报警行（uid 唯一）
    copies = {}           # 原始 alarm_id -> [uid, ...]（详情将复制给每个 uid）
    extras_added = extras_skipped = 0
    for aid, cid, code, code_key, name, title, search_text, dc in raw_rows:
        ci = cat_index.get(cid)
        if ci is None:
            continue
        cleaned = clean_record(code, name, title)
        uids = []
        for k, (c, n) in enumerate(cleaned):
            nc = norm_code(c)
            if k > 0:
                if nc in existing_keys.get(ci, set()):
                    extras_skipped += 1
                    continue
                extras_added += 1
                existing_keys.setdefault(ci, set()).add(nc)
            uid = f"{aid}#{k}"
            uids.append(uid)
            tmp.append({
                "uid": uid, "cit": ci, "code": c, "code_key": nc,
                "name": n, "title": f"{c}：{n}" if c else n,
                "dc": 0,  # 详情计数稍后统计
            })
        copies[aid] = uids

    tmp.sort(key=lambda r: (r["cit"], r["code"]))
    alarm_index = {r["uid"]: n for n, r in enumerate(tmp)}

    # ---------- details（多码记录：详情复制给每个拼接出的码） ----------
    tmp_d = []            # 含 did 与 ai
    search_parts = {}     # uid -> [文本片段]
    for did, aid, cause, verification, action, doc_ref, model in cur.execute(
        "SELECT id, alarm_id, cause, verification, action, document_ref, raw_model FROM details"
    ):
        for uid in copies.get(aid, []):
            ai = alarm_index.get(uid)
            if ai is None:
                continue
            tmp_d.append({"did": did, "ai": ai, "cause": cause or "", "ver": verification or "",
                          "act": action or "", "doc": doc_ref or "", "model": model or ""})
            search_parts.setdefault(uid, []).append(
                f"{cause or ''} {verification or ''} {action or ''} {model or ''}"
            )
    tmp_d.sort(key=lambda r: r["ai"])
    detail_by_did = {}
    for n, r in enumerate(tmp_d):
        detail_by_did.setdefault(r["did"], []).append(n)
    # 详情计数回填
    for r in tmp_d:
        tmp[r["ai"]]["dc"] += 1

    # ---------- provenance（复制到每个详情副本） ----------
    tmp_p = []
    for did, sheet_name, srow, raw_json in cur.execute(
        "SELECT detail_id, source_sheet, source_row, raw_json FROM provenance"
    ):
        for di in detail_by_did.get(did, []):
            ai = tmp_d[di]["ai"]
            cit = tmp[ai]["cit"]
            name_map = sheet_name_idx[cit]
            sname = sheet_name or ""
            if sname not in name_map:
                catalogs[cit][6].append((sname, []))
                name_map[sname] = len(catalogs[cit][6]) - 1
            sheet_idx = name_map[sname]
            assert sheet_idx < 256, "sheet 下标超出 u8"
            cells_json = load_json_list(raw_json)
            cells = [str(c.get("value", "")) for c in sorted(cells_json, key=lambda c: c.get("column", 0))]
            tmp_p.append({"di": di, "sheet": sheet_idx, "row": int(srow or 0), "cells": cells})
    tmp_p.sort(key=lambda r: r["di"])

    # ---------- search 检索文本重建 ----------
    for r in tmp:
        brand, category, series, _, _, _, _ = catalogs[r["cit"]]
        parts = " ".join(search_parts.get(r["uid"], []))
        r["search"] = f"{brand} {category} {series} {r['code']} {r['name']} {parts}".lower()

    # ---------- 编码 ----------
    buf = bytearray()
    cat_enc = []
    for brand, category, series, source_path, models, alarm_count, sheets in catalogs:
        e = s(brand) + s(category) + s(series) + s(source_path)
        e += v([s(m) for m in models])
        e += i(alarm_count)
        e += v([s(name) + v([s(h) for h in headers]) for (name, headers) in sheets])
        cat_enc.append(e)
    buf += v(cat_enc)
    buf += v([
        i(r["cit"]) + s(r["code"]) + s(r["code_key"]) + s(r["name"]) + s(r["title"]) + s(r["search"]) + i(r["dc"])
        for r in tmp
    ])
    buf += v([
        i(r["ai"]) + s(r["cause"]) + s(r["ver"]) + s(r["act"]) + s(r["doc"]) + s(r["model"])
        for r in tmp_d
    ])
    buf += v([
        i(r["di"]) + i(r["sheet"]) + i(r["row"]) + v([s(c) for c in r["cells"]])
        for r in tmp_p
    ])

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "wb") as f:
        f.write(buf)

    print(f"OK -> {OUT}")
    print(f"大小: {len(buf)/1048576:.2f} MB")
    print(f"catalogs={len(catalogs)}  alarms={len(tmp)}  details={len(tmp_d)}  provenance={len(tmp_p)}")
    print(f"多码拆分：新增 {extras_added} 条，跳过已存在 {extras_skipped} 条")
    return 0


if __name__ == "__main__":
    sys.exit(main())
