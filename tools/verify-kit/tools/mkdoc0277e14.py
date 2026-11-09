#!/usr/bin/env python3
# mkdoc0277e14.py: docs/0277-e14-verification.md (for the PM's ingest) from the E14 result files (tables copied, not retyped) and the prose below
import re, os, glob
E = '/home/ubuntu/work/signature-studio-develop/lift/out-git/ci1-results/0277/e14'
def rd(p): return open(E + '/' + p).read().strip('\n')
def section(txt, title, level='##'):
    m = re.search(r'^' + re.escape(level) + ' ' + re.escape(title) + r'.*?$\n(.*?)(?=^#{2,3} |\Z)', txt, re.S | re.M)
    return m.group(1).strip('\n') if m else '(missing: ' + title + ')'
R = rd('RESULTS-E14.md')
pages = rd('PAGES-e14.md')
b3 = pages[pages.index('## B3 = P + V7'):] if '## B3 = P + V7' in pages else pages
fi = rd('RESULTS-E14-first-idle.md')
fi_body = fi[fi.index('\n## Chrome'):]
pure = rd('RESULTS-E14-pure.md')
pure_body = pure[pure.index('\n## 中央値'):]
p3 = rd('data/P3-chrome.md')
g9 = rd('g9a-export/RESULTS-g9a-export.md')
g9_body = g9[g9.index('| 判定 |'):]
dem = lambda t: re.sub(r'(?m)^## ', '### ', re.sub(r'(?m)^# .*\n', '', t, count=1))
b3, fi, pure, g9 = dem(b3), dem(fi), dem(pure), dem(g9)
st_cm_a, st_cm_b, st_cm_d = '4,591', '4,570', '−21.5'
dev = rd('deviations.md') if os.path.exists(E + '/deviations.md') else ''
doc = f"""# ci1 → PM 決定 0277（E14）: E14 の最終の組 P + V7（`page-e14-PV7`、port 5063、bundle は E13 の本番 `fl-engine-ci-0363b050`）の実ブラウザ確認 — 非計時の確認（15 シーン・File ×100・Export・alpha7）、first idle（E13 比、Chrome 10 組・Firefox 6 組）、純再生 pure（E13 対 P + V7、Chrome 6 組）、G9-a の test bundle の Export（参考）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158〔Xvfb :77、WebGL あり、llvmpipe〕、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。各値に load と I/O 圧 avg10、swap の入出力を併記し、計時の run は『開始時 load ≤8・I/O 圧 0・swap の入出力 0』の run だけ数えた〔first idle は全 16 組〔32 run〕、pure は 12 run が数えた run〕）
1. **測ったこと**: PM の検証済みの木（`lift/out-pmchk-e14f/tree/app` = main の app + gpu1 の present Worker P〔0311〕+ V7）の page を ci1 の kit で `vite build` し（`page-e14-PV7`、`vite.kit.config.ts` の追加以外は木と byte 一致、P 単独の page との差は `desk-worker.ts` の +4 / −1 だけ）、E13 の本番 bundle `fl-engine-ci-0363b050`（R2 から読み取りで取得した物）と組み合わせた。A = E13 の page（5053、d0447902）対 B = P + V7（5063）で: 非計時の確認（Chrome 15 シーン、File ×100、Export、alpha7）、first idle（Chrome 10 組・Firefox 6 組、交互）、純再生 pure（Chrome 6 組・12 run、交互）。別に、gpu1 の port 4969 の G9-a の test bundle の Export を 1 本（参考。製品の bundle ではない）。
2. **結論**:
   - **first idle は基準（E13 比 +100 ms 以内）を両ブラウザで満たす**: Chrome の中央値 A {st_cm_a} / B {st_cm_b} ms、組ごとの差（B − A）の中央値 −54 ms、平均 −46 ± 47（SE）、7/10 が負、両側 p 0.38。Firefox は 4,206 / 4,107 ms、差の中央値 −136 ms、平均 −86 ± 161（SE）、4/6 が負、p 0.63。外れ値なし。P の page は first idle を遅らせていない。
   - **純再生 pure は P + V7 が時計を実時間に近づけ、書き込み率と underrun は区別できない**: 仮想 s/実 s 0.656 → 0.912、present/s 13.6 → 29.4、lagCuts/s 18.2 → 3.8（いずれも p 0.002）、音楽 10 s の bin の壁時計 14〜17 s → 約 10.4〜11.1 s。書き込み率（10〜60 s）0.912 → 0.937（p 0.35）、60 s の underrun quanta 7.5 → 8.5（p 0.65）、30〜60 s は両方 0。**代償**: Worker スレッド全体（engine Worker + P Worker）の CPU が 60 s で 51.0 → 73.5 s（+44%）。**基準が動いている**: 同じ A（E13）が 01:47 の E13 の pure より遅いので、A 対 B の同窓の差だけを読む。
   - 非計時の確認はすべて通った: Chrome 15/15 シーン ok、File ×100 = 100/100、Export の WAV は 22,863,828 B・sha256 `381cdb5a…f979` で参照と一致、alpha7 の画は E13 と 0 画素差（主窓 @0,0 1920x1080）。
   - **G9-a の test bundle（参考）**: 完走、`fle: unsupported` 0 行、g9-diag 2 行、WAV の sha256 は `94c93666…4090`（基準と違う。差は Balance の自動化 clip の内側とその終端の尾だけ〔alpha1 の解析、0310 §6〕）。Wine の参照との照合待ちで G9 は未到達。
3. **未確認**: 依頼者の機械（0 件）。**e2e（`test:e2e` preview → dev → worker）は未実施**（mt3 の build の後の PM の info を待つ。手順は `E2E-INVENTORY.md`）。Firefox の 15 シーン・File ×100・Export・alpha7、lat（入力の true T）、5 分再生、warm の first idle は未実施。Esc の late の原因は gpu1 の領域。

## 1 判定基準（手で作成。`report.py` の判定表は P の page が beacon に `desk.mode` を出さず A と B が別の mode の行になるため、改修〔`desk.mode` が None で `webglRenderer` が非 null の run を gl とみなす〕後の出力を `RESULTS-E14.md` に置いた）
| 基準 | 値 | 判定 |
|---|---|---|
| 停止なし（Chrome の 15 シーン、d7 は意図した終了） | 15/15 ok | yes（Firefox は未実施） |
| Export の WAV が参照と一致 | 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` | yes |
| File ×100 | Chrome 100/100（Firefox 未実施） | yes |
| first idle ≤ E13 + 100 ms（Chrome、WebGL） | 中央値 B − A = {st_cm_d} ms（組ごとの差の中央値 −54 ms）、10 組 | yes |
| first idle ≤ E13 + 100 ms（Firefox、WebGL あり） | 中央値 B − A = −99.5 ms（組ごとの差の中央値 −136 ms）、6 組 | yes |
| 純再生が E13 より悪化しない | 書き込み率・underrun は区別できない、仮想 s/実 s と present は改善 | yes（代償: Worker の CPU +44%） |
| alpha7（E13 と画素比較） | 0 画素差 | yes |

## 2 page（B3 = P + V7）
{section(b3, 'B3 = P + V7', '##') if False else b3}

## 3 first idle（A = E13 対 B = P + V7）
{fi}

## 4 純再生 pure（A = E13 対 B = P + V7、Chrome 6 組）
{pure}

### 4.1 `p3-win.py` の全文（表の原文）
{p3}

## 5 非計時の確認の表（`RESULTS-E14.md` から。first idle の値は mt3 の build が走っていた時間の run で参考）
### 5.1 シーン
{section(R, 'Scenes (new bundle)')}
### 5.2 alpha7
{section(R, 'alpha7 (main window geometry, guest 1920x1080, Chrome)')}
### 5.3 Export
{section(R, 'Export (Chrome, the reference project, Ctrl+R)')}

## 6 G9-a の test bundle の Export（参考。製品の bundle ではない。gpu1 の port 4969、非計時）
{g9}

## 7 e2e の棚卸し
`E2E-INVENTORY.md`（CI が自動で走らせる物、kit に無い物、最終 page を E13 の bundle で通す e2e の手順、P の経路で engine-desk 系の読み出しが通るかの読み）。**e2e は未実施**。

## 8 自己申告の逸脱
{dev}
"""
os.makedirs(E + '/docs', exist_ok=True); open(E + '/docs/0277-e14-verification.md', 'w').write(doc)
print(len(doc), 'chars')
