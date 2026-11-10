# 0331 RT-B の bundle を実 Chrome で回す計画(scenario・読み方・assets の script)— 乾燥の確認まで(gpu1/gpu8、2026-11-07 11:xx JST)

**実際に実行した物**: **Chrome は起動していない**。実 RT-B bundle も無い(mt3 の冷間 build は 11:18:42 に始まり、完了見込み 12:25)。実行したのは(1)`tools/mk-rtb-scenarios.py` で 5 本の scenario を既存の scenario(0313 の g9a、0319 の c1-balance・c1-scroll・c1-o1・c1-o1b)から生成、(2)新しい `tools/scn-lint.py`(静的検査。driver の `sdriveg2.mjs` が実装する op と、各 op が読む field を driver の source から読み出して照合し、`evalPage` の JavaScript は node で構文検査)を 5 本に実行 = **error 0・warning 0**、壊した scenario に実行して 8 errors を検出(負の対照)、(3)`tools/rtb-read.py`(読み取りの script)を **cand1 の実 run のログ**(`0319/logs/cand1-{v3,balance,scroll,drawer5a2,o1}`)に実行して、表が出ることと値が既知の事実に合うことを確認(v3: Balance・Filter PASS、Delay 2 absent、Problems 6。scroll: sha が cand1 と同一、ScrollWindowEx 66 回、scrolled hwnd 0x7f990058 = lid 22 = `TMIDIForm`、CLIPCHILDREN YES。drawer: STOPPED を検出。o1: ULW 0、layered 1)、(4)`tools/mk-assets-rtb.sh` を cand1 の bundle で試して(manifest のコピーに env が入り、bundle の manifest は無変更)消した、(5)`tools/serve-rtpage.sh`(vite preview の page-rt1)を実際に起動して manifest を取得できることを確認して止めた、(6)`tools/run-rtb.sh` を `DRY=1` で 5 種とも回した(lint・server の応答・メモリの検査を通り、**Chrome は起動しない**)。RT-B の実 bundle での実測は 0。

## 0. bundle ができたら(1 コマンド × 3)
```
cd ~/work/signature-studio-develop/lift/out-gpu2
bash tools/mk-assets-rtb.sh <RT-B の bundle の dir> rtb          # assets-rtb を作る(bundle の link + manifest のコピーに診断の env + proj)
bash tools/serve-rtpage.sh rtb 4995                             # background:true(page-rt1、COOP/COEP 付き)
bash tools/run-rtb.sh v3 4995                                   # 続けて balance、scroll、drawer、o1(PM の順。Chrome は 1 つずつ、PM の合図の後)
```
- `mk-assets-rtb.sh` は bundle を書き換えない。manifest のコピーの `engine.env` に `FLE_DIAG_WIN=1`(窓の create/destroy・vis・show・paint-msg・BeginPaint/EndPaint・draw-discard・DefWindowProc・ULW)、`FLE_DIAG_SCROLL=1`、`FLE_DIAG_VFS_MISS=1`、`FLE_DIAG_CB=1` を足す(alpha1 の診断は `FLE_DIAG_WIN` に paint と ULW が乗る。PM の言う PAINT/ULW は別の env ではない)。追加は `EXTRA_ENV="K=V,K=V"`。manifest の URL が bundle の directory 名で始まることも検査する。SHA256SUMS に合わない file は警告(テスト版の manifest は env を足すので manifest.json だけ違う想定)。日本語名の FLP が要る時は `JA=1`。
- `run-rtb.sh KIND PORT [TAG]`: lint → server の応答 → **MemAvailable ≥ 8 GB かつ swap の入出力が 0**(満たさなければ起動しない。`FORCE=1` で上書き、その時は報告に書く。この機械では他の担当の負荷で 4〜16 KB/s の標本が出ることがあり、DRY で 5 種中 3 種が『swap 0 でない』で止まった)→ `pre-run-note.sh` → `run-rt.sh` → `rtb-read.py` → 小さい結果を `gpu8-results/0331/logs/<TAG>/` にコピー。`DRY=1` なら検査だけ。
- 所要の目安(cand1 の run から): v3 約 3.5 分、balance 約 4 分、scroll 約 40 秒、drawer 約 1.5 分、o1 約 6 分。合計 約 16 分。
- 前提: テスト版の bundle(診断の env を読むのはテスト版だけ。本番形は無視する)。page は **page-rt1**(RT 用。p0 の page では pthread の worker が作れず『Tried to spawn a new thread … Invalid URL』で落ちる = seed2b で実測)。

