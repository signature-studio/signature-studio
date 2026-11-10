# 0272(仮): D(alpha10b)の起動直後の配置の根拠、最小化のゴミ幅の本当の原因、ページ側の復元ボタン(alpha11)

**依頼者の機械・実機 Windows・実 GPU・ブラウザの本番(bundle `fl-engine-ci-ff5ee429`、Worker `a3fc8c43-a3a9-48a2-b9cf-86b1dbf5cfca`)での測定は 0 件。** 以下は、b42fad36 の生成 C から作った native N1(HEAD の runtime = b00932ec と同一 / + alpha10.diff / + alpha10.diff + alpha10b.diff〔`f_425500` の thunk は表の行が無い生成 C なのでリンク時の `--wrap` で `fle_imp_SetWindowPlacement` に繋いだ〕)に、G3 の模擬入力(**ユーザーの操作ではない**)と gdb を使って測った値、Chrome 156 の e2e(エンジン無し)、vitest。性能の測定ではない(構造の測定だけ: 窓表、API の呼び出し列、画素の違いの有無)。測定中の load average は 6〜10(他の担当と共有)。native N1 の行は本番 bundle の行ではない。Wine・私の逆アセンブルの読みは UNOFFICIAL。代役・再生・推測は含まない。

状態: **仮の決定**。作業者 `alpha1`、2026-11-06 JST 08:15〜。成果: (a) 根拠の表と原因の確定(docs / tables)、(b) `alpha11-app.diff`(`app/` のみ。HEAD d6926b4d に `git apply --check` と `patch -p1 --dry-run` が通る)、(c) 実 FLEngine の生成 C の通し(lifter 水準)の任意のテスト `optional/`。

## 結論(先頭の数行)

