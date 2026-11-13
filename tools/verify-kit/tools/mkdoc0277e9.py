#!/usr/bin/env python3
# mkdoc0277e8.py: docs/0277-e8-verification.md (for the PM's ingest) from RESULTS-E9.md (tables copied, not retyped), out/title/report.json and the prose below
import re, json, os
REPO = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
KIT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
E = REPO + '/lift/out-git/ci1-results/0277/e9'
K = KIT
R = open(E + '/RESULTS-E9.md').read()

def section(title, level='##'):
    m = re.search(r'^' + re.escape(level) + ' ' + re.escape(title) + r'.*?$\n(.*?)(?=^#{2,3} |\Z)', R, re.S | re.M)
    return m.group(1).strip('\n') if m else '(missing: ' + title + ')'

p3 = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/P3-*.md')))
title_txt = open(E + '/data/TITLE.md').read().strip() if os.path.exists(E + '/data/TITLE.md') else '(未取得)'
logo_txt = open(E + '/data/LOGO.md').read().strip() if os.path.exists(E + '/data/LOGO.md') else '(未取得)'
lat = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/LAT-*.md'))) or '(未取得)'
summary = open(E + '/summary-text.md').read().strip() if os.path.exists(E + '/summary-text.md') else ''
doc = f"""# ci1 → PM 決定 0277（E9）: E9 の本番 bundle `fl-engine-ci-62098b85`（Worker 2f390ad3、ページ = 38efb398）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle、alpha7、logo、最小化→復元（実マウスクリック）、純再生 pure（E8 対 E9）、入力の true T（lat）

{summary}

## 1 判定基準（E9 対 E8、run の load と I/O 圧は各表）
{section('Pass criteria')}

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
{section('Scenes (new bundle)')}

## 3 first idle（単独 run・冷間・交互。prev = E8、new = E9。平均 ± 標準誤差を表の上の判定行に付けた）
{section('First idle')}

## 4 Export
{section('Export')}

## 5 lid 8（ロゴ窓）の起動直後
{logo_txt}

## 6 最小化 → `#taskbar-restore` → 復元と題 '(none)'（Chrome、E9）
{title_txt}

## 7 純再生 pure（E8 対 E9、参照 zip を開き Space で 60 s、10 s 窓、旧新交互）
{p3}

## 8 入力の true T（lat、再生中と idle の File クリック・Esc、E8 対 E9 交互）
{lat}

## 9 自己申告の逸脱
{open(E + '/deviations.md').read().strip() if os.path.exists(E + '/deviations.md') else ''}

## PM 記入欄（ci1 は書かない）
"""
open(E + '/docs/0277-e9-verification.md', 'w').write(doc) if os.path.isdir(E + '/docs') else (os.makedirs(E + '/docs'), open(E + '/docs/0277-e9-verification.md', 'w').write(doc))
print(len(doc), 'chars')
