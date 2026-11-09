# 0291 最小化 → 復元で playlist の題が『Pattern 1』から『(none)』に変わる件(alpha1 = alpha16)

決定 0291(仮)。基準 HEAD = main 38efb398(E9)。日付 2026-11-06。出力 `lift/out-git/alpha16-results/`。ci1 の観測(E8 本番 bundle のブラウザ、5 通りの復元で再現、復元後の題の画像は既知の『(none)』の画像と 0 pixel 差、起動直後とは 408 pixel 差)の出どころを native N1 で追った。

**実エンジンの実行について**: 測定はすべて native N1(`lift/out-alpha3` の N1 = 生成 C を clang -O2 でビルドした実行ファイル。E8 側 `n1-e8d` = e8-wip の runtime、実エンジン = FLEngine_x64.dll と同梱 DLL の lift 結果)で、G3 スクリプト(`a11-restore.g3`: 起動 → ウィザードを Enter で閉じる → 主窓の最小化ボタンのクリック → タスクバーの復元)を仮想時計で走らせたもの。**入力は harness の模擬で、利用者の操作ではない**。gdb の python で runtime の関数・guest の関数・guest のメモリを読んだ(プログラムは止めない)。ブラウザの bundle(E8/E9)・実 Windows の測定は 0 件。

## 0. 結論(先頭 3 点)

【測ったこと】
1. **N1 で再現した**。主窓の面の画素で、playlist の題は最小化前(e03)『Playlist - Arrangement ▸ Pattern 1』、最小化 → 復元の後(rst2)『Playlist - Arrangement ▸ (none) ▸』(`pics/pl-title-e03-vs-rst2-x1040.png`)。
2. **題を組み立てるのは FL の guest 関数 `f_e3ebd0`(VA 0xe3ebd0)**: 『パターン名か (none) か』は FL の内部状態 `form[0xb18]`(mode = 2)と、選択の大域 `[0x16ba944]` だけで決まる(runtime の API の戻り値は読まない)。N1 では起動直後から**この大域は -1 = 『選択なし』が定常**で、定常の題は『(none)』。『Pattern 1』は**起動の最後の約 10 回の再評価の間だけ大域が一時的に 0x50015000(≠ -1)だった**ために組み立てられた**古い表示**で、その後大域が -1 に戻っても再評価が起きず(Playlist フォームに WM_SIZE 系が届かない)残る。最小化で FL 自身のレイアウトが Playlist フォームを 1049 → 1059 幅にし、復元で 1049 に戻す(`SetWindowPos`)→ Playlist の WM_SIZE → `f_e40150` → `f_e3ef60` → `f_e3ebd0` が走って初めて『(none)』になる。**最小化でも同じ再評価が起きる**(復元だけの現象ではない)。
3. **契約の逸脱は、この題の変化に関わる所では見つからなかった**。最小化・復元で主窓・Playlist に届くメッセージの順序と引数は alpha10 の Wine 10.0 との比較表と同じ形(焦点・活性化のメッセージと、最小化中の WINDOWPOS を FL が書き換えた値の扱いだけが既知の差)。題の再評価は WM_SIZE が届くことだけが契機で、その引数の細部には依らない。**直す差分案は無い → 『FL 自身の挙動』として閉じる**(実 Windows でも同じ関数が同じ契機で再評価する。起動直後の『Pattern 1』が実 Windows でも出るかは、起動中に Playlist へもう一度 WM_SIZE 系が届くかで決まり、契約から読み切れない = 未確認。§6)。

【未確認】実 Windows・ブラウザ(E8/E9)での測定 0 件。起動直後の古い表示が実 Windows でも出るか(WM_PAINT と WM_SIZE の順序の契約との照合は未了)。

---

## 1. 再現(画素)

