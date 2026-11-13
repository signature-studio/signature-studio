#!/usr/bin/env python3
# mkdoc0277.py: docs/0277-e7b-verification.md (for the PM's ingest) from RESULTS-final.md (tables are copied, not retyped) and the prose below
import os, re, sys
REPO = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
D = REPO + '/lift/out-git/ci1-results/0277'
R = open(D + '/RESULTS-final.md').read()

def section(title, level='##'):
    m = re.search(r'^' + re.escape(level) + ' ' + re.escape(title) + r'.*?$\n(.*?)(?=^#{2,3} |\Z)', R, re.S | re.M)
    return m.group(1).strip('\n') if m else '(missing: ' + title + ')'

scenes = section('Scenes (new bundle)')
idle = section('First idle')
export = section('Export')
playsum = section('Playback summary', '###')
mixer = section('Mixer header drag')
alpha = section('alpha7')

doc = f'''# ci1 → PM 決定 0277（最終）: E7b の本番 bundle `fl-engine-ci-d9c160aa`（Worker 51f102d8、ページ = HEAD 0b228624）の実ブラウザ確認 — 検証キット、停止なし、Export 一致、first idle、File ×100、再生中の underrun の窓の定義

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77` の Xvfb、beacon の `webglRenderer` = llvmpipe、`desk.mode` = gl）。各値に load と I/O 圧 avg10 を併記）
1. **測ったこと**:
   - 配信バケット `fl-web-assets` の bundle を**読み取りで取得**し（`wrangler r2 object get`、局所の再ビルドなし）、manifest の全ファイルの size と sha256 を照合（wasm `d6e3b018accb…` 142,587,221 B、glue `8dc2abf01efc…`、seed `f7726113ae34…`、dll `2f494419c1ad…` の全て一致。比較の相手 `fl-engine-ci-ff5ee429` も一致）。同じページ（CI 42137885173 の headSha 0b228624 の app）で新旧を配信して比べた。
   - 11 シーン + more plugins + Add メニューの fade + F8 picker の 2 種（Chrome・Firefox の各 15）、File メニュー ×100、alpha7 の主窓の位置、first idle（単独 run・冷間・旧新交互）、Export（Ctrl+R）、再生 60 s、Mixer ヘッダードラッグの Worker CPU/移動。
2. **結論**: 決定 0277 の判定基準 4 つを**満たした**。
   - 停止なし: 15 シーン × 2 ブラウザ、再生 60 s × 2 ブラウザ、Export で停止 0（d7 は意図した終了）。
   - Export の WAV が参照と一致: 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`。
   - first idle が ff5ee429 に対し +500 ms 以内（新が速い）: Chrome の静かな run（load ≤12・I/O <1、各 6 run）で旧 5,996 → 新 5,403 ms（−593）、Firefox（各 6 run）で旧 5,276 → 新 4,900 ms（−376）。
   - File メニュー ×100: Chrome 100/100、Firefox 100/100。
   - alpha7: 主窓は @0,0 1920x1080、スクリーンショットで UI は崩れていない（ff5ee429 と同じ見え方）。
   - **決着していない物**: 再生中の underrun は Chrome の旧新 3 組すべてで新が多い向き（新 946 / 444 / 852、旧 116 / 14 / 486 quanta）だが、範囲が重なり n=3、結論にしない（第 5 節）。Mixer ドラッグは Chrome で差なし、Firefox は判定できない。
3. **未確認**: オーナーの機械（0 件）。Firefox の旧 ff5ee429 の再生中 underrun（未実施）。再生中の episode ごとの最長（beacon に無い）。mixer スレッド側の CPU（Worker busy は主 Worker のみ）。warm 起動。load ≤8・I/O 0 の静かな窓での再生比較（この文書の数値は load 4〜15 の run）。

## 1 判定基準（新 bundle、run の load と I/O 圧は各表）
{section('Pass criteria')}

## 2 11 シーン + 追加（新 bundle、Chrome・Firefox、各行の load と I/O 圧）
{scenes}

## 3 alpha7（Chrome、1920x1080 のゲスト）
{alpha}
- スクリーンショット: `shots/alpha7-d9c160aa-main-1920x1080.png`（ツールバーが上端から始まり、ブラウザ枠・チャンネルラック・ミキサー（Master と Insert 1〜9）・プレイリスト（Track 1〜19）・右上の最小化/最大化/閉じるが全て見える）。

## 4 first idle（単独 run・冷間・毎回新しいプロファイル・旧新交互。run 番号の `q` は Chrome の静かな窓での取り直し（11:27、load 5.5・I/O 0））
{idle}

## 5 Export と再生中の underrun

### 5.1 Export（Chrome、参照プロジェクトを開き Ctrl+R）
{export}

### 5.2 再生 60 s と underrun の量の定義
**数えた量**: ページの `window.__app.audio.detail()` の `underrunQuanta`（AudioWorklet の 128 フレーム ≈ 2.9 ms の描画単位で、出力リングが空だった数。音のデバイスが鳴り始めてからの**累計**。`app/src/audio/underrun.ts`、`ring-layout.ts`）と `episodes`（連続した underrun の塊の数）。opt1 の audio-b.json の `detail.underrunQuanta` と**同じカウンタ**。

**窓の定義の違い**:
- opt1 の値（ff5ee429 の 60 s 再生で 1,603 / 1,565 quanta など）は**累計**で、プロジェクトを開く間のエンジンの停止（私の run では再生前に 1,375〜2,118 quanta・1〜2 episodes が既に付く）を含む。
- 私の『再生中』は、`playing-a`（Space の 4 s 後 + 10 s の再生、再生前の読みの約 19 s 後）の終わりの読みと、停止後の読みの**差**（約 52 s の再生）で、開く間の分を含まない。
- 累計で比べると私の値も旧 1,809 / 2,140 / 1,942、新 2,342 / 1,908 / 2,407 で opt1 の 1,600 台と同じ桁、**旧新は区別できない**。再生前の停止は旧の方が長い run が多い（旧 1,684 / 2,118 / 1,456、新 1,375 / 1,432 / 1,522）ため、累計では差が小さくなる。桁の違いは窓の違い。

{playsum}
- 時間順は 新 r1（10:54）→ 旧 r1（11:13）→ 新 r2（11:16）→ 旧 r2（11:18）→ 新 r3（11:21）→ 旧 r3（11:23）。I/O 圧は全 run 0、load は 4〜15 で旧新が揃っていない（参考値）。
- Chrome の 3 組すべてで新が多い（946 vs 116、444 vs 14、852 vs 486）が、旧 r3（486）は新 r2（444）より多く、旧の run 間で 14〜486 と 35 倍動く。n=3 ずつで**結論にしない**。
- 『エンジンが自分で書いた gap フレーム』（`engineGapFrames` の差）は、エンジンが publish に間に合わず無音で埋めた量（decision 0211）。underrun の多い run で多い（新 113k / 52k / 102k、旧 12k / 1.4k / 57k）= 遅れはページ側のリングではなく**エンジン側**で起きている（読み）。
- 再生の Worker busy（CPU ÷ wall）は新が低い（新 679〜885 ms/s、旧 745〜908 ms/s）のに underrun は新が多い。underrun は再生の開始から約 10〜40 s 目（audio 時刻 63〜93 s）に集中する（Chrome r1: 10 s ごとに +0 / +228 / +389 / +308 / +21 quanta、Firefox: +0 / +80 / +267 / +338 / +0）。
- Firefox の再生は新 1 回だけ（load 3〜6）: Worker busy 706 / 866 / 991 ms/s（最小 / 中央 / 最大）、再生中 685 quanta・69 episodes。旧は未実施。
- **仮説（検証していない。私の読み）**: (H1) Worker busy は主 Worker の CPU だけで、音を作る別スレッド（ゲストの mixer スレッド）の遅れを含まない。(H2) 新は Worker が空く（IdleDriver の lead 待ちが増える）ため publish の周期が揃わず、重い区間で 1 周期遅れると hole になる。どちらも run の順序・時刻・ブラウザの背景負荷などの他の読みを排除していない。

## 6 Mixer ヘッダードラッグの Worker CPU / 移動（1920x1080、旧新交互）
{mixer}
- Chrome は差なし（−1 %）。Firefox は run ごとのばらつきが中央値の −42 〜 +35 % で、旧新の差より大きく**判定できない**。E7b の mixer CPU の効果（locshare・hyb +18）はこの測定では見えない。

## 7 自己申告の逸脱
1. **d1・d10・d11 の最初の run は無効だった（私の誤り）**。検証キットの `proj`（配信する `/assets/proj/`）に `Sampler.flp` と `SHP_Sawing Synth.wav` が無く、404 の本文（35 バイト）が『Sampler.flp』として開かれた。d11 は Chrome で新旧 bundle とも、Firefox で新 bundle が、`CompareStringW` の名前つき停止（exit 134）になり、d1・d10 は ok と出たが Sampler を操作していない無効 run だった。本物のファイル（git 管理の `docs/investigations/crash1/tools/Sampler-project/` と同一）を配信して再実行し、全て ok（d11 は 207 s → 27 s）。無効 run は表に数えていない。再発防止: `verify-bundle.sh proj` と、preflight の『4 ファイルが 200 とサイズで返る』検査（違えば rc 3 で停止）。
2. Firefox の d7（意図した終了）で WebDriver の `invalid session id` が 1 回（load 17.2、2 ジョブ並行）。同じ条件の再実行 3 回は全て ok（意図した終了）で、再現しなかった。原因は不明。
3. 局所の `.br`（品質 6）は私の作成物（転送サイズは本番と違う。内容の sha256 は manifest と一致）。
4. Firefox の Mixer 測定は 3 セッションに分けた（load 5〜29 の変動、他の作業者の job と PM の検証）。『差が無い』ではなく『判定できない』。
5. 旧新の再生比較は Chrome だけで、load が揃っていない（4〜15）。
6. `bash` ツールを今回も 1 回呼んだ（無いツール。`shell` で再実行）。`sleep` / `&` / `pkill -f` は使っていない。

## 8 検証キット（`ci1-results/0277/kit/`、README に順番と判定基準）
`verify-bundle.sh` のモード: fetch / proj / page / preflight / scenes / alpha7 / idle / export / play / ab / report / clean。`NEW`・`REF`（ページの git ref）・`WORKER` を環境変数で渡し、`PREV` は既定 ff5ee429。preflight は Firefox の beacon が `webglRenderer` 非 null かつ `desk.mode` = gl でなければ rc 2 で止まる。結果は `out/RESULTS.md`（1 つの表、各値に load と I/O 圧）。ブラウザ試験は CI に入れない。

## PM 記入欄（ci1 は書かない）
'''
open(D + '/docs/0277-e7b-verification.md', 'w').write(doc)
print(len(doc), 'chars')