1. **D の起動直後の配置が変わる 27 窓は、たった 1 つの値から来る: 主フォームの `showCmd == SW_SHOWMAXIMIZED(3)` を、FL の `WM_WINDOWPOSCHANGING` ハンドラ(`f_8076c0`、`GetWindowPlacement` の呼び出しは 0x8077af、戻り番地 0x8077b4)が読む。** 決め手はマスク実験(`tables/mask-experiments.tsv`): D の runtime が「最大化」を FL にだけ「通常(showCmd 1)」と答えると、113 窓の窓表が HEAD と**完全に同じ**(0 窓が違う)。答えを変えるのを**その 1 つの呼び出し元だけ**にしても同じ(0 窓)。他の 3 つの読み手(`IsMaximized` 0x808ca0、復元矩形の getter 0x8087eb、最大化中の SetBounds 0x808965)だけを変えても D と同じ(27 窓)= 起動の配置には効かない。**最小化の状態(showCmd 2)は起動の配置に効かない**(隠しても D と同じ)。ptMinPosition と flags は、FL が読むのは 0xba52a0 の書き戻し(構造体全体をそのまま `SetWindowPlacement` に返す)だけで、単独のマスク実験はしていない。
2. **27 窓はすべて主フォーム(lid 14)の子孫**(直接の子 6、その下 21)。主フォーム自身の矩形 (0, 0, 1920 x 1080) は変わらない。HEAD では FL が自分の最大化を知らされず、主フォームの子を 6 画素の枠の内側に並べていた(例: `TQuickControlBar` (6, 6, 1908 x 82))。D では枠が無くなり (0, 0, 1920 x 82)。**機構(gdb、`logs/gwp/frame-words-*.txt`)**: 主フォームのオブジェクトの +0x690〜+0x69c の 4 語は FL の独自の枠の幅(6, 6, 6, 6)で、`f_8076c0` の最大化の枝(showCmd 3)がそれを (0, 0, 0, 0) にし、枠のフラグ +0x6a9 を 1 → 0 にし、WINDOWPOS を作業領域 (0, 0, 1920 x 1080) に作り直して SWP_NOCOPYBITS(0x100)を足す。showCmd 3 を隠すと 4 語は最大化のあいだ (6, 6, 6, 6) のまま(= HEAD)。窓ごとの規則は `tables/startup-27-windows.tsv`(直接の子は x・y が 6 動き、幅・高さが 12 増える。その下の窓は親の幅・高さの増減に付いて行く、右・下に寄せた窓は親の増分だけ動く)。同じ 27 窓が起動 16 + 参照プロジェクト 10 = **26 スナップのすべて**で変わる(窓表の行数 116 は不変)。
3. **どちらが Windows に近いか**: 契約(Learn GetWindowPlacement / WINDOWPLACEMENT)では、`length` が正しければ呼び出しは成功し、最大化された窓の `showCmd` は `SW_SHOWMAXIMIZED`。HEAD の配置は「FL が自分の最大化状態を FALSE の応答で知らされた」状態で、Windows の FL には起こらない。D は FL 自身のコード(`f_8076c0` の最大化の枝)が最大化された窓に対して行う配置。→ **契約の上では D が Windows に近い**(実機での照合は 0 件)。
4. **0267 の説明の訂正(PM の指摘どおり。『FL が未初期化のスタックを使う』は一般には誤り)**: FL の `GetWindowPlacement` の呼び出しは**逆アセンブルで 16 か所**(同じ IAT スロット 0x1970da0 に **2 本の thunk** `f_424f40` と `f_424f50` があり、前者から 10 か所、後者から 6 か所。`tables/gwp-call-sites.tsv`)。**9 か所は戻り値を検査する**(`test eax,eax; je`)ので、HEAD の失敗は『最大化でも最小化でもない』と同じ扱いになるだけで、スタックは読まれない。**検査しないのは 7 か所**(0x617f10、0xba5814、0x1037673、0x10410c8、0x616cd9、0x862ab1、0x863cee)で、`rcNormalPosition` を直ちにコピーするか構造体をそのまま使う(0x863cee は `showCmd` から FL の WindowState を決める `UpdateWindowState` 風の関数で、失敗時はスタックの値が入る)。**到達したのは 0x617f10 だけ**(最小化のシーン)。**最小化のゴミ幅の原因は 0x617f10**: VCL の `TWinControl.UpdateBounds` の『窓が最小化されているとき』の枝(`GetWindowRect` の代わりに `GetWindowPlacement().rcNormalPosition` から Left/Top/Width/Height を取る)。長さ 44 を断る runtime では構造体が書かれず、スタックの残り(測った run では (-182791424, 32767, -163582064, 32767) = スタックのアドレスの上下半分)がフォームの大きさになり、`AlignControls` が全部の子をその幅で `SetWindowPos` する(`cx=156103730`、別の run では `769193010`: アドレスにより run ごとに違う)。
5. **ゴミ幅は alpha10(A+B+C)が作ったのではない**: **HEAD(alpha10 なし)の最小化でも同じゴミが 8 件出る**(`tables/minimize-garbage.tsv`)。B(最小化した窓の矩形 0 x 0)は VCL の iconic の枝では使われない。D で消える理由は、`GetWindowPlacement` が `rcNormalPosition`(復元矩形 (-5, -6, 1925, 1086))を書くから(FL 自身のその後の `SetWindowPos(TFruityLoopsMainForm, cx=1930, cy=1092)` は復元サイズ)。**`showCmd` は無関係**: 呼び出しが成功すれば、最小化を「通常」と答える変種(x-min)でも、常に「通常」と答える変種(x-both)でもゴミは 0。→ PM の条件(『D の効果の原因が読めないまま入れない』)の『読み違いの確定』: **起動の配置 = 1 つの値 `showCmd == 3`(f_8076c0)、最小化のゴミ = `rcNormalPosition`(0x617f10、検査なし)**。
6. **node の 84 / 55 / 87 行の予測**(`tables/adoption-and-node-predictions.tsv`): 動く行は `g3-snap`(clock_ms が仮想 2 ms 小さい)、`g3-progress`、`g3-surface lid 14`(大きさは 1920x1080 のまま、sha256 と nonzero が変わる)、`g3-wintable`(27 窓の矩形)。動かない行: `g3-win`、`g3-layered`、他の lid の `g3-surface`、`g3-timers`。HLE の `warp_bilin` の画素数が 1 行動く(-601)。ハッシュ・カウンタの値は native と wasm で違うので、動く行の**種類**だけが移る。
7. **(b) ページ側の復元ボタン**: `alpha11-app.diff`(`app/` 17 ファイル、+399 / -8)。`fle_g3_taskbar_iconic` と `fle_g3_taskbar_syscommand` の**両方**がエンジンにあるときだけ働く(無ければ何もしない = 現本番の bundle では不活性)。vitest 全体 **67 ファイル 1,307 passed 0 failed、skip 7 件(元からの 1 ファイル)**(新規 16 件を含む)、typecheck 3 設定、Chrome 156 の e2e 15 / 15(`node test/e2e/run.mjs --target dev --only taskbar-restore`)。
8. **(c)** 実 FLEngine(`binaries/win64/FLEngine_x64.dll`)の lifter 水準の生成 C で、`f_425500` と `f_425510`(同じ IAT スロットの 2 本の thunk)が `fle_imp_SetWindowPlacement` を呼び、未対応 thunk が 225 → 223 になること(行を足した表の生成 C): §8。CI の全面ビルド(生成 C の全体のコンパイルと実行)は**走らせていない**。