N1 で `a11-restore.g3` を走らせ、スナップショット e03(最小化前)・min1(最小化後)・rst1(復元直後)・rst2(復元 2 秒後)で主窓(lid 14、1920×1080)の面を生バイトで保存し、playlist の題の領域(Playlist フォーム lid 53 は TWPControl lid 16 〔(372, 82)〕の中の (499, 0)、題の TNewCaption lid 91 は (4, 4) なので画面の (875, 86))を切り出した。

- `pics/pl-title-e03-vs-rst2-x1040.png`(左 = e03、中 = rst2、右 = 差): 左『Playlist - Arrangement ▸ Pattern 1』、中『Playlist - Arrangement ▸ (none) ▸』。
- `pics/pl-title-e03-vs-min1-x1040.png`: min1 の面は最小化中で内容が無効(縞)。`pics/pl-title-rst1-vs-rst2-x1040.png`: rst1 と rst2 は同じ。
- 窓表(`GetWindowText` に当たる値)の Playlist の題は 全スナップショットで『Playlist -』のまま(題の続きは窓のテキストではなく FL の描画の中身)。

## 2. 題を決める FL の関数(逆アセンブルと gdb)

FLEngine_x64.dll の VA(ImageBase 0x400000 を含む)。逆アセンブルは `lift/objdump-text.txt`。

**`f_e3ebd0`(0xe3ebd0)**: 引数 form(rcx)。
```
mode = form->[0xb18]                       ; int
if mode == 0: 何もしない
else if mode == 1: idx = form->[0xc48]; obj = PatternList[idx]; s = obj->GetName()      ; 0xfe8bb0, vtbl+0x68
else:             sel = form->[0xd2c]->[0x610]->[4]          ; = 大域 [0x16ba944]
                  if sel == -1: s = L"(none)"                 ; リテラル 0xe3ed34、0xe3ec5f で参照(f_8da350 で UnicodeString に)
                  else:         s = 名前(選択)                ; f_e50820
caption = Format(s, 区切り 0xe3ed50, 0xe3ed60, flag)             ; f_44bc50
form->[0x7a0].SetText(caption)                               ; f_60a490
その後 f_e3eb90(form), f_e3d980(form)
```
- `form` は Playlist フォーム(TEventEditForm、N1 では 0x7ffff1c0d310)。mode は起動時に `f_e34450(form, 2)` が **1 回だけ**呼ばれて 2 になる(呼び出し元は主初期化 `f_11be0a0`)。以後 mode は変わらない(gdb で `f_e34450` の呼び出しを全部見た)。
- 呼び出し元: `f_e3ef60`(レイアウト、`form[0x760]` の幅を決めて `f_e3ebd0` を呼ぶ)、`f_e40150`(Playlist フォームのサイズ変更の処理)。どちらも主窓の WndProc(`f_52a570` → `f_612940` → … )から届くサイズ変更の経路。

## 3. トリガ(いつ再評価されるか)

`tools/a16-strcall.py`(`f_8da350` に L"(none)" のリテラルが渡される呼び出し)と `tools/a16-playlist.py`(`f_e3ebd0` の呼び出し)で:
- 起動から最小化前(e03)まで: `f_e3ebd0` は 2,024 回呼ばれる。最初の約 2,000 回は mode = 2 で選択の大域 = -1(= 『(none)』の枝)、**最後の 10 回だけ大域が 0x50015000**(= -1 でない値、題は選択の名前 = 『Pattern 1』)。
- 最小化(e03 → min1)と復元(min1 → rst1)で **同じ呼び出しが各 2 回**: `f_e40150` 経由 1 回と `f_e3ef60` 経由 1 回。毎回大域は -1 で『(none)』を作る。
- 何も操作しない 40 仮想秒(`a16-idle40.g3`)では `f_e3ebd0` は呼ばれない。
- 最小化・復元で Playlist フォームに届く物(`tools/a16-msgs.py`、`call_wndproc` の gdb): 主窓の AlignControls が `SetWindowPos(Playlist, (499, 0, 1059×998))`(最小化)/ `(499, 0, 1049×998)`(復元)→ `WM_WINDOWPOSCHANGING` / `WM_NCCALCSIZE` / `WM_WINDOWPOSCHANGED` / **`WM_SIZE`(SIZE_RESTORED、1059×998 / 1049×998)** → `f_e40150`。
  (最小化中に FL の AlignControls が幅を変える理由は、0282 の『15 px』と同じ根: FL の `WM_WINDOWPOSCHANGING` の処理が WINDOWPOS を (32, -16, 558×400) に書き換え、VCL の `UpdateBounds` が iconic の枝で `SetWindowPos(1930×1092)` を呼ぶ。runtime はそれらを契約どおりに適用している。)