## 1. scenario と読み方
| # | scenario(`tools/`) | 何をする | 読み(`rtb-read.py KIND RUNDIR`) | 合格の見方 |
|---|---|---|---|---|
| 1 | `scn-rtb-v3.json` | wizard を閉じる → `serina1-serum2.flp` を開く → Problems 窓の絵・describe・engine log 全部 | `v3`: Balance / Filter / **Delay 2** の `LoadLibraryW` と `GetProcAddress(CreatePlugInstance)` の表、Problems 窓の高さから plugin 数(8 行 = 343 px、1 行 16 px)、stop 行 | Delay 2 = `LoadLibraryW "…Fruity Delay 2_x64.dll" -> 0x7800000`、`GetProcAddress(0x7800000, "CreatePlugInstance") -> 0x7abf1a0`、Balance `0x6800000`/`0x6abdde0`、Filter `0x7000000`/`0x72cb670`、`fle: unsupported` 0、Problems **5**(295 px。cand1 は 6 = 311 px) |
| 2 | `scn-rtb-balance.json` | 0313 の stage 3: project を開く → Problems を閉じる → Insert 1 を選ぶ → スロット 0 の名前を 1 回クリック → 1 / 3 / 10 s の絵・窓の一覧・log の増分 | `balance`: クリックで作られた窓の系統、最後の `TWPControlForm`(cand1 は lid 171)の子の数、**プラグイン自身のコード(FLEngine の image の外)が作った窓**、窓ごとの `vis` `show` `paint-msg` `BeginPaint` `EndPaint` `draw-discard` `DefWindowProc` の回数の表、lid 171 の最初の行、`draw-discard` の `first-hidden-lid`、付録に `a22-diag-read.py` の窓の木 | client が描かれるか: `03c-10s.png`。原因の見方(alpha1 の手順 §3.1): 子窓が 0 のままなら Balance の form の窓が作られていない/作った直後に壊された。`draw-discard … first-hidden-lid N` は『その窓が見えない窓の下にあるので描画を捨てた』。`DefWindowProc` に落ちる窓があれば自前の WM_PAINT がゲストに届いていない。cand1 の事実: 子 0、プラグイン自身の窓の create 0 行(`TWPControlForm` の create 自体は caller 0x6973abe = Balance の image の中) |
| 3 | `scn-rtb-scroll.json` | F10 → General → ホイール 6 回。前後の sha・絵、`log-open`(開く時の create style・vis・paint)と `log-scroll`(ScrollWindowEx・draw-discard) | `scroll`: canvas/peek/dump の sha を cand1 の値(top `63294acd2778e87a`、scrolled `f5cc73589fd22261`)と比較、ScrollWindowEx の hwnd → lid → **create style の `0x02000000`(WS_CLIPCHILDREN)と `0x04000000`(WS_CLIPSIBLINGS)の有無**と親の連鎖、スクロール前後の `draw-discard`(lid・class ごと) | 絵が崩れないか(`scrolled-canvas.png`、帯・同じ行の重なりが無い)。sha が cand1 と同一なら『変わっていない』。cand1 の事実: 66 回、hwnd 0x7f990058 = lid 22 = `TMIDIForm`(style 0x86000000 = WS_CLIPCHILDREN あり)、flags 0、dy の繰り返し、paint の診断は無かった |
| 4 | `scn-rtb-drawer.json`(`intendedExit:true` = 止まっても log を保存) | wizard を開いたまま、Monitor ドロワーを畳む/開く を 2 回(各 4 s)。各段の状態・窓の一覧・log の増分、`FLE_DIAG_CB` | `drawer`: 4 段の表(page の state・exit・guest・log の行数・stop の行数)、全体の stop/nesting 行、`cb depth` の最大(**FLE_DIAG_CB の入れ子の段数**)、停止があれば `a22-cb-chain.py --last-only` | **止まらなければ callback-stack-limit の修正が効いた**(cand1 は最初の畳みで exit 134、`fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16)`、stop context `guest return 0x6136d6 rsp 0x126f2ad8 rcx 0x7ff900000e30 rdx 0x7f99016c r8 0x5 r9 0`)。新しい停止の文言(`LIMIT-guest/host/count`、`guest stack is used up`)が出たら全文を保存して先頭で報告。段数の最大が 16 を超えていれば、旧い上限ならこの経路で止まっていた証拠 |
| 5 | `scn-rtb-o1.json` | project を開く → Problems を閉じる → Pattern 1 の clip をゆっくり(400 ms 保持・30 歩)掴んで 2 トラック下へ → 途中の絵と窓の一覧 → 放す → 0.5 s / 3 s の絵 → Edit メニュー(Undo の項目名)→ ドロワーの畳み/開き(主窓全体の再描画)→ Ctrl+Z → 激しいドラッグ 8 s(`dragRate`)→ 絵・窓の一覧 | `o1`: stop/ImageList の行、**ULW の呼び**(`fl-diag: ULW …` の件数・hwnd ごと・最後の 4 件)、`TLightweightLayeredControl` の create、ドラッグ中の窓の一覧(**主窓以外の窓 = 浮いた clip の候補**)、dragRate、Track 5〜7 の切り抜きの縦積み `rtb-crops/tracks-stack.png` と FL の hint 欄の縦積み `hint-stack.png` | dcx(`GetDCEx(DCX_LOCKWINDOWUPDATE)`)と imagelist-drag の効果: `pic-slow-mid.png` に浮いた clip が描かれるか。cand1 は元の位置から clip が消えるだけで、浮いた clip は描かれず、放した後も Track 5〜7 に見えない。放した後の行方は `pic-after-collapse.png`(再描画後)と Edit メニュー(`pic-edit-menu.png`)で切り分ける。**ImageList の drag 系は成功しても log に出ない**(imagelist-drag は停止の時だけ `fle_stop` の文言が出る)ので、『呼ばれたか』は絵と ULW で読む |
- 共通: 各 run の先頭に `state()`(page の state・stopped・firstIdleMs・opErrors・exitText)を出す。結果は `RUNDIR/rtb-read.txt` と `0331/logs/<TAG>/` に。