## 冒頭の三点(正直さ)

1. **何を実測したか**: native N1(HEAD / A+B+C / D / D に診断のマスクを足した変種)を G3 の模擬入力で走らせ、(i) 起動 16 スナップ(`cmp-short.g3` = node 84 行系)と参照プロジェクト 10 スナップ(55 / 87 行系)の g3 行と窓表、(ii) 最小化クリックの API 列、(iii) gdb で FL の `GetWindowPlacement` の全呼び出し(呼び出し元の戻り番地、答え、`showCmd`、`rcNormalPosition`)と最初のゴミ `SetWindowPos` のバックトレース、を取った。ページは vitest と Chrome(エンジン無し)で検査した。
2. **結論**: 上のとおり。
3. **未確認**: 実 Windows での FL の起動時配置(契約からの推論)、ブラウザ(wasm)での最小化 → 復元の通し(エンジンの export は emscripten で未コンパイル、ページの Worker の経路は単体テストとエンジン無しの e2e だけ)、node の 84 / 55 / 87 行(予測のみ)、CI の全面ビルドでの `SetWindowPlacement` 行入りの生成 C の実行、`f_8076c0` が枠の幅を 0 にする呼び出し(vtable +0x300)の中身(+0x690 の 4 語が 6 → 0 になることは gdb で確認したが、その関数自体は読んでいない)、0x617f10 以外の検査しない 6 か所(0xba5814、0x1037673、0x10410c8、0x616cd9、0x862ab1、0x863cee。どのシーンでも到達しなかったので、動かしていない)。

## 1. FL の `GetWindowPlacement` の呼び出し元(`tables/gwp-call-sites.tsv`、`logs/gwp/`)

`FLEngine_x64.dll`(ImageBase 0x400000)の `GetWindowPlacement` の thunk は 2 本(`f_424f40` と `f_424f50`、どちらも `jmp [IAT 0x1970da0]`)。`call 0x424f40` が 10 か所、`call 0x424f50` が 6 か所(第 1 報の『11 か所』は後者を 0x617f10 の 1 か所しか数えていなかった。**訂正: 16 か所**)。起動(`cmp-short.g3`)の D では 426 回(HEAD は同じ数が全部失敗)、呼び出し元は 5 か所:

| 戻り番地 | 関数 | 戻り値の検査 | 読む値 | 回数(D、起動) |
|---|---|---|---|---|
| 0x8077b4 | `f_8076c0` = `WM_WINDOWPOSCHANGING`(Message.LParam = WINDOWPOS) | あり | `showCmd` 2 → 枠のフラグ `[form+0x6a9]` を 0 に。3 → WINDOWPOS を作業領域で作り直す枝 | 264(うち 6 回が showCmd 3、すべて lid 14) |
| 0x8087eb | `f_8087c0` 復元矩形の getter | あり | 3 → `rcNormalPosition` + 枠の幅 | 33 |
| 0x808965 | `f_808930` SetBounds 風(最大化中は rcNormal を書き換える) | あり | 3 | 33 |
| 0x808ca0 | `f_808c80` `IsMaximized` | あり | `showCmd == 3` | 94(うち 1 回が showCmd 3、lid 14) |
| 0xba52a0 | 窓の表示 / 位置を決める関数の中 | あり | 構造体全体(`showCmd` を SW_HIDE に、`rcNormalPosition` を窓の矩形に書き換えて `SetWindowPlacement`) | 2(lid 14、lid 68)→ `SetWindowPlacement` 2 回 |

最小化のシーン(`a11-min.g3`)では**もう 1 か所**が到達する: **0x617f15(検査なし)** 5〜6 回。FL が読むのは `showCmd`(検査する 4 か所)と `rcNormalPosition`(検査する 1 か所 + 検査しない 4 か所)だけで、`ptMinPosition` と `flags` は読まない(読まれるのは 0xba52a0 の書き戻しのとき)。

## 2. マスク実験(`tables/mask-experiments.tsv`、`diag/diag-mask_showcmd-NOT-FOR-MERGE.diff`)

D の `GetWindowPlacement` が、環境変数で、特定の `showCmd` を 1(通常)と答える診断の変種(製品には入れない)で、起動の窓表(e03 のスナップ、113 窓)と最小化後のゴミを測った。

| 変種 | HEAD の起動と違う窓 | D と違う窓 | ゴミ |
|---|---|---|---|
| HEAD / A+B+C(呼び出しが失敗) | 0 | 27 | 8 |
| D | 27 | 0 | 0 |
| D、最大化(3)を全員に隠す | **0** | 27 | 0 |
| D、最小化(2)を全員に隠す | 27 | 0 | 0 |
| D、最大化を `f_8076c0`(0x8077b4)にだけ隠す | **0** | 27 | 0 |
| D、最大化を 0x808ca0 / 0x8087eb / 0x808965 にだけ隠す(各) | 27 | 0 | 0 |

読み: 27 窓の原因は `f_8076c0` が読む『主フォームは最大化されている』の 1 つ。ゴミは showCmd と無関係(呼び出しの成功で消える)。

## 3. 27 窓の窓ごとの根拠(`tables/startup-27-windows.tsv`)

起動 3 秒後(e03)の窓表、A+B+C と D の差。27 窓の内訳(lid: 規則):

- **主フォームの直接の子 6**: 16 `TWPControl`(ブラウザ以外の領域)・49 `TWPControl`(ブラウザの枠)・50 `TQuickControlBar`(上部ツールバー)・101 `TQuickControlBar`(下端の細い帯)・109 `TWPControl`(右端の幅 0)・110 `TWinControlProxy`。6 画素の枠が無くなる: 50 は (6, 6, 1908 x 82) → (0, 0, 1920 x 82)、101 は y 1074 → 1080(下端)、109 は x 1914 → 1920、49 は (6, 88, 362 x 986) → (0, 82, 364 x 998)、16 は (376, 88, 1538 x 986) → (372, 82, 1548 x 998)。
- **親に付いて行く窓 21**: 51・69(ツールバー、幅 +12)、53 `TEventEditForm`(Playlist、幅 +10・高さ +12)、64 `TFXForm`(Mixer、幅 +10、下に寄せた窓なので y +12)、68 `TSampleListForm`(Browser、幅 +2・高さ +12)、およびその下の 80・82・83・85・86・87・91〜94・102・104〜108。右に寄せた窓(80、83、93)は親の幅の増分だけ x が動く。
- 変わらない窓: 主フォーム自身(14)と、その他 86 窓。

