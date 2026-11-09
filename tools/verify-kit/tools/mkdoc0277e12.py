#!/usr/bin/env python3
# mkdoc0277e8.py: docs/0277-e8-verification.md (for the PM's ingest) from RESULTS-E12.md (tables copied, not retyped), out/title/report.json and the prose below
import re, json, os
E = '/home/ubuntu/work/signature-studio-develop/lift/out-git/ci1-results/0277/e12'
K = '/home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit'
R = open(E + '/RESULTS-E12.md').read()

def section(title, level='##'):
    m = re.search(r'^' + re.escape(level) + ' ' + re.escape(title) + r'.*?$\n(.*?)(?=^#{2,3} |\Z)', R, re.S | re.M)
    return m.group(1).strip('\n') if m else '(missing: ' + title + ')'

p3 = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/P3-*.md')))
summary = open(E + '/summary-text.md').read().strip() if os.path.exists(E + '/summary-text.md') else ''
p3 = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/P3-*.md')))
long_txt = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/LONG-*.md'))) or '(5 分再生は未実施。gpu1 の実スレッド版の窓の後に取る)'
doc = f"""# ci1 → PM 決定 0277（E12）: E12 の本番 bundle `fl-engine-ci-7a9b0800`（Worker e9565033、ページ = a977401d）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle（E11 比、16 組と遅い 2 本の分析）、alpha7、純再生 pure（E11 対 E12、壁時計と音楽位置）

{summary}

## 1 判定基準（E12 対 E11、run の load と I/O 圧は各表）
{section('Pass criteria')}

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
{section('Scenes (new bundle)')}

## 3 first idle（単独 run・冷間・交互。prev = E11、new = E12。基準 +100 ms）
{section('First idle')}

### 3.1 E12 の first idle の遅い 2 本（5・6 組目）の分析と追加 6 組\n{open(E + '/first-idle-analysis.md').read().strip() if os.path.exists(E + '/first-idle-analysis.md') else ''}\n\n## 4 Export
{section('Export')}

## 5 純再生 pure（E11 対 E12、参照 zip を開き Space で 60 s、10 s 窓、2 s ごとの snapshot、交互）
{p3}

## 6 5 分再生
{long_txt}

## 7 自己申告の逸脱
{open(E + '/deviations.md').read().strip() if os.path.exists(E + '/deviations.md') else ''}

## PM 記入欄（ci1 は書かない）
"""
os.makedirs(E + '/docs', exist_ok=True); open(E + '/docs/0277-e12-verification.md', 'w').write(doc)
print(len(doc), 'chars')
