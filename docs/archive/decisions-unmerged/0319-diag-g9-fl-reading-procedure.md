# 0319 補遺: diag-g9-fl(cand1)の診断ログの読み方(gpu1 の run を読む準備。alpha1、08:3x)

**実際に実行した物**: `tools/a22-diag-read.py`(Python、依存なし)を (1) native の harness(`~/work/alpha22w/scroll0319/scroll_0319.c` を、HEAD ed125d09 + 0323 + scroll-model + time-env + stop-context + diag-g9-fl + diag-g9-fl-scroll を実適用した runtime C にリンクし、`FLE_DIAG_WIN=1 FLE_DIAG_SCROLL=1` で実行)が出した **本物の診断行**(window create/destroy 16 行・ScrollWindowEx 6 行)に対して、(2) 診断の printf の書式から**手で書いた fixture**(`tables/0319-diag-fixture-HANDWRITTEN-not-a-run.log`。**run の出力ではない**。DefWindowProc・vfs-miss・thread wait・stop context の行の読み取りテスト用)に対して実行した。**wasm の bundle・Chrome・実エンジンの FL では実行していない**(cand1 の bundle はまだ無い)。以下の『期待』『読み』は仮説で、gpu1 の実 run の出力で確かめる。

## 1. 有効にする(コード変更なし)

ページは engine の環境変数を `manifest.engine.env` で渡す(`app/src/main.ts` の `applyScreenParam` が FLE_MONITOR_W/H を入れるのと同じ道)。gpu1 の私的な manifest の複製の `engine.env` に次を足す(既定の manifest には入れない):

```
"FLE_DIAG_WIN": "1", "FLE_DIAG_SCROLL": "1", "FLE_DIAG_VFS_MISS": "1", "FLE_FRUITY_THREAD_TRACE": "1"
```

cand1 は `getenv` で読む(製品形の bundle でも効く。main に入れる版は `FLE_TEST_GETENV`〔決定 0243〕に戻る)。出力は engine の stderr = ページが engine のログとして集める所(gpu1 の run のログ)。上限: window 4000 行・DefWindowProc 3000 行・ScrollWindowEx 4000 行・vfs-miss 6000 行(超えたら黙る)。

## 2. 何が出るか(5 種)

| 行 | 内容 | 有効化 |
|---|---|---|
| `fl-diag: win create/destroy lid L class "C" title "T" rect (x,y,WxH) style … exstyle … parent_lid P [(owner)] caller VA thread N` | ゲストが作った/壊した窓。`parent_lid -1 (owner)` = 親も owner も無い最上位窓(表記が紛らわしいだけ)。caller = CreateWindowEx を呼んだゲストの戻り番地(destroy は 0)。VA が 0x400000〜0x18C0000 なら FLEngine 本体、その外は読み込んだ module(plug-in の DLL)の可能性 | FLE_DIAG_WIN |
| `fl-diag: DefWindowProc WM_ERASEBKGND\|WM_PAINT lid L rect … visible V chain C(lid) < …` | その窓の既定窓手続きが処理した WM_ERASEBKGND / WM_PAINT と、親の連鎖(4 段まで) | FLE_DIAG_WIN |
| `fl-diag: ScrollWindowEx\|ScrollWindow hwnd … dx … dy … prcScroll … prcClip … flags … -> 戻り値 invalid_handle … parts … uncovered (l,t,r,b) update (l,t,r,b) caller VA` | scroll の引数と戻り値。旧 path の行は `rows moved N client WxH` の形(model が答えられない時の fallback) | FLE_DIAG_SCROLL(要 diag-g9-fl-scroll.diff。無いと model の経路は出ない) |
| `fl-diag: vfs-miss API status S path P` | ゲストが探して仮想ボリュームに無かった path(同じ path は 1 回)。status 2 = 最後の成分が無い、3 = 途中のディレクトリが無い | FLE_DIAG_VFS_MISS |
| `fle-thread: wait API kind K thread T timeout MS caller VA` | thread が待ちに入った API と時間 | FLE_FRUITY_THREAD_TRACE |
| `fle: stop context: guest return … rsp … rcx … rdx … r8 … r9` | 名前つき停止の直後に 1 行(stop-context.diff。常に出る) | — |

## 3. 読む(`python3 tools/a22-diag-read.py LOG`。ログは接頭辞つきでよい)