## 2. 読み取りで見えないこと(先に言っておく)
- ImageList の drag 系が呼ばれたか(上)。
- 診断は alpha1 の TEST 専用で観察だけ。上限(window 4000 行・DefWindowProc 3000 行・ScrollWindowEx 4000 行・vfs-miss 6000 行・paint 系は全体 8000 行)を超えた行は出ない(件数が上限ちょうどなら『上限に当たった』と読む)。
- `a22-diag-read.py` の窓の木は caller を『engine(0x400000〜0x18C0000)』か『other』に分けるだけ。other がどの module(Balance・Filter・Delay 2)かは VA の範囲で人が読む(Balance 0x6800000〜、Filter 0x7000000〜、Delay 2 0x7800000〜)。
- Problems 窓の plugin 数は窓の高さからの計算(8 行 = 343 px、project が sample の欠けを出さない時のみ)。絵 `problems-pic.png` で数も確認する。
- この計画は cand1 の run のログに対して試した。RT-B の実 log の書式が違う(診断の行の形が変わる)と、`rtb-read.py` の正規表現が拾わず『0 行』と出る(NOTE の行で『この build には診断が無い、または env が off』と言う)。その時は生の log を見る。

## 3. 規則の確認
- Chrome は 1 つずつ、私のものだけ。開始は PM の合図の後(ci1 の計時の窓と重ねない)。`pre-run-note.sh` で free / vmstat を記録、swap の入出力 ≠ 0 か MemAvailable < 8 GB なら始めない(`run-rtb.sh` が止める)。
- テスト版の bundle は本番でも push でもなく、計時に使わない(first idle は診断入りで参考値)。
- 診断の env を足すのは manifest の**コピー**だけ(`mk-assets-rtb.sh`)。bundle 自体は書き換えない。
- scenario の Problems 窓の Ok は『窓の下端から 28 px』(`clickIn.dyFromBottom`)で押す。plugin の数(8/7/6/5)や sample の欠けで窓の高さが変わっても当たる(cand1 の run では 6 plugins の高さ 311 px で Ok は dy 283、ja FLP の 6 + sample 4 の 423 px では dy 395 = どちらも高さ − 28)。`sdriveg2.mjs` に追加した 1 field(driver の他の動作は無変更)。