窓表の差の機構は上の『機構』(+0x690 の枠の幅 6 → 0)。それが決まると VCL の `AlignControls` が主フォームの `ClientRect` を使って子を並べ直し、6 画素と、その結果の幅・高さの 12(= 6 x 2)が出る。

規則は窓表の差から機械的に出した一次の記述(親の増減と同じかどうか)。『なぜ 49 の幅だけ 362 → 364 か』のような個々の寸法の理由は、VCL の `AlignControls` の中の寸法(スプリッタの幅など)で、追っていない。

## 4. 最小化のゴミ幅(`tables/minimize-garbage.tsv`、`logs/api/`)

- FL のクリック → `ReleaseCapture` → ヒント・グラフ・テストのフォームを隠す `SetWindowPos`(ra 0x86838d)→ `ShowWindow(TLightweightLayeredControl, SW_HIDE)`(ra 0x868766)→ (`ShowWindow(主フォーム, SW_MINIMIZE)` の中で、窓手続きの `WM_SIZE` の処理として)**子窓の再配置** → `ShowWindow` が戻る(recorder は戻るときに記録するので、中で起きた `SetWindowPos` は `ShowWindow` の行より前に並ぶ)。
- ゴミの最初の呼び出しの backtrace(`logs/api/first-absurd-SetWindowPos-backtrace-ABC.txt`): `SetWindowPos ← f_825470(TControl.SetBounds)← … ← f_6108f0(AlignControls)← f_859c20 ← f_85d420 ← f_8064d0 ← f_610b80 ← f_614a30 ← f_85a440(形の変更のハンドラ)← … ← fle_imp_DefWindowProcW`。
- 起点は、AlignControls に入る前にフォームの Left / Top / Width / Height が壊れていること。壊したのは 0x617f10(`UpdateBounds` の iconic の枝)。gdb で、その呼び出しの後の構造体(書かれていない)が (-182791424, 32767, -163582064, 32767) だったことを確認(`logs/gwp/minimize-ABC.txt`)。
- HEAD(alpha10 なし)でも同じ(`a11-min-b`: ゴミ 8 件、主フォームの矩形 (-1675162576, 1064, 16396 x 412))。**0267 の文書の『A+B+C だけだと崩れる』は『崩れは HEAD からあり、`GetWindowPlacement` の失敗が原因』と読み替える**(A+B+C は戻す手段を作ったので崩れが見えるようになっただけ)。

## 5. 採用の順序と node の予測(`tables/adoption-and-node-predictions.tsv`、`tables/g3-rows-changed.tsv`)

要点: D は lifter の表に 1 行足すので生成 C が変わり、既定の生成 C の sha が変わる(PM が新しい sha を記録する)。次の cold build に載せる(PM の決定)。alpha10.diff → alpha10b.diff の順。動く行の種類は結論 6。

## 6. ページ側の復元入口(`alpha11-app.diff`)

**設計(旗・URL パラメータ・新オプションなし、窓の番号をページが決めない)**