## 4. なぜ最初が 『Pattern 1』か(古い表示)

選択の大域 `[0x16ba944]` を起動の最初から gdb の書き込み監視で見た(`tools/a16-watch.py`、`A16_ARM=start`):

| 順 | 書き込み | 書いた物(guest の呼び出しの鎖の先頭) | その時に配送されていたメッセージ |
|---|---|---|---|
| 1 | 0 → -1 | `f_11b3730` ← `f_11b32a0` ← `fb_1375990`(起動) | — |
| 2 | -1 → **0x50015000** | `f_e9c6f0` ← `f_12ddb20` ← … ← `f_11ef180` | `WM_PAINT`(サンプル一覧 lid 68、hwnd 0x7f990110) |
| 3 | 0x50015000 → 0 | `f_3a67380` ← `f_2c9dae0` ← `f_11b3730` | 同上 |
| 4 | 0 → **-1** | `f_11b3730` ← `fb_11d58c0` ← … ← `f_52a570` ← `dispatch_message` | 同上 |

- 2 の間に `f_e3ebd0` が最後の 10 回呼ばれた(鎖は `f_e3ef60` ← `f_ea0ec0` ← **`f_e9c6f0`** ← …、つまり 2 を書いた関数の中)→ 題 = 『Pattern 1』。
- 4 で大域が -1 に戻る。この後 e03 までの約 11,000 件のイベントの中で、Playlist フォームに届くのは `WM_PAINT` 16・`WM_ERASEBKGND`・`WM_GETTEXT`(+ LEN)・独自の 0x55a だけで、**`WM_SIZE` / `WM_WINDOWPOSCHANGED` は 1 つも無い**。主窓へは `SetActiveWindow` の `WM_WINDOWPOSCHANGING/CHANGED`(NOSIZE)と `WM_PAINT` 479 のみ。→ 題は再評価されず、2 の間に作られた『Pattern 1』が残る。
- したがって『Pattern 1』は**一時値で組み立てた題の更新漏れ**で、FL の関数の形から決まる(`f_e9c6f0` が大域を一時的に立て、その中で題を再評価し、戻すときには再評価しない)。runtime の API の戻り値は題に入っていない。

## 5. 契約との照合(最小化・復元)

alpha10(決定 0267)の Wine 10.0 との比較表(`alpha10-results/tables/syscmd-wine-vs-runtime.tsv`、テスト窓)を基準に、FL の主窓で実際に届いた物を並べた(`tables/0291-fl-main-form-minimize-restore.txt`、`tools/a16-msgs.py`):