## 4. 作った物の一覧(`lift/out-git/gpu8-results/tools/` と `lift/out-gpu2/tools/` の両方)
| file | 役 |
|---|---|
| `scn-rtb-v3.json` `scn-rtb-balance.json` `scn-rtb-scroll.json` `scn-rtb-drawer.json` `scn-rtb-o1.json` | 5 本の scenario(`mk-rtb-scenarios.py` が既存の scenario から生成) |
| `scn-lint.py` | 静的検査(driver の op/field を driver の source から読む、mark の前後関係、名前の重複、file の存在、evalPage の構文、静的な時間) |
| `rtb-read.py` | 読み取り(v3 / balance / scroll / drawer / o1) |
| `mk-assets-rtb.sh` | bundle の dir から `assets-NAME` を作る(1 コマンド) |
| `serve-rtpage.sh` | page-rt1 を `assets-NAME` で vite preview |
| `run-rtb.sh` | lint → 検査 → run → 読み → 保存(`DRY=1` で検査だけ) |
| `sdriveg2.mjs` | `clickIn.dyFromBottom` の追加(と 0319 の `listWindows.name`) |

## 5. 乾燥の確認の結果(Chrome なし)
- `scn-lint.py` を 5 本に: **0 error・0 warning**(静的な時間: v3 6 s、balance 30 s、scroll 12 s、drawer 18 s、o1 42 s〔waits + dragRate。project を開く時間と window の待ちは別〕)。負の対照(未知の op、field の綴り違い、必須の欠け、存在しない mark、重複した名前、`dy` も `dyFromBottom` も無い clickIn、無い flp、壊れた JavaScript)で 8 errors + 1 warning を検出し rc 1。
- `run-rtb.sh DRY=1`: v3・balance・drawer は swap の入出力が 0 でない標本(12 / 4 / 16 KB/s)で『起動しない』と止まった(設計どおり)、scroll と o1 は検査を通った。
- `rtb-read.py` を cand1 の実 log に: v3 は Balance / Filter PASS・Delay 2 absent・Problems 6(311 px)、balance は lid 171・子 0・プラグインの image から作られた窓 1(`caller 0x6973abe`)・paint の診断が無い旨の NOTE、scroll は sha が cand1 と同一・66 回・lid 22 `TMIDIForm` CLIPCHILDREN YES、drawer は 3 行の停止(`callback nesting`・`stop context`)を拾って STOPPED、o1 は ULW 0・layered 1・dragRate 481 moves。
- 未実行: RT-B の実 bundle、実 Chrome。