1. `src/engine/fruity.ts`: `taskbarIconic()` = `fle_g3_taskbar_iconic()`(最小化されたボタンのある最初の窓の lid、無ければ 0)。**`fle_g3_taskbar_iconic` と `fle_g3_taskbar_syscommand` の両方が関数のときだけ**値を返し、無ければ `null`(= 現本番の bundle)。`inputApi.syscommand` も同じ条件。
2. `src/engine/worker.ts`: 既存の `deskTick`(`readAppView` を読む同じ場所)で `taskbarIconic()` を読み、`TaskbarReport`(`src/engine/taskbar-report.ts`、純粋なクラス)が**変化したときだけ** `{ t: 'taskbar', lid }` を投稿する(`null` は一度も投稿しない、最初の 0 も投稿しない)。通信は変化の数だけ。
3. `src/ui/taskbar.ts` `TaskbarRestore`: 報告された lid があるあいだだけボタンを出す(`hidden` を外す)。押すと raw 入力 `{ k: 'syscmd', kind: IN.SYSCMD, lid, cmd: SC.RESTORE (0xF120) }` を作り、**既存の道**(SharedArrayBuffer の ring → `IdleDriver` → エンジンの idle で `fle_g3_taskbar_syscommand(lid, 0xF120)`)で渡す。`display` の変更(決定 0212)と同じ型: エンジンが忙しければ待ち行列、最初の idle の前なら捨てる。押した後 `#desktop` にフォーカスを戻す。エンジンが終わったら(`exit`)ボタンを消す。
4. `index.html` / `style.css`: `<div id="taskbar" role="status"><button id="taskbar-restore" type="button" hidden lang="ja">最小化された窓を元に戻す</button></div>`。`#desktop-box` の後ろ(Tab で desktop から出ると最初に着く)。`position: absolute; left: 12px; bottom: 12px` の重ね置きで、**場所を取らない**(`#stage` の寸法が変わらない = 部屋が変わらない = エンジンに表示サイズの変更を送らない: e2e で確認)。
5. 入力の型: `RawInput` に `syscmd`(`input/events.ts`、`input/ring.ts` の `T_SYSCMD`、`describeRaw`)。`IdleDriver.apply` に 1 分岐。

**診断した 3 点**

- **二重に出ない**: 同じ lid が何度報告されても DOM にボタンは 1 つ(e2e で 3 回報告 → `querySelectorAll` が 1)。別の窓が報告されれば同じボタンがその窓用になる。
- **キーボードで押せる(a11y)**: `<button type="button">`。Tab で desktop から着き(e2e)、Enter・Space が押下になり(e2e)、押した後フォーカスは desktop に戻る(e2e)。非表示のボタンは Tab 順に入らない(e2e)。出現は `role="status"` の領域で通知される(`hidden` の解除)。
- **最小化したまま閉じられない**: 解釈 2 つを確認した。(i) ページの『Stop engine / End session』は復元ボタンと無関係に効く(この変更は触らない)。エンジンが終わったら `exit` のハンドラがボタンを消す(`TaskbarRestore.clear()`)。(ii) ゲストの窓を最小化したまま閉じる入口(`SC_CLOSE` 0xF060)はホスト API にあるが、**ページには配線していない**(依頼は復元だけ)。必要なら同じ型の raw 入力でそのまま足せる。

**テスト**

- vitest: 新規 `test/unit/taskbar.test.ts` 14 件(`TaskbarRestore` 5、`TaskbarReport` 3、`IdleDriver` の syscmd 4、マークアップ 2)、`input-ring.test.ts` +1、`fruity-engine.test.ts` +1(計 16 件)。全体 67 ファイル 1,307 passed、7 skip(元からの 1 ファイル)、0 failed(`lift/` を足した d6926b4d の複製で。`lift/` が無いと 5 ファイルが落ちる: それらは lift/runtime の C ヘッダを読むテストで、変更と無関係)。`typecheck` の 3 設定は通過(`tsconfig.json`、`tsconfig.webworker.json`、`tsconfig.test.json`)。`test:worker`・`test:e2e` の全体は走らせていない(app の連鎖は PM)。
- e2e(Chrome 156、エンジン無し): 新規 `test/e2e/taskbar.mjs`(`run.mjs` に group `taskbar-restore` を追加)、`node test/e2e/run.mjs --target dev --only taskbar-restore` で 15 / 15 通過。Worker のメッセージは `window.__app.taskbar.update(N)`(EngineClient が `{t:'taskbar'}` を受けて呼ぶ関数)で擬似した。**Worker が実際の export を読んで投稿する経路は e2e では通していない**(`FruityEngine.taskbarIconic` と `TaskbarReport` を単体テストで固定)。