| 段 | Wine 10.0(テスト窓) | runtime(FL の主窓) | 判定 |
|---|---|---|---|
| 最小化(FL が `ShowWindow(SW_MINIMIZE)`) | [`WM_KILLFOCUS`] `WM_WINDOWPOSCHANGING` → `NCCALCSIZE` → `WINDOWPOSCHANGED` → `WM_MOVE`(-32000, -32000) → `WM_SIZE`(SIZE_MINIMIZED, 0×0) | `WM_WINDOWPOSCHANGING`(-32000, -32000, 0×0)→ **FL が WINDOWPOS を (32, -16, 558×400) に書き換え** → `NCCALCSIZE`(32, -16, 590, 384)→ `WINDOWPOSCHANGED`(32, -16, 558×400)→ `WM_MOVE`(32, -16)→ `WM_SIZE`(SIZE_MINIMIZED, 558×400) | 順序は同じ。`WM_KILLFOCUS` が無い(alpha10 で既知の『焦点・活性化』の差)。`WM_MOVE`/`WM_SIZE` の値は FL が書き換えた WINDOWPOS をそのまま使う(runtime が guest の書き換えを採る)。**実 Windows が最小化中の WINDOWPOS の書き換えを同様に採るかは契約から読み切れない(未確認)** |
| 復元(タスクバー) | `WM_SYSCOMMAND`(SC_RESTORE)→ [`GETMINMAXINFO`] → `WPCHANGING/NCCALCSIZE/WPCHANGED` → `WM_MOVE`(0, 0)→ `WM_SIZE`(SIZE_MAXIMIZED, 1920×1080)→ `WM_SETFOCUS` → `WM_ACTIVATE` | `WM_SYSCOMMAND`(0xf120)→ `GETMINMAXINFO` → `WPCHANGING`(0, 0, 1920×1080)→ `NCCALCSIZE` → `WPCHANGED` → `WM_MOVE`(0, 0)→ `WM_SIZE`(2, 1920×1080)→ `WM_NCACTIVATE`(0)/`WM_ACTIVATE`(0)〔`SetActiveWindow`〕→ … → `WM_NCACTIVATE`(1)/`WM_ACTIVATE`(1)〔`SetFocus`〕 | 順序・引数は同じ(焦点・活性化の細部は alpha10 で既知の差)。VCL の `TApplication.Restore` が `SetActiveWindow` → `SetFocus` を呼ぶ形と読める |
| Playlist(子)へ | `WM_WINDOWPOSCHANGING/CHANGED` → `WM_SIZE` | 同じ(§3) | 一致 |

- 題の再評価の契機は『Playlist に `WM_SIZE` が届くこと』だけで、その `lParam` の値(1059/1049)は `f_e3ebd0` が読まない。上の差が題に影響した形跡はない。
- 欠けている/多い物が『FL にパターン未選択と判断させる』形にはなっていない: `f_e3ebd0` が読むのは FL 内部の `form[0xb18]` と大域 `[0x16ba944]` で、どちらも最小化・復元の前後で値が同じ(e03 の時点で既に mode = 2、大域 = -1)。

## 6. 判定と、残る未確認

**判定: FL 自身の挙動**。根拠: (1) 題を作る関数が FL の内部状態だけを読む、(2) 古い表示 『Pattern 1』は起動中の一時値で作られ更新されない FL の形、(3) 再評価の契機の WM_SIZE は 契約どおりに届いている。直す差分案は無い(足すなら題の再評価を代行することになり、lift の忠実性の方針に反する)。

残る未確認(契約からは決められない物):
1. **起動直後の『Pattern 1』が実 Windows でも出るか**: 実 Windows で、サンプル一覧の WM_PAINT(§4 の 2〜4)の後に Playlist へ WM_SIZE 系がもう一度届くなら、題は起動直後に『(none)』になり得る。届き方は WM_PAINT の配送順(契約: 『メッセージキューが空のとき更新領域のある窓へ』、窓どうしの順序は未規定に近い)に依るので、実機の参照が要る。
2. 最小化中の WINDOWPOS の書き換えを実 Windows が採るか(WM_MOVE/WM_SIZE の値。0282 の『15 px』と同じ未確認)。
3. ブラウザ(E8/E9)での再現: N1 と同じ関数・同じ契機のはずだが、ブラウザの bundle では測っていない。ci1 の『5 通りの復元』で最小化も含めて題が同じになるか(最小化だけでも『(none)』になるはず)を確かめると、本件の説明の裏が取れる。

## 7. 逸脱・自己申告

