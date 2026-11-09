#!/usr/bin/env python3
# mkdoc0277e8.py: docs/0277-e8-verification.md (for the PM's ingest) from RESULTS-E8.md (tables copied, not retyped), out/title/report.json and the prose below
import re, json, os
E = '/home/ubuntu/work/signature-studio-develop/lift/out-git/ci1-results/0277/e8'
K = '/home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit'
R = open(E + '/RESULTS-E8.md').read()

def section(title, level='##'):
    m = re.search(r'^' + re.escape(level) + ' ' + re.escape(title) + r'.*?$\n(.*?)(?=^#{2,3} |\Z)', R, re.S | re.M)
    return m.group(1).strip('\n') if m else '(missing: ' + title + ')'

p3 = '\n\n'.join(open(p).read().strip() for p in sorted(__import__('glob').glob(E + '/data/P3-*.md')))
title_rows = []
tp = K + '/out/title/report.json'
if os.path.exists(tp):
    for r in json.load(open(tp)):
        pr = {p['probe']: p['result'] for p in r.get('probes') or []}
        def pl(k):
            try: j = json.loads(pr[k]); return [l for l in j['lines'] if 'Playlist' in l or 'playlist' in l]
            except Exception: return None
        def short(ls):
            if ls is None: return 'n/a'
            return '; '.join(re.sub(r'^\S+ ', '', l)[:90] for l in ls) or '(no Playlist title line)'
        title_rows.append((r['name'], r['what'], r.get('ok'), short(pl('titles-0-start')), short(pl('titles-1-minimized')), short(pl('titles-2-restored')), pr.get('tb-1-minimized', ''), pr.get('tb-2-restored', ''), r.get('loadStart'), r.get('ioStart')))
title_tab = ['| 変種 | 内容 | 起動直後のプレイリストの題の行 | 最小化中 | 復元後 | 最小化中のボタン | 復元後のボタン | load / I/O 圧（開始） |', '|---|---|---|---|---|---|---|---|']
for n, what, ok, a, b, c, t1, t2, ld, io in title_rows:
    title_tab.append(f'| {n} | {what} | {a} | {b} | {c} | `{t1[:110]}` | `{t2[:110]}` | {ld} / {io} |')
title_txt = open(K + '/out/title/TITLE.md').read().strip() if os.path.exists(K + '/out/title/TITLE.md') else '(未取得)'