## 7. 直していないもの / 逸脱 / 限界

- 『最小化されたゲスト窓の SC_CLOSE』・『最大化 / 最小化のボタン』はページに無い(復元だけ)。
- 複数の窓が最小化されているとき、ボタンは**最初の 1 つ**だけ(`fle_g3_taskbar_iconic` が最初の lid を返す仕様)。復元すると次の窓のボタンが出る。
- `f_8076c0` の最大化の枝が 6 画素の枠を消す仕組み(§未確認)。
- 起動直後の配置を実 Windows と照合していない。契約からの推論。
- 逸脱: なし。PM の木には書いていない。コミット・push なし。

## 8. (c) 実 FLEngine の生成 C(lifter 水準)

任意のテスト `optional/alpha11-flengine-thunk.diff`(`lift/crates/fe-lift/tests/alpha11_flengine_thunk.rs` 新規): 実 `FLEngine_x64.dll` を lifter に通し(本物の 59,000 関数の生成。C を書く先は関数ごとのハッシュと 2 関数のテキストだけ)、`f_425500` と `f_425510` の本文が `{ void fle_imp_SetWindowPlacement(void); fle_imp_SetWindowPlacement(); return; }` で `fle_unsupported` を含まないこと、`handled["jmp_imp:SetWindowPlacement"] == 2`、`unsupported_total == 223` を固定する。結果(実 DLL、`FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll`、1 passed、8 s): `f_425500` と、**同じ IAT スロット 0x1970ed8 を使うもう 1 本の thunk `f_425510`** の両方が `fle_imp_SetWindowPlacement` を呼ぶ(`fle_unsupported` なし)、`handled["jmp_imp:SetWindowPlacement"] == 2`、`unsupported_total == 223`(行の前は 225)。**これにより alpha10b.diff の `vcall_thunk_dll.rs` の期待 224 は誤りで 223 が正しい**(第 1 報の訂正。MERGE-NOTES)。native N1 の `--wrap=f_425500` は 0x425500 だけを繋ぐ。FL が呼ぶのは 0x425500(ra 0xba5328)で、0x425510 は呼ばれていない(gdb の SWP 2 回はどちらも ra 0xba5328)

CI の全面ビルド(生成 C 全体の clang 〔native〕または emscripten 〔wasm〕でのコンパイル、実行)は走らせていない。

## 9. 再現

- N1 のビルド: `GENOBJ=$PWD/gen3 RMC=$PWD/keep/rebuild/realmod_table_gen.c NO_COPY=1 JOBS=2 UPSRC=/nonexistent TAG=<t> bash tools/build-n1b.sh <bin> <tree>`(D は `build-n1c.sh` = `--wrap=f_425500`)。木は `git archive HEAD lift/runtime lift/harness` に alpha10.diff(+ alpha10b.diff)。
- 実行: `SRC=<tree> bash tools/run-n1.sh <bin> scripts/a11-cmp.g3 <out>`(`a11-ref.g3` は `SEED=lift/out-ref3/tables/vfs-seed-nested.tsv`)、gdb は `tools/run-gdb.sh <bin> <g3> tools/a11-gwp.gdb <out>`(`GWP_OUT=`)、`tools/a11-garb.gdb`(`GARB_OUT=`)。
- 表: `tools/a11-wintable.py BASE DNEW N`、`tools/a11-27.py`、`tools/a11-g3rows.py`。マスク: `diag/diag-mask_showcmd-NOT-FOR-MERGE.diff` を D の `fle_win.c` に当て、`A11_MASK_MAX=1` / `A11_MASK_MIN=1` / `A11_MASK_RA=0x8077b4`。
- ページ: `cd app && node node_modules/vitest/vitest.mjs run`、`node node_modules/typescript/bin/tsc --noEmit -p tsconfig.json`(`.webworker.json`、`.test.json`)、`APP_WORK=/tmp/x node test/e2e/run.mjs --target dev --only taskbar-restore`。