- `pkill -x gdb` を一度打ち、自分の gdb のつもりが**同じユーザーの gdb を全部止めた**(12:51、0291 の作業中)。他の agent の gdb が止まった可能性。PM に報告済み(以後 pkill は使わず PID を読んで kill する)。
- 存在しないツール名 `bash` の呼び出し、`sleep` の試行(拒否)を繰り返した。
- `/tmp`(tmpfs)の枯渇で書き込みが失敗(自分の assets・バイナリを消して解消)。

## 8. 成果物(`lift/out-git/alpha16-results/`)

| 物 | 内容 |
|---|---|
| `docs/0291-playlist-title-pattern1-to-none.md` | 本書 |
| `pics/` | 題の領域の画素(e03 対 rst2 ほか) |
| `tables/0291-fl-main-form-minimize-restore.txt` | 最小化・復元で主窓と Playlist に届いたメッセージ(引数つき) |
| `logs/*.gdb.gz` | gdb の記録(`f_e3ebd0` の呼び出し、`f_8da350` の呼び出し、大域の書き込み監視、配送メッセージ ほか) |
| `scripts/` | G3 スクリプト(`a11-restore.g3`、`a16-idle40.g3`、`a16-nomin.g3`、`a16-restore-nowiz.g3`) |
| `tools/` | gdb の python(`a16-playlist`・`a16-strcall`・`a16-watch`・`a16-msgs`・`a16-memfind`・`a16-settext`・`a16-textout`・`a16-mode`)、`a16-run-gdb.sh`(`out-alpha3/tools/run-gdb.sh` に SEED を足した写し)、表の整形 `a16-msgs-table.py`・`a16-settext-table.py` |

## PM の判断と取り込み(2026-11-06 14:15、docs のみ。製品の差分なし)

取り込み: alpha1(alpha16)の最終の文書・表・画・台本・gdb の道具を `docs/investigations/alpha16/` に置いた(生の gdb ログ約 0.7 MB は入れない。原本は `lift/out-git/alpha16-results/logs/`)。測定は native N1(E8 側の runtime、実エンジン = FLEngine_x64.dll と同梱 DLL の lift)を G3 の模擬入力で走らせた物で、**ブラウザ(E8/E9)・実 Windows・依頼者の機械は 0 件**。ci1 のブラウザでの観測(5 通りの復元で再現)と N1 の再現が一致する。

判断:
1. **『FL 自身の挙動』として閉じる。直す差分は作らない**。題は FL の guest 関数 `f_e3ebd0`(mode 2 では大域 `[0x16ba944]` が -1 なら `L"(none)"`、それ以外なら選択の名前)が FL の内部状態だけで決める(runtime の API の戻り値は読まない)。起動直後の『Pattern 1』は、起動の最後の約 10 回の再評価の間だけ大域が一時的に 0x50015000 だったために作られた古い表示で、その後 Playlist に WM_SIZE 系が届かず更新されない。最小化・復元のどちらでも主窓のレイアウトが Playlist に WM_SIZE を送り、再評価されて『(none)』になる。足すなら題の再評価を代行することになり、lift 忠実性の方針に反する。
2. 契約の照合: 最小化・復元で主窓に届く順序と引数は alpha10 の Wine 10.0 との比較表と同じ形で、差は焦点・活性化(既知)と、最小化中の WINDOWPOS を FL が書き換えた値の扱い(0282 の 15 px と同じ根)だけ。題の再評価に影響しない。
3. 未確認(契約から決められない): (a) 起動直後の『Pattern 1』が実 Windows でも出るか(サンプル一覧の WM_PAINT の後に Playlist へ WM_SIZE 系がもう一度届くか。窓どうしの WM_PAINT の配送順は未規定に近い)、(b) 最小化中の WINDOWPOS の書き換えを実 Windows が採るか、(c) ブラウザで N1 と同じ関数・契機か(ci1 の `title` モードに『最小化だけで題が (none) になるか』の変種を足して確認する)。