doc = f'''# ci1 → PM 決定 0277（E8）: E8 の本番 bundle `fl-engine-ci-875bfd62`（Worker 4a03e043、ページ = c74b9194）の実ブラウザ確認 — 停止なし、Export 一致、File ×100、first idle、alpha7 の窓内の平行移動、最小化→`#taskbar-restore`→復元、P3（E7b 対 E8）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77`、beacon の `webglRenderer` = llvmpipe、`desk.mode` = gl）。各値に load と I/O 圧 avg10 を併記）
1. **測ったこと**: 配信バケットの bundle を読み取りで取得し manifest の全ファイルの sha256 を照合（wasm `0a54e7f86358…` 142,586,692 B、glue `ddaf06700da6…` 744,344 B、seed・dll は E7b と同じ）。同じページ（c74b9194）で E7b `fl-engine-ci-d9c160aa` と E8 を配信して、15 シーン × 2 ブラウザ、File ×100、first idle（単独・冷間・交互）、Export、alpha7 の画（E7b と並べる）、最小化 → `#taskbar-restore` → 復元（マウス・キーボード）、P3（参照 zip を 60 s 再生、10 s 窓、旧新交互）。
2. **結論**: 決定 0277 の E8 の判定項目を**満たした**。停止なし（Chrome・Firefox 各 15 シーン、Export、P3 の再生）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5a…f979`）、File ×100 は Chrome・Firefox とも 100/100、first idle は E8 の方が速い向き（Chrome 5,156 対 5,321 ms、Firefox 4,486 対 4,823 ms）。**P3 は E7b と差がない**（Chrome、p ≥ 0.24。Firefox は数えた run 各 2 で判定できないが差も見えない）。alpha7: 主窓は @0,0 1920x1080 で不変、窓の中身は 4〜6 px 平行移動（alpha1 の予測した『FL 自身が最大化を知らされる』の帰結、崩れではない）。最小化 → `#taskbar-restore` → 復元は本番 bundle でマウス（FL の最小化ボタンのクリック）→ ページのボタン（DOM の click、実マウス、Tab → Enter）で通った。
3. **未確認**: オーナーの機械（0 件）。復元後のプレイリストの題が『Pattern 1』から『(none)』になる原因（再現条件だけ第 6 節に表にした。原因は調べていない）。Firefox の P3 は E7b 対 E8 を 3 組流したが、数えられた run（開始時 load ≤8・I/O 圧 0）は各 2 で、差は検出できない（p ≥ 0.67、run ごとのばらつきが大きい。E7b の 1 run は underrun 8 quanta、他は 400 台）。ff5ee429 対 E7b の P3 決着は別の報告。

## 1 判定基準（E8 対 E7b、run の load と I/O 圧は各表）
{section('Pass criteria')}

## 2 15 シーン（Chrome・Firefox、各行の load と I/O 圧。2 ジョブ並行のため first idle は参考値）
{section('Scenes (new bundle)')}

## 3 first idle（単独 run・冷間・毎回新しいプロファイル・E7b 対 E8 交互。prev = E7b、new = E8）
{section('First idle')}

## 4 Export（Chrome、参照プロジェクトを開き Ctrl+R）
{section('Export')}

## 5 alpha7: 起動直後の画（Chrome、1920x1080 ゲスト）を E7b と並べた
- 窓表: 主窓 `TFruityLoopsMainForm 'FL Studio 2026'` は E7b・E8 とも **@0,0 1920x1080 z0**（可視の窓は 1 つで同一）。
- 画素: 全 2,073,600 画素中 709,324（34.2 %）が違う。原因は窓の中身の平行移動（部分ごとの最良の移動、残差 0〜8 画素）:

| 部分 | E7b から E8 への移動 |
|---|---|
| メニュー（FILE EDIT…）、Browser のアイコン | 左 6、上 6 |
| チャンネルラック、Sampler の行、プレイリストの Track 名、パターン一覧 | 左 4、上 6 |
| mixer（Master と Insert 1〜9） | 左 4、**下 6** |
| 右上の最小化/最大化/閉じる、プレイリストの閉じる | **右 5〜6**、上 6 |

- 目視: ツールバーが上端から始まる（E7b は上に 6 px の外枠があった）。右上の窓ボタンは右端に寄る（閉じるの右端 x≈1912 < 1920 で見える）。メニュー文字・transport・Browser・Channel rack・Sampler・mixer・プレイリストは全て見え、欠けて読めない物はない。下端は Browser の水平スクロールバーの下半分が 6 px 切れる。
- 解釈（ci1 の読み、PM が『許容、直さない』と判断）: FL の子窓が最大化した窓の外枠ぶん（約 6 px）だけ外へ配置された。実 Windows の最大化窓と同じ見え方。
- 3 枚の画の場所（`ci1-results/0277/e8/shots/`）: `alpha7-E7b-d9c160aa-main-1920x1080.png`（E7b）、`alpha7-E8-875bfd62-main-1920x1080.png`（E8）、`alpha7-E7b-vs-E8-diff.png`（差分: 違う画素が白、他は E7b を暗く）。拡大して上下に並べた切り出し（上が E7b、下が E8）: `cmp-top-left.png`、`cmp-top-right.png`、`cmp-bottom-left.png`、`cmp-bottom-right.png`。

## 6 最小化 → `#taskbar-restore` → 復元（Chrome、本番 bundle E8）
FL の最小化ボタン（ゲスト座標 1863,20 付近）をマウスでクリックし、ページのボタンで復元:

| 時点 | 窓表（可視） | `#taskbar-restore` の DOM | フォーカス |
|---|---|---|---|
| 起動直後 | 主窓 @0,0 1920x1080 の 1 窓 | `hidden` = true、display none | `desktop` |
| 最小化の 3 s 後 | **0 窓** | `hidden` = false、display inline-block、位置 x12 y1071 幅 182 高さ 33（ページ座標。desktop は y36 から 1920x1080 なので左下）、数は 1 | `desktop` |
| Tab を押した後（キーボードの経路） | 0 窓 | 同じ（x12 y1071） | **`taskbar-restore`** |
| クリック（DOM の click）/ Enter の 3 s 後 | **主窓 @0,0 1920x1080 の 1 窓（元に戻る）** | `hidden` = true | `desktop`（フォーカスが戻る） |

- エンジンは停止せず（`engine-idle`）。復元後の画: ツールバーが右へ約 15 px ずれる（既知の AlignControls）。ほかに、復元後も最小化ボタンが青くハイライト（ポインタが残る）、ヒント欄に『Minimize』が出る。
- 復元後のプレイリストの題が『Pattern 1』から『(none)』になる件の再現条件（原因は調べていない。PM が alpha1 に native で見てもらう）。窓表（page の `deskDumpBase64(true)`）には主窓 1 つしか無く（`n` = 1、題 `FL Studio 2026`、最小化中は 0 窓）、プレイリストの題は FL が描く文字なので、**画で**比べた（復元後の画の見出し部分を、起動直後の画と、前に取った『(none)』の画と画素比較）。5 変種:

{title_txt}

- **5 変種すべてで『(none)』になる（既知の『(none)』の画と 0 画素差、起動直後の『Pattern 1』とは 408 画素差）**。最小化から復元までの時間（0.5 s・3 s・15 s）にも、復元の方法（DOM の click、Tab → Enter、CDP の実マウスクリック）にも依らない。復元後の画と起動直後の画で違う画素は 41,667〜41,699 で、**全て y 0〜102 の行**（ツールバーの 15 px 右ずれ、ヒント欄の『Minimize』、プレイリストの見出し）。それ以外の窓（Browser・Channel rack・mixer・プレイリストの本体）の画素は起動直後と同一。\n- 実マウスクリック（CDP の `Input.dispatchMouseEvent`、ゲスト座標 103,1051 = ボタンの上）の経路は変種 `mouse-3s`: ボタンが押され、窓が戻った（ボタン hidden、可視の窓が 1 つ、フォーカスが desktop）。これで E8 の本番 bundle でも実マウスクリックが通ったことが確認できた。

## 7 P3: E7b 対 E8（参照 zip を開き Space で 60 s、10 s 窓、旧新交互。開始時 load ≤8 かつ I/O 圧 0 の run だけ数える）
{p3}

## 8 自己申告の逸脱
1. Firefox の d5 の最初の起動失敗（geckodriver のポート衝突 `Address already in use`、ハーネスの起動失敗）を最初の集計で見落とした（『全て ok』は結果のある行だけを数えていた）。報告前に気づき、集計に『結果なし』の行を足して再実行した（ok）。
2. P3 の最初の 1 run（Chrome）は I/O 圧の記録が無い（ドライバの記録漏れ）。以降は記録あり。
3. 復元の経路の一つは DOM の `.click()`（ページのボタンの実クリックではない）。実マウスクリックは第 6 節の変種 `mouse-3s`。

## PM 記入欄（ci1 は書かない）
'''
open(E + '/docs/0277-e8-verification.md', 'w').write(doc) if os.path.isdir(E + '/docs') else (os.makedirs(E + '/docs'), open(E + '/docs/0277-e8-verification.md', 'w').write(doc))
print(len(doc), 'chars; title variants', len(title_rows))