## 10. cargo(私のワークツリー `~/work/a7-cargo` = HEAD の複製 + alpha10 + alpha10b-v2。PM の木ではない)— SKIPPED を併記

- 0267 の報告の cargo は、ワークツリーに実 `FLEngine_x64.dll` が無く、実 DLL テストが `SKIPPED` で早期 return して pass と数えられていた。今回、`binaries/` と `tools/` を PM の木への symlink にし、`out-blend2d` と `out-qfc` はコピーして、`--nocapture` 付きで再走した(SKIPPED の通知が見える)。
- 実 DLL を読む 35 ファイル: **780 passed、0 failed、0 ignored**。SKIPPED の通知 1(`out-qfc` なし → 上の再走で解消)。
- 残り 63 ファイル(0267 の 48 ファイルの通し、表を読む 36 ファイル、`alpha11_flengine_thunk`、`vcall_thunk_dll` ほか): **782 passed、0 failed、0 ignored**。SKIPPED の通知 3(`out-blend2d`、`out-qfc` は解消、残りは `wall_fr9t` の任意の比較 `FLE_FR9T_CLEAN_RT` 未設定)。
- `deferred` と `wall_fr7` の再走(データのコピー後): 33 passed、0 failed、SKIPPED 0。
- 走らせたのは fe-lift のテスト 158 ファイル中 98(重複なし)。残りは走らせていない。全体テスト(`test.sh full`)、a4、node の 84 / 55 / 87 行も走らせていない。
- 訂正: `vcall_thunk_dll` の期待は 223(224 は誤り)。`alpha10b-v2.diff` / `alpha10b-fix.diff`。

## PM の判断と取り込み(2026-11-06 09:45、docs のみ)

取り込み: alpha1 の最終(09:50 付け)の文書・表・道具を `docs/investigations/alpha11/`(差分は `patches/`、`alpha11-app.diff` = ページ側の復元ボタン〔app のみ、17 ファイル、`d6926b4d` に dry-run 清潔〕)に置いた。生ログ(約 1.5 MB)は `lift/out-git/alpha11-results/logs` に残し、リポジトリには入れない。測定は alpha1 の native N1 と gdb、Chrome 156 の e2e(エンジンなし)、vitest、cargo(実 DLL 入りの 98 ファイルで 1,562 passed / 0 failed、SKIPPED は wall_fr9t の任意の比較 1 件のみ)で、実 Windows・本番 bundle のブラウザ・node 84/55/87 行は 0 件。
判断:
1. **D の起動時の配置の変化(27 窓)は、FL 自身の WM_WINDOWPOSCHANGING 処理(f_8076c0)が GetWindowPlacement の showCmd == SW_SHOWMAXIMIZED を読んで枠の幅を 0 にするためで、runtime は配置を決めていない**(lift-fidelity の (b) 外部 API の契約の実装)。Learn の契約に忠実な戻り値のほうが Windows に近い(実機は 0 件)。
2. **`alpha11-app.diff` は app だけの変更で、エンジンの export(`fle_g3_taskbar_iconic` と `fle_g3_taskbar_syscommand`)が両方あるときだけ動く**(無ければ不活性)ので、現本番の bundle には影響しない。PM が app の連鎖(typecheck / vitest / test:worker / test:e2e)を回してから、CI が空いたときに app だけで push する。復元ボタンが実際に出るのは E8 の後。
3. 未確認: ブラウザ(wasm)での最小化 → 復元の通し、node 84/55/87 行の実際の変化(alpha1 の予測表は種類のみ)、alpha11 の e2e は擬似 export のみ。SC_CLOSE のページ側の配線は無い。