見出しは固定: 0 入力 / 1 窓の系統 / 2 ScrollWindow(Ex) / 3 vfs-miss / 4 thread の待ち / 5 stop の文脈。

### 3.1 Balance の窓の系統を取り出す(O13: 名前をクリックしても枠だけで中身が描かれない)
前提(実測、0310 §13、node G3、診断なし): FL 自身の枠 `TPluginForm`(237×179)+ `TNewCaption` + `TVectorSheet` + `TWPControlForm`(227×148)は作られ、**`TWPControlForm` に子窓が 0**。クリック中に窓が 17 個作られ 8 個壊れた。
1. gpu1 の run で Balance の名前を 1 回クリックし、ログを `a22-diag-read.py` に渡す。既定の `--family` は `TPluginForm|TWPControlForm|TVectorSheet|TNewCaption|TFormBalance|TFruity`。最初の系統の窓の lid 以降(lid は増える一方)を木で出す。別の範囲は `--from-lid/--to-lid`。
2. 見る物と読み:
   - **`[check] TWPControlForm … child windows`** の数: 0 のままなら、Balance の form の窓は作られていない(または作った直後に壊された)。作られて壊されたなら 3.の born-and-died の行(作成と破棄の呼び元と順序)が原因の入り口。
   - **born-and-died の窓**(作ってすぐ壊した 8 個の候補): class・rect・caller。同じ caller が作り続けて壊すなら、その caller の側(ゲストの呼び出し)が途中で失敗している疑い。caller の VA を `fle_stop` の文脈(§5)や lift の関数表と突き合わせる。
   - **DefWindowProc の欄**(`Def: erase/paint` の回数): 自前の描画を持つはずの窓(Delphi の custom control は自分の WndProc で描く)が既定窓手続きに落ちているなら、その窓の WM_PAINT/WM_ERASEBKGND がゲストの WndProc に届いていない(メッセージの経路の疑い)。class の背景ブラシで消すはずの窓なら、ci1 の 0323(DefWindowProc の背景消去)が効いているか確認。
   - **vfs-miss の同時刻の行**(§3.3): Balance の form が探した資材(skin・フォント・設定)の欠け。
3. 合否: 『子窓が作られ、DefWindowProc に落ちる窓が無く、vfs-miss が無い』なら Balance の form は描かれる見込み。どれかが外れたら、その行が次の調査対象(症状ごとの対症はしない: (a) 余計な処理か (b) 契約の不足かを、行から決める)。

### 3.2 scroll(O9)
`flags 0`(SW_INVALIDATE/SW_ERASE なし)の呼びが多く、その後ゲストが InvalidateRect しないなら、画面が直らない経路。`-> 戻り値` と `uncovered`/`update` の矩形が窓の大きさと合うか。`invalid_handle 1` は死んだ窓への呼び。ゲストが ScrollWindowEx の後に呼ぶ InvalidateRgn は、model の hrgnUpdate(矩形)を使う。

### 3.3 vfs-miss
`by kind` の `fonts` が日本語フォント探索(O16)の手がかり: ゲストが `C:\Windows\Fonts\…` の何を探して無かったか(FL の FreeType module が使うフォント名・ファイル名)。`data`/`plugins` は FL が探して無かった資材(O3 のロゴ画像など)。status 3(途中のディレクトリが無い)は『その階層ごと無い』= 遅延 VFS に足す資材の単位の手がかり。

### 3.4 thread の待ち
Fruity thread の待ちの API・kind・caller の組ごとの回数と timeout 合計。長い待ちが多い組は、協調 1 Worker での詰まりの候補(P3)。

### 3.5 layered 窓(O1/O3)
`--family TLightweightLayeredControl` で、その窓の作成(rect が 4×3 か 3×3 か)・親・呼び元・後の破棄を見る。サイズが作成時に 4×3 で後から変わらないなら、サイズを決めるゲストの処理(画像の読み込み後に決める)が走っていない = 画像の資材(vfs-miss の `data`)の欠けを疑う。

## 4. 限界
- caller は『ゲストの戻り番地』で、どの module か(FLEngine か plug-in の DLL か)は VA の範囲だけで分かれる(`engine` = 0x400000〜0x18C0000、それ以外は `other`)。module 表との突き合わせは lift の成果物が要る。
- 上限を超えた行は出ない(行数を `== 0.` で確認)。
- 診断は観察だけで、ゲストが見る物を変えない(TEST 専用の方針)。
