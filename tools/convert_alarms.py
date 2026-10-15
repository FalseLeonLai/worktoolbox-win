#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
将原 App 的 alarms.sqlite 转换为工作工具箱(Windows 版) 使用的紧凑数据文件。

输出：app/data/alarms.bin —— postcard 二进制格式，
与 app/src/data/alarm.rs 中的结构体（AlarmDb/Catalog/SheetInfo/Alarm/Detail/Provenance）
字段顺序一一对应。

用法：  python3 tools/convert_alarms.py
（源库：ref/assets/builtin/alarms.sqlite；重新生成数据时再跑一次即可）
"""
import json
import os
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
    catalogs = []          # (brand, category, series, source_path, models, alarm_count, sheets)
    sheet_name_idx = {}    # cat_idx -> {sheet_name: sheet_idx}
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

    # ---------- alarms（按 目录 → 报警码 排序） ----------
    tmp = []
    for aid, cid, code, code_key, name, title, search_text, dc in cur.execute(
        "SELECT id, catalog_id, code, code_key, name, title, search_text, detail_count FROM alarms"
    ):
        ci = cat_index.get(cid)
        if ci is None:
            continue
        brand, category, series, _, _, _, _ = catalogs[ci]
        search = (search_text or "").lower()
        if not search:
            search = f"{brand} {category} {series} {code} {name}".lower()
        tmp.append({
            "aid": aid, "cit": ci, "code": code or "", "code_key": code_key or "",
            "name": name or "", "title": title or "", "search": search, "dc": int(dc or 0),
        })
    tmp.sort(key=lambda r: (r["cit"], r["code"]))
    alarm_index = {r["aid"]: n for n, r in enumerate(tmp)}

    # ---------- details（按新 alarm 下标排序） ----------
    tmp_d = []
    for did, aid, cause, verification, action, doc_ref, model in cur.execute(
        "SELECT id, alarm_id, cause, verification, action, document_ref, raw_model FROM details"
    ):
        ai = alarm_index.get(aid)
        if ai is None:
            continue
        tmp_d.append({"did": did, "ai": ai, "cause": cause or "", "ver": verification or "",
                      "act": action or "", "doc": doc_ref or "", "model": model or ""})
    tmp_d.sort(key=lambda r: r["ai"])
    detail_index = {r["did"]: n for n, r in enumerate(tmp_d)}

    # ---------- provenance（按新 detail 下标排序；单元格只留 value，压缩掉 type/format 等） ----------
    tmp_p = []
    for did, sheet_name, srow, raw_json in cur.execute(
        "SELECT detail_id, source_sheet, source_row, raw_json FROM provenance"
    ):
        di = detail_index.get(did)
        if di is None:
            continue
        ai = tmp_d[di]["ai"]
        cit = tmp[ai]["cit"]
        name_map = sheet_name_idx[cit]
        sname = sheet_name or ""
        if sname not in name_map:
            # 原始表里没有该 sheet 记录时，补一个占位（不丢信息）
            catalogs[cit][6].append((sname, []))
            name_map[sname] = len(catalogs[cit][6]) - 1
        sheet_idx = name_map[sname]
        assert sheet_idx < 256, "sheet 下标超出 u8"
        cells_json = load_json_list(raw_json)
        cells = [str(c.get("value", "")) for c in sorted(cells_json, key=lambda c: c.get("column", 0))]
        tmp_p.append({"di": di, "sheet": sheet_idx, "row": int(srow or 0), "cells": cells})
    tmp_p.sort(key=lambda r: r["di"])

    # ---------- 编码 ----------
    buf = bytearray()
    # AlarmDb.catalogs
    cat_enc = []
    for brand, category, series, source_path, models, alarm_count, sheets in catalogs:
        e = s(brand) + s(category) + s(series) + s(source_path)
        e += v([s(m) for m in models])
        e += i(alarm_count)
        e += v([s(name) + v([s(h) for h in headers]) for (name, headers) in sheets])
        cat_enc.append(e)
    buf += v(cat_enc)
    # AlarmDb.alarms
    buf += v([
        i(r["cit"]) + s(r["code"]) + s(r["code_key"]) + s(r["name"]) + s(r["title"]) + s(r["search"]) + i(r["dc"])
        for r in tmp
    ])
    # AlarmDb.details
    buf += v([
        i(r["ai"]) + s(r["cause"]) + s(r["ver"]) + s(r["act"]) + s(r["doc"]) + s(r["model"])
        for r in tmp_d
    ])
    # AlarmDb.provenance
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
