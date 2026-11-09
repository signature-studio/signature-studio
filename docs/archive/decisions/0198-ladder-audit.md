# 0198: ladder(G0〜G11)の字義どおりの監査 — 各段階の受け入れ条件を 69 項目に分解し、ブラウザ(Chrome 156 ヘッドレス)の実エンジンの観測と突き合わせた。監査と軽い再観測だけで、実装・tracked ファイルの編集はしていない。仮の決定

状態: **仮の決定(判定の語彙、曖昧語の読み、「一定時間」「領域」などの選択はすべて仮。PM が直してよい)**。依頼は PM(番号 0198。決定 0198 の範囲は「読み取りと軽い再観測だけ」)。作業者は ladder2(サブエージェント。task ツールを持たないので調査・再観測・文書作成を自分で行った)。基準は HEAD **504348b**(ブランチ main)。作業の終わり近くに HEAD が **8384133**(決定 0190: エンジンの中の Serum 2 が setupProcessing/setActive/setProcessing と最初の process に届く)へ進んだので、0190 の冒頭三点を読んで G8-6・G9-3・G11-1 を更新し、`ladder-update-proposal.diff` の dry-run は 8384133 の `git archive` で確認した。再観測は、配信中の 5173 に触れず、自前の `vite preview`(ポート 4510〜4514、4520・4521)を、`FL_ASSETS_DIR=<abs>/lift/out-websrv/assets`(既定の入口の manifest の md5 `41d5486c83f4…` = SHIPH、決定 0189)、**URL パラメータ無し**(音声の項目だけ `?audio=1`)、隔離 Chrome 156.0.8078.47(V8 15.6.75.19)ヘッドレス(`app/test/e2e/browser.mjs` の隔離: 127.0.0.1 以外は名前解決不可、プロキシ・IPv6 なし)で行った。ページは HEAD の `app/dist`(`index-BRKVbDtt.js`)。**実機 Windows・依頼者の機械との照合は 0 件**。この機械は 16 コア・SwiftShader・load 16〜57(他の担当が同時に走っていた)で、**速度は主張しない**(first idle の秒数は負荷に支配された値)。

## 冒頭の三点

1. **何を調べたか**: `docs/investigations/goal-ladder.md` §1 の表の「ブラウザで観測できる受け入れ条件」を字義どおりに読み、G0〜G11 を **69 個の判定項目**に分解した(`tables/ladder-items.tsv`。曖昧な語は読みを併記)。各項目について、決定 0116〜0193、`docs/investigations/**`、`docs/milestones/performance-targets.md`、`lift/out-git/*-results/` の実測を探し、**どの環境(native / node / Chrome)で、実エンジンか、既定の入口か、いつ**観測されたかを付けた。観測の無い項目は「未観測」、native/node だけの項目は「native/node のみ」、代役・保存した画像の再生・試験用の刺激・推測・P1〜P3 規則に当たる物は「数えない」、実機の画面・音が要る項目は「依頼者の供給が必要」とした。さらに、既定の入口の実エンジンを隔離 Chrome で **R1〜R5(窓の一覧・画素・入力・メニュー・ホイール・ドラッグ・再生・File>Open を 1〜3 回ずつ)と A1・A2(AudioWorklet)**で再観測した(`tables/ladder-reobserve.tsv`)。

2. **結論**: (a) **最も厳しい読み**(段階の全項目が、ブラウザ・実エンジン・既定の入口で観測され、下位の全段階が到達で、P1〜P3 規則を適用し、曖昧語は不利な読み)の最高到達段階は **G0**(しかも G0 の根拠は 2026-10-28 の FRUITY 無効 wasm の観測で、出荷構成は a4 の行を出せない)。(b) **字義どおりの読み**(ladder §1 自身が「厳密な直列ではない」と書くので、段階ごとに自分の項目で判定し、曖昧語は読み A〔能力〕を採る)は **G4**。ただし G1・G2・G3 に**各 1 項目の記録の欠け**があり(G1-7: Worker ログ上の Run 入口〔未観測。G1-5・G1-6 は native のみ〕、G2-7: `document.title` への反映〔読みで分かれる〕、G3-4: 実エンジン自身の `WM_TIMER`〔未観測〕)、「G1〜G3 を通過した」と書くならこの 3 項目を明記する必要がある。連続性を要求する字義なら G0 のまま。(c) 従来の報告「最高 G0、G1〜G4 は観測のみ」は**過大ではない**。ただし G2・G4 はほぼ充足、G1・G3 は各 1 項目の欠けと**具体化できる**。逆に G7「未達」、G8・G10「未測定」は 2026-11-03 の観測と合わず**古い**(G6・G7 は項目をほぼ充足するが、P2・P3 が未達なので ladder の注記により「達成」と数えない)。(d) 依頼者の供給が要るのは G4 の読み C、G5 の比較、G8 の書き出しの比較。

3. **未確認**: 実機 Windows との照合(0 件)、依頼者の機械・ブラウザの値(0 件)、Firefox・Safari・headed・GPU・HiDPI、耳での確認、出荷系の構成の Chrome の canvas と native の同状態の面の突き合わせ(G4-4b。Chrome と node の同じ wasm の基準との一致〔G4-4〕は R1・R2・R5 で確認した)、現行 HEAD の FRUITY 無効 wasm を Chrome で走らせた a4 の 7 行(G0)、Worker ログ上の Run 入口(G1-7。出荷構成に計装が無く、計装つき wasm の測定は無い)、実エンジン自身の `WM_TIMER`(G3-4)、ホイール・ドラッグで変わった**値**(画素が変わったことだけ見た)、`?audio=1` の 300 s、所定のプロジェクト(`serina1-serum2.zip`)は実エンジンで一度も開いていない。

## 進捗の誠実性(AGENTS.md の節、`docs/policies/progress-integrity.md` に従う)

**実エンジンが実行された事実(私が今回実行して観測した物)**: 既定の入口(パラメータ無し)の `fl-engine-ship1-SHIPH`(`engineKind: emscripten-fruity-v1`、STAND-IN の帯なし、`window.__app` の `engineKind` と `#standin-banner` を R1〜R5 で読んだ)を、隔離 Chrome 156 のページで開き、エンジンが first idle に着き、窓表に主窓・ウィザード・TApplication が現れ、canvas に画素が出て、CDP の trusted 入力(`Input.dispatchMouseEvent`/`dispatchKeyEvent`)に反応し、メニューが開き、ホバーで強調が変わり、ホイール・ドラッグ・再生ボタンで画面が変わり、`?audio=1` で AudioWorklet がエンジンのブロックを連続して受け取ることを見た(R1〜R5、A1、A2。**画像は実エンジンの canvas を Chrome が撮った物。再生・事前生成はしていない**)。**代役・再生・推測が含まれる範囲**: 代役 `surface-table-v1` は使っていない。入力は人の手ではなく CDP が作るブラウザ自身の trusted イベント(isTrusted)で、人の操作の代わり(模擬)である。過去の実測(native/node/他の構成の Chrome、決定 0116〜0193)は**文書の転記で、再測定していない**(表の `reobserved` 欄が `No` の項目)。判定の「読み」(曖昧語の取り方)は私の選択で §9 に S- として一覧する。**「実エンジン」の意味**: 既定の入口の構成 SHIPH は、FLEngine と同梱 DLL を lift した物に、Blend2D のコンテキスト 18 入口の上流 Blend2D 0.11.1(JIT なし)への置換(決定 0160・0167)、dsp の刈り込み(0168)、dsp の熱い関数 1 本の HLE(0162)、直接 wasm の混成 977 関数(0189)、決定 0171・0178 の import の行(モデルの答え)を足した出荷構成である。これらは PM が受け入れた構成で、代役ではないが、「無改造の lift」ではない。

## 1. 判定の語彙(`verdict` 列)

| 記号 | 意味 |
|---|---|
| `OBS-B-DEF` | ブラウザ(Chrome 156 ヘッドレス)・実エンジン・**既定の入口**で観測済み(本監査の再観測を含む) |
| `OBS-B-OTH` | ブラウザ・実エンジンで観測済みだが、既定の入口以外の構成(opt-in のパラメータ、過去の構成、試験用バンドル・manifest) |
| `NATIVE-ONLY` | native(または node)だけ。ブラウザ未観測 |
| `INDIRECT` | 直接の記録は無く、下流の観測・同一コードの同一出力からの間接 |
| `NOT-OBS` | 未観測 |
| `EXCLUDED` | 数えない規則に該当(試験用の刺激、代役、推測、ladder 冒頭の P1〜P3 の注記) |
| `OWNER` | 依頼者の供給が必要(実機の画面・音) |
| `AMBIG` | 曖昧語の読みで判定が分かれる(両方の読みを併記) |

環境の語: native = lift した C の native ビルド(-O0/-O2)、node = 同じ wasm を node で実行、Chrome = 隔離 Chrome 156 ヘッドレスの Web Worker。

## 2. 結果: 段階 × 字義どおりの判定

「欠け」「次に観測すべきこと」は §4 の項目表と対応する。**項目の判定は、ladder §1 の受け入れ条件を字義どおりに分解したもの**で、判定語彙は §1。

| 段階 | 字義どおりの判定 | 項目の内訳 | 次に観測すべきこと(1 行) |
|---|---|---|---|
| G0 | 到達(条件つき: ブラウザの観測は 2026-10-28 の FRUITY 無効 wasm。現行 HEAD の Chrome では未再観測) | 4 項目: NATIVE-ONLY 1 OBS-B-DEF 1 OBS-B-OTH 2 | 現行 HEAD の FRUITY 無効 wasm を Chrome の Worker で 1 回走らせ、a4 の 7 行を native と diff する(別ビルド。決定 0128 の見積もりで約 27 分、RSS 8.8 GB)。または PM が「FRUITY 有効の出荷構成で DllMain が戻り CreateFruityInstance が走る」を G0 の充足と読む。 |
| G1 | 部分(欠け: G1-7 Worker ログ上の Run 入口〔未観測〕。G1-5・G1-6 は native のみ。G1-1〜G1-3 は下流からの間接) | 7 項目: INDIRECT 3 NATIVE-ONLY 2 NOT-OBS 1 OBS-B-DEF 1 | Run(f_86a660 の入口)で 1 行を出す opt-in の runtime フックを wasm に入れ、Chrome の Worker ログに Run 入口が出ることを見る(出荷構成の再ビルドが要る)。 |
| G2 | 部分(欠け: G2-7 document.title〔読み A=JS が題を読める、なら充足〕。G2-3 のメッセージ列は結果からの間接) | 7 項目: AMBIG 1 INDIRECT 1 OBS-B-DEF 5 | PM が G2-7 の読み(能力か実反映か)を決める。実反映なら app/src/main.ts:593 の近くで appView の主窓の題を document.title に反映(数行)し、R1 と同じ手順で確認する。 |
| G3 | 部分(欠け: G3-4 実エンジン自身の WM_TIMER。G3-3 はホスト作成の試験用タイマーで数えない) | 6 項目: EXCLUDED 1 NOT-OBS 1 OBS-B-DEF 3 OBS-B-OTH 1 | 実エンジンの SetTimer が生きる場面(再生中、ダイアログ、Settings など)を探し、fle_win_timer_log の配送時刻を Chrome の Worker に出して間隔を測る。 |
| G4 | 読み A で到達 / 読み B1〔Chrome == node の基準〕で到達 / 読み B2〔== native〕は出荷系の構成で突き合わせ記録なし / 読み C は依頼者の供給 | 6 項目: NOT-OBS 1 OBS-B-DEF 3 OBS-B-OTH 1 OWNER 1 | 出荷系の構成(F1x/SHIPH)を native -O0/-O2 でも同じ仮想時刻(a3000: lid 14 cf92b670…、lid 48 221e46db…)まで走らせ、面の SHA が Chrome・node と一致するかを見る(G4-4b)。 |
| G5 | 未到達(G5-4 実機との比較は依頼者の供給。既知の欠けあり) | 4 項目: OBS-B-DEF 3 OWNER 1 | 依頼者が実機(Windows、DPI 100%、1920x1080)の起動直後のスクリーンショットを供給する。並行して Browser 欄の黒い領域の原因を調べる(決定 0134 の PM 注記は WebView2 なしと読む)。 |
| G6 | 項目は充足(マウス・キー・ホイール・ホバー・メニュー・ドラッグ)。数えない規則(P2 未達)に該当 | 8 項目: EXCLUDED 1 OBS-B-DEF 7 | P2(120 fps、入力→更新 p95 ≤ 17 ms)を満たす。ホイール・ドラッグは画素の差でなく画面上の値(テンポ、フェーダー)の変化として読む。 |
| G7 | 項目は ?audio=1(opt-in)で条件つきに充足(再生中 30 s で 5 quanta の欠け、無音 10 s は 0)。数えない規則(P3 未達)に該当 | 6 項目: EXCLUDED 1 OBS-B-OTH 5 | PM が「一定時間」の数値を決める(例: 300 s で欠け 0)。?audio=1 を既定にするか。P3。300 s 欠け 0 の独立再現(決定 0175 は 300 s で欠けあり)。 |
| G8 | 未到達(部分: ブラウザで .flp を開くのは試験用バンドルのみ。既定の入口では File>Open が何も起こさない。再生→AudioWorklet、デモ、実機比較が無い) | 7 項目: EXCLUDED 1 NATIVE-ONLY 1 NOT-OBS 2 OBS-B-OTH 2 OWNER 1 | 出荷 SHIPH にブラウザのファイル橋(-DFLE_FILEDLG_BROWSER、決定 0193 の Not done)を載せ、Sampler.flp を開いて再生し、AudioWorklet の非ゼロを Chrome で測る。同梱デモ NewStuff.flp も開く。 |
| G9 | 未到達(native で Fruity Wrapper の load のみ) | 3 項目: NATIVE-ONLY 1 NOT-OBS 2 | Serum を使わない Fruity Wrapper の GUI、または同梱ジェネレーターを 1 つ lift して、Chrome で GUI と音を出す(native では Serum 2 の最初の process の SIGSEGV〔決定 0190〕が先)。 |
| G10 | 未到達(部分: メニュー・モーダル・ドラッグ。Settings・ピアノロール・保存・MP3 が無い。WAV は native と試験構成のみ) | 8 項目: NOT-OBS 3 OBS-B-DEF 3 OBS-B-OTH 2 | Settings・ピアノロールを開く、プロジェクトを保存する、出荷構成で Export を完走させる(b2d3〔決定 0195〕待ち)。 |
| G11 | 未到達(Serum 2/OTT は FL の外の単体のみ) | 3 項目: NOT-OBS 3 | native の実エンジンの中で Serum 2 の最初の process の SIGSEGV(決定 0190)を解いて出力を見る。ライセンスの経路と、サードパーティ VST3 の FL の中での Chrome での実行。 |

項目名に「(参考)」を付けた物(G0-3、G0-4、G6-7)と G5-3(既知の欠け)は ladder の字義の外で、記録のために表に入れた(内訳の数には含まれるが、段階の判定には使っていない)。G8-6(所定のプロジェクト)も字義の外。

## 3. PM が採るべき最高到達段階の読み(2 通り)

| 読み | 規則 | 最高到達段階 | 一番近い欠け |
|---|---|---|---|
| **最も厳しい** | 段階 k の全項目が `OBS-B-DEF`(ブラウザ・実エンジン・既定の入口)で、G0〜G(k-1) もすべて到達。`INDIRECT`/`AMBIG`/`EXCLUDED` は数えない。ladder 注記の P1〜P3 を適用。曖昧語は不利な読み | **G0**(条件つき。G0 の a4 の行 7 本は FRUITY 無効 wasm の 2026-10-28 の観測で、出荷構成は `a4: init_count=0 of ? (InitUnits not reached)` と出す〔R3、`docs/investigations/verify4/logs/a4/von2/a4-lines.txt` と同じ〕。既定の入口の出荷構成で数えるなら G0 の字義の行も出せない) | G1-7(Worker ログの Run 入口) |
| **字義どおり** | 段階ごとに自分の項目で判定する(ladder §1「厳密な直列ではない」)。ブラウザの実エンジンの観測なら構成を問わず数える(`OBS-B-OTH` も可)。曖昧語は PM が決めるまで読み A(能力)を採り、読み B を併記。ladder 注記の P1〜P3 は適用 | **G4**(読み A)。G5 は依頼者の供給で止まる | G1-7、G2-7(読み B なら)、G3-4、G4-4b(読み B2 なら) |
| (参考)字義どおり + 連続を要求 | 上に加えて G0〜G(k-1) を全項目で要求 | **G0** | G1-7 |

感度(PM が決める点): (i) G1-7(と native のみの G1-5・G1-6)を「下流の観測(主窓・入力応答・同一コードの idle)で代える」なら G1 が到達。(ii) G2-7 を能力読み(JS が題を読める)にするなら G2 が到達。(iii) G3-3 の試験用タイマーを数えるなら G3 が到達(私は数えなかった: ホスト作成の刺激で、実エンジンのタイマーではない)。(iv) 3 つを認めると連続で **G4(読み A)**。認めなければ G0。**G6・G7 は項目をほぼ満たすが、P2・P3 を満たすまで「達成」と数えない**(ladder の注記)。

従来の報告との照合: PM が書いてきた「最高 G0、G1〜G4 は観測のみ」(roadmap-to-goal.md の区分表、progress.md)は、連続・全項目の読みと一致し**過大ではない**。「観測のみ」を具体化すると、G2・G4 はほぼ充足、G1・G3 は各 1 項目の欠け(G1-7、G3-4)。**古い**ラベルは、progress.md/roadmap の G7「未達」(本監査 A2 は項目をほぼ満たす)、G8「未測定」(決定 0144・0178・0193)、G10「未測定」(決定 0171・0188・0193)。

## 4. 段階ごとの項目表(全文は `tables/ladder-items.tsv` — 証拠の出典・読みの説明つき)

### G0

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G0-1 | Web Worker 内の wasm が DllMain→InitUnits を 942/942 で完走する | `OBS-B-OTH` | Chrome Worker(+node); FRUITY 無効の wasm(配信していない構成) | No(現行 HEAD の Chrome では未再観測。出荷構成には a4 の計装が無い: R3 の Worker ログは `functions executed (distinct): 0`、`a4: init_count=0 of ? (InitUnits not reached)`。FRUITY 有効の構成では仕様どおりの出力 = docs/investigations/verify4/logs/a4/von2/a4-lines.txt) |
| G0-2 | a4 の行 7 本が native と一致する | `OBS-B-OTH` | Chrome Worker + native + node; 同上 | No |
| G0-3 | (参考)現行 HEAD の native で a4 が不変 | `NATIVE-ONLY` | native; FRUITY 無効 | No |
| G0-4 | (参考)既定の入口の出荷構成で DllMain が戻る | `OBS-B-DEF` | Chrome Worker(出荷構成 SHIPH); 既定の入口 | R3 |

### G1

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G1-1 | 署名検査(WinVerifyTrust)を通過している | `INDIRECT` | Chrome Worker(出荷構成); 既定の入口 | R3 |
| G1-2 | CPU 検査を通過している | `INDIRECT` | native(直接)+Chrome(下流の観測); 既定の入口(下流) | R1〜R3(下流) |
| G1-3 | BasicInitialization を通過している | `INDIRECT` | native + Chrome(下流); 既定の入口(下流) | R1〜R3(下流) |
| G1-4 | TApplication の設定が済んでいる | `OBS-B-DEF` | Chrome Worker(出荷構成); 既定の入口 | R1,R2,R3 |
| G1-5 | 初期化の本体(f_13751e0)を通過している | `NATIVE-ONLY` | native(gdb 入口数)+Chrome(下流); native -O0 計装; Chrome は下流 | No |
| G1-6 | Run(0x1375950)の入口に着いている | `NATIVE-ONLY` | native; native -O0 計装(ダイアログ付き構成と最終構成) | No |
| G1-7 | Worker のログ(診断フック)に Run の入口への到達が出る | `NOT-OBS` | —; — | R3(行なしを確認) |

### G2

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G2-1 | 窓表に可視の最上位ウィンドウが作られる(隠しの TPUtilWindow を除く) | `OBS-B-DEF` | Chrome Worker(出荷構成)+native; 既定の入口 | R1,R2,R3 |
| G2-2 | クラス登録・CreateWindowExW・ShowWindow が実エンジンのコードに対して行われる | `OBS-B-DEF` | Chrome(結果)+native; 既定の入口(結果) | R3 |
| G2-3 | WM_NCCREATE〜WM_CREATE の配送 | `INDIRECT` | native/unit + Chrome(結果のみ); — | No |
| G2-4 | サイズが JS から読める | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2,R3 |
| G2-5 | タイトルが JS から読める | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2,R3 |
| G2-6 | canvas のサイズに反映できる | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G2-7 | document.title に反映できる | `AMBIG` | Chrome; 既定の入口 | R1,R2 |

### G3

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G3-1 | エンジンが PeekMessageW/WaitMessage/MsgWaitForMultipleObjects で待つ(idle に入る) | `OBS-B-DEF` | Chrome Worker; 既定の入口 | R1,R2,R3,A1 |
| G3-2 | 待つ間もタブが応答する(別スレッドの rAF が維持される) | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2,R5 |
| G3-3 | WM_TIMER が等間隔で届く — (a) ホストが窓に付けた試験用タイマー | `EXCLUDED` | native/node/Chrome; 試験用タイマー | No |
| G3-4 | WM_TIMER が等間隔で届く — (b) 実エンジン自身のタイマー | `NOT-OBS` | native; — | No(出荷構成の Worker ログに配送の記録は出ない) |
| G3-5 | アイドルで CPU を使い切らない | `OBS-B-OTH` | Chrome; 既定の入口 + ?audio=1 | A1,A2 |
| G3-6 | エンジンのメッセージ配送が回る(入力が WndProc に届く) | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |

### G4

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G4-1 | エンジンが描いた画素が canvas に出る | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2,R3 |
| G4-2 | 空白でない | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G4-3 | エンジンの GDI 呼び出し結果と画素が一致する — 読み A(canvas == Worker が書いた面) | `OBS-B-OTH` | Chrome; 過去の構成(D 系) | No |
| G4-4 | 同 — 読み B1(canvas == 同じ wasm を node で走らせた 82 行基準の面) | `OBS-B-DEF` | Chrome + node; 既定の入口 | R1,R2,R5(lid 48) |
| G4-4b | 同 — 読み B2(canvas == native の同じ状態の面。出荷系の構成 F1x/SHIPH) | `NOT-OBS` | Chrome + native; — | No |
| G4-5 | 同 — 読み C(実機 Windows の GDI の画素) | `OWNER` | —; — | — |

### G5

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G5-1 | 起動直後のメイン画面全体(メニュー、ツールバー、ブラウザ、プレイリスト、ミキサーなど)が描かれる | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2(目視) |
| G5-2 | テキストとアイコンを含む | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2(目視) |
| G5-3 | 既知の欠け(判定には使わないが記録) | `OBS-B-DEF` | Chrome; 既定の入口 | R1(黒い Browser 欄とメッセージのみ) |
| G5-4 | 実機のスクリーンショットと領域ごとに比較できる | `OWNER` | —; — | — |

### G6

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G6-1 | マウスが WM_* としてエンジンに届く | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G6-2 | キーボードが届く | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G6-3 | ホイールが届き画面が変わる | `OBS-B-DEF` | Chrome; 既定の入口 | R2,R5 |
| G6-4 | ホバーの強調(画面が変わる) | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2,R5 |
| G6-5 | メニューが開く | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G6-6 | ドラッグ | `OBS-B-DEF` | Chrome; 既定の入口 | R2,R5 |
| G6-7 | 再生ボタンで画面(時計)が動く | `OBS-B-DEF` | Chrome; 既定の入口 | R2,R5 |
| G6-8 | 数値の条件 P2(UI 120 fps・遅延ほぼなし) | `EXCLUDED` | Chrome; 既定の入口 | R1,R2(参考) |

### G7

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G7-1 | エンジンが音声出力を開く | `OBS-B-OTH` | Chrome(?audio=1)+native; ?audio=1(opt-in。既定の入口に AudioWorklet は無い: performance-targets.md §4d、panel p3 na) | A1,A2 |
| G7-2 | 空のプロジェクトで再生する | `OBS-B-OTH` | Chrome; ?audio=1 + 既定 | A2(再生の確認),R2 |
| G7-3 | AudioWorklet が一定のブロックを連続して受け取る | `OBS-B-OTH` | Chrome; ?audio=1 | A1,A2 |
| G7-4 | アンダーラン 0 を一定時間 | `OBS-B-OTH` | Chrome; ?audio=1 | A1,A2 |
| G7-5 | 出力は無音でよい | `OBS-B-OTH` | Chrome; ?audio=1 | A1,A2 |
| G7-6 | 数値の条件 P3(Song Mode の再生で音が途切れない) | `EXCLUDED` | —; — | — |

### G8

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G8-1 | .flp を読み込む(ファイル I/O。OPFS または MEMFS) | `OBS-B-OTH` | native + Chrome(既定外); 試験用バンドル(出荷 SHIPH に -DFLE_FILEDLG_BROWSER は未搭載。既定の入口で File>Open... は何も起こさない: R4) | R4(既定の入口では開けないことの確認のみ) |
| G8-2 | 同梱デモを読み込む | `NOT-OBS` | —; — | No |
| G8-3 | 画面に反映される | `OBS-B-OTH` | Chrome; 試験用バンドル | No |
| G8-4 | 再生すると AudioWorklet に非ゼロのサンプルが出る(読み込んだプロジェクトで) | `NATIVE-ONLY` | native; — | No |
| G8-5 | レンダリング結果を実機の書き出しと比較できる | `OWNER` | —; — | — |
| G8-6 | 所定のプロジェクト(serina1-serum2.zip)を開く | `NOT-OBS` | native; — | No |
| G8-7 | 数値の条件 P1〜P3(「G8 を達成と数えない」) | `EXCLUDED` | —; — | — |

### G9

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G9-1 | Plugins\Fruity の DLL を LoadLibraryW + CreatePlugInstance で読む | `NATIVE-ONLY` | native; — | No |
| G9-2 | GUI が出る | `NOT-OBS` | —; — | No |
| G9-3 | 音が出る | `NOT-OBS` | —; — | No |

### G10

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G10-1 | メニュー | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G10-2 | モーダル | `OBS-B-DEF` | Chrome; 既定の入口 | R1,R2 |
| G10-3 | ファイルダイアログ | `OBS-B-OTH` | Chrome; 試験用バンドル(既定の入口では不可) | R4 |
| G10-4 | ドラッグ | `OBS-B-DEF` | Chrome; 既定の入口 | R2 |
| G10-5 | ミキサー・ピアノロール・設定を一通り操作 | `NOT-OBS` | Chrome; 既定の入口 | R2(フェーダーのみ) |
| G10-6 | プロジェクトの保存 | `NOT-OBS` | Chrome(描画のみ); — | No |
| G10-7 | WAV の書き出し | `OBS-B-OTH` | native + Chrome(試験構成); 試験用 manifest | No |
| G10-8 | MP3 の書き出し | `NOT-OBS` | —; — | No |

### G11

| 項目 | 字義の項目 | 判定 | 環境・構成 | 本監査の再観測 |
|---|---|---|---|---|
| G11-1 | VST2/VST3/CLAP の第三者プラグイン | `NOT-OBS` | Chrome(単体)/native(FL 内); — | No |
| G11-2 | ライセンス検証 | `NOT-OBS` | —; — | No |
| G11-3 | 更新確認・ネットワーク | `NOT-OBS` | Chrome; 既定の入口 | R1(目視) |

## 5. 再観測(軽い)の記録

全て `nice -n 15`、Chrome は同時に 1 つ(順に実行。他の担当の Chrome が別に動いていた)、`FL_PERF_LOG` は自分の `lift/out-ladder2/run*/perf-log.jsonl`(配信中の `lift/out-websrv/perf-log.jsonl` は 18 行のまま)。ネットワーク記録(netlog。`logs/netlog-lines.txt`)は全 7 実行で `non_loopback_tcp_connects=0`、`udp_bytes_sent_events=0`(決定 0124 と同じ `[2001:4860:4860::8888]:443` への `connect()` の試みだけ)。Chrome 自身の背景の要求(accounts.google.com、www.google.com、update.googleapis.com。9〜12 件)は名前解決不可(-105)で失敗した。

| 実行 | 内容 | コマンド(`<abs>` = このリポジトリの絶対パス) | load(開始/終了) |
|---|---|---|---|
| R1 | 窓表・canvas・画素・ウィザード・メニュー・ホバー・Escape・rAF | `APP_WORK=lift/out-ladder2/appwork FL_ASSETS_DIR=<abs>/lift/out-websrv/assets nice -n 15 node lift/out-ladder2/tools/reobserve.mjs --out lift/out-ladder2/run1 --port 4510` | 23.1/31.6 |
| R2 | R1 + 無入力の対照 + 再生ボタン + ホイール + ドラッグ + Worker ログの a4 行 | `… node lift/out-ladder2/tools/reobserve2.mjs --out lift/out-ladder2/run2 --port 4511` | 29.2/43.0 |
| R3 | 窓表と Worker ログ全 371 行(`logs/run3-engine-log.txt`) | `… reobserve3.mjs --out lift/out-ladder2/run3 --port 4512` | 45.5/56.8 |
| R4 | 既定の入口で File>Open… を押す(8 s 観察) | `… reobserve4.mjs --out lift/out-ladder2/run4 --port 4513` | 36.8/34.9 |
| R5 | R2 の繰り返し | `… reobserve2.mjs --out lift/out-ladder2/run5 --port 4514` | 16.0/16.9 |
| A1 | `?audio=1`、トランスポートは再生していない(§6) | `cd app && APP_WORK=… FL_ASSETS_DIR=… nice -n 15 node test/e2e/audio-g7.mjs --bundle /assets/manifest.json --out lift/out-ladder2/run-audio1 --quiet-s 10 --play-s 20 --post-enter-s 10 --idles 0 --port 4520`(既存の道具。無改造) | 28.9/22.0 |
| A2 | `?audio=1`、再生ボタンを (358,20) でクリックして再生 | 同上、`--out lift/out-ladder2/run-audio2 --play-s 30 --click '358,20' --shot --port 4521` | 35.2/33.7 |

観測の要点(数字は `tables/ladder-reobserve.tsv` の実測):

- **窓・画素(R1〜R5)**: R1・R2・R3・R5 の 4 回で窓表 29 窓・可視 3(TWelcomeWizard 874x570、TFruityLoopsMainForm "FL Studio 2026" 1930x1092 @(-5,-6)、TApplication 0x0)。`document.title` は R1・R2・R5 の全てで `fl-engine-Oz · FL Studio web (private)`(エンジンの題は反映されない)。first-idle の画面は非黒 1,839,552/2,024,099 画素、8,807〜9,092 色。
- **入力(R1・R2・R5)**: ウィザードのボタンのクリックで面 SHA が `221e46db…` → `e5b281ff…`(3 回とも同値)。ウィザードのボタンへのホバーは 3 回とも SHA 不変(強調が無いか未配送か未判別)。Enter でウィザードが閉じる。File のクリックで `TQuickPopupMenuWindow` 208x298 が開き Escape で閉じる。メニュー項目へのホバーでポップアップ SHA が変わる(対照〔無入力 1.2 s 後〕は同値で安定。変化後の値は R1・R2 が `a9cd00c8…`、R5 が `3372ba15…` の 2 種で、理由は未調査)。
- **ホイール・ドラッグ・再生(R2・R5)**: 無入力 1.5 s の対照は時計・テンポ・フェーダー・再生ボタンの領域で 0 画素差(画面全体で 27〜90 画素)。再生ボタンのクリックで時計 864/533 画素・ボタン 971 画素が変化し、停止で時計は再生前と 0 画素差。テンポ欄上のホイール 3 ノッチで 1,412 画素(2 回とも同値)、マスターフェーダーのドラッグで 977 画素(2 回とも同値)。**表示値は読んでいない**。
- **出荷構成の Worker ログ(R3)**: 署名の主張表 2 行(`established=yes`、`not verified: chain,revocation,time`)、`calling entry … == stop: returned from entry … rax=0x1`、`functions executed (distinct): 0`(計装なし)、`a4: init_count=0 of ?`、`fruity: calling CreateFruityInstance(…)`、`G2 evidence of the main form (decision 0126): TFruityLoopsMainForm became visible`。**Run の入口を示す行は無い**。
- **File>Open…(R4)**: メニュー項目を押すとメニューは閉じ、ページの要求 0 件・箱なし・窓なし・停止なしで engine-idle のまま(8 s)。出荷 SHIPH にはブラウザのファイル橋が無い(決定 0193)。
- **音(A1・A2)**: AudioWorklet は 35,995 / 40,013 quanta を連続して受け取った(ctx 104.5 / 116.1 s)。A2(再生中と画像で確認): 無音 10 s は新規アンダーラン 0、再生 30 s は 5 quanta(14.5 ms、1 episode)。起動段階(ページを開いて約 40 s まで)に 645〜1,029 quanta/26〜35 episode(最長 87〜90)。全体 681(A2)/1,049(A1)。出力は無音(peak 0)。Worker の CPU は無音 19%、再生 33.7%(A2、load 36〜38)。**A1 は再生していない**(§6)。エンジンが主窓の面をページへ送る間隔は 62.5 回/s、p50 16.0 ms、p99 28〜32 ms(A1。アイドル再描画で、`WM_TIMER` ではない)。

## 6. 自分の誤りと訂正

1. **A1 の再生の検知は無効だった(第 1 報で気づいて訂正)**: `audio-g7.mjs` の「再生中」の判定は窓の更新率 ≥ 3/s で、出荷構成はアイドルでも主窓を 62.5 回/s 描き直す(決定 0187)ため、再生していなくても常に true になる。A1 の `playSeen: true` は無効で、Space は再生フェーズの後(75.5・92.9・110.4 s)に「停止」の試みとして 3 回押されただけ。A1 の「30 s で新規 0」はトランスポート停止中の値。再生の確認は A2(再生ボタンのクリック + 画像の一時停止アイコンと時計 1:16:14)で取り直した。
2. **R1 の「hoverChangedMainSha」は交絡していた**(主窓はアイドルでも描き直され SHA が動く)。ホバーの判定は R2 以降、ポップアップ窓の SHA と無入力の対照に変えた。R1 の `E4_file_menu_click.hoverChangedMainSha` は判定に使っていない。
3. 存在しない `bash` ツールを 1 回呼んだ(shell ツールで再実行。副作用なし)。`sleep` を 1 回使って拒否された(規則違反。待ちは背景実行の完了通知に切り替えた)。

## 7. ladder 文書の古い記述(`tables/ladder-stale.tsv`。書き換え案は `ladder-update-proposal.diff`。古い記述は消さず、§2.1「2026-11-03 時点の監査」を追記する形)

| goal-ladder.md の行 | 場所 | 古い記述(要約) | 今の状況(出典) |
|---|---|---|---|
| L37 | §1 表 G1 の「現在の状況」 | 実測（部分。native のみ…）。初期化の本体 f_13751e0 は 1 点も実行されていない。ブラウザ（wasm）では未測定 | 古い: native では Run の初入口まで到達(決定 0126: 134,400,394、決定 0138)。ブラウザ(Chrome の Worker)でも実エンジンが最初の idle・主窓・入力応答に到達(決定 0128・0137・0161、本監査 R1〜R5)。欠けは Worker ログ上の Run 入口の記録(G1-7)。 |
| L38 | §1 表 G2 | 未測定（隠しの補助ウィンドウ 1 つだけ実測。決定 0071）。TApplication の窓で止まる地点まで | 古い: 主窓 TFruityLoopsMainForm が visible(native 決定 0126、wasm 0128、Chrome 0141、本監査 R1〜R5)。欠けは document.title(G2-7、読みで分かれる)。 |
| L39 | §1 表 G3 | 未測定（…実エンジンのループは未実行） | 古い: idle で JS に制御が返り入力が WndProc に届く(決定 0128・0136、本監査)。欠けは実エンジン自身の WM_TIMER(G3-4)。 |
| L40 | §1 表 G4 | 未測定 | 古い: canvas 読み戻し(決定 0128・0137、本監査 R1〜R5)。 |
| L41 | §1 表 G5 | 未測定（実機のスクリーンショットも未入手） | 部分的に古い: 画面は描かれ文字・アイコンもある(本監査 R1 の画像)。実機のスクリーンショットが無い点は今も同じ(G5-4)。既知の欠け(Browser 欄が黒い等)を追記すべき。 |
| L42 | §1 表 G6 | 未測定 | 古い: マウス・キー・ホイール・ホバー・メニュー・ドラッグが Chrome で届き画面が変わる(決定 0136・0141・0171、本監査 R2・R5)。P2 規則で「達成」とは数えない。 |
| L43 | §1 表 G7 | 未測定（AudioWorklet 側の受け皿は JS 試作で実測。エンジン側の音声経路は未実行） | 古い: 決定 0139・0175、本監査 A1・A2(エンジンのブロックを AudioWorklet が連続受領、無音)。P3 規則で「達成」とは数えない。progress.md の「G7 未達」も本監査の項目とは合わない。 |
| L44 | §1 表 G8 | 未測定 | 古い: native で .flp を開いて再生(決定 0144)、Chrome で .flp を開く(決定 0193、試験用バンドル)。既定の入口では File>Open が何も起こさない(R4)。 |
| L45 | §1 表 G9 | 未測定（棚卸しだけ） | 部分的に古い: native で Fruity Wrapper の DllMain と CreatePlugInstance(決定 0183)。GUI・音は未。 |
| L46 | §1 表 G10 | 未測定 | 古い: メニュー・モーダル・ドラッグ・native の WAV 書き出し(決定 0171・0188・0193)。 |
| L47 | §1 表 G11 | 未測定 | 部分的に古い: Serum 2/OTT の単体の実行(決定 0156・0165・0177・0186)、FL の中の native(決定 0183・0190: Serum 2 の setupProcessing/setActive と最初の process の呼び出し、その中で SIGSEGV)。FL の中のブラウザ実行は 0 件。 |
| L49 | §1 横断の軸 (b)(c) | a4 の初期化は wasm で約 3 秒…／wasm 389 MB。brotli で 44 MB | 古い: 出荷構成 SHIPH は wasm 171,302,606 B(ネイティブ 6 モジュール合計の 2.427 倍)、brotli 13.30 MB、PM の 1 回の測定で最初の idle 6.2 s(performance-targets.md §4d)。 |
| L53 | §2 現在の最高到達段階 | G0（…a4 の行 7 本が一致） | G0 の判定自体は変わらない。ただし根拠は FRUITY 無効 wasm(2026-10-28)で、出荷構成は a4 の行を出さない(R3)。 |
| L55 | §2 G1 は部分 | …ブラウザ（wasm）では CreateFruityInstance を一度も呼んでいない | 古い: 出荷構成は Chrome の Worker で CreateFruityInstance を呼び、Run の中で idle に着く(R3 ログ: calling CreateFruityInstance…、主窓の G2 evidence)。 |
| L56 | §2 G2 以降 | G2 以降は、実エンジンのコードが実行された記録が無い | 誤り: 決定 0126〜0193 と本監査。 |
| L227-L238 | §3.10 の G0〜G11 行(根拠の強さ) | G1 実測（停止点まで）…f_13751e0 の先は 1 点も実行されていない 等 | 静的分析(2026-10-30)時点の見積もり。実行の記録が増えたので、行ごとに「2026-11-03 の監査」を併記すべき。 |
| L270 | §4.2 U4 | 実エンジンのメッセージループは未実行 | 古い: 実エンジンのループは native と Chrome で回る(決定 0128・0136・0166)。JSPI のスタック深さ・モーダルのネストの未測定は別問題。 |
| L337 | §6.2 ゴール到達の例 | ゴール到達: G0 まで…G1 は部分、G2〜G11 は未測定 | 古い(本監査の表で置き換える)。 |
| L348 | §6.2 表の例 | G2 未測定｜G3 未測定｜G4 読みのみ｜G6 読みのみ｜G7 読みのみ… | 古い。 |
| L365 | §6.3 A3 | G1 の wasm 版は未測定 | 古い。出荷構成で Run の中の idle に着く(Run 入口の直接記録は無い: G1-7)。 |
| L369 | §6.4 推奨 1 | 到達: G0…G1 は native で部分。G2 以降は未測定または読みのみ | 古い(本監査の表と 2 通りの読みで置き換える)。 |
| L384 | §7.1 の 7 | …CreateFruityInstance を wasm／ブラウザで一度も呼んでいない | 古い。 |
| L390-L398 | §7.2 確認できなかったこと | f_13751e0 の下で実際に踏む行なし API の数と種類／音声のミキサースレッド／FLP を開いたとき走るコード 等 | 一部は決定 0129〜0193 で解消(ミキサースレッド=0139・0147、FLP=0144・0178・0193)。 |

## 8. 依頼者の供給が必要なもの

- **実機 Windows の起動直後のスクリーンショット**(G5-4 と G4 の読み C。DPI 100%、1920x1080。`~/ダウンロード` に画像は無い)。
- **実機の書き出し WAV**(G8-5。同じ空のプロジェクトまたは `Sampler.flp` を実機の FL Studio で書き出した物)。
- **耳での確認**(G7 の音、G8 の音の正しさ。0 件)。
- **依頼者の機械・ブラウザの値**(P1〜P3 の判定)。計測ビーコン(決定 0152・0192)の記録は 0 件のまま。

## 9. 出典のない選択(S-)

| 名前 | 選択 | 理由と代替 |
|---|---|---|
| S-ladder-chain | 「最高到達段階」を、連続を要求する読み(厳しい)と、段階ごとに判定する読み(字義)の 2 通りで示す | ladder §1 が「厳密な直列ではない」と書くので字義は段階ごと。代替: 連続のみ(G0)/段階ごとのみ(G4) |
| S-ladder-readA | G4 の「エンジンの GDI 呼び出し結果と画素が一致」を読み A(canvas の読み戻し == Worker が書いた面)で判定し、読み B(== native の同状態の面)・読み C(== 実機 GDI)を別項目にした | 文言に比較対象が書かれていない。代替: B を必須(G4 は部分)、C を必須(G4 は未到達) |
| S-ladder-g1log | G1 の「Worker のログ(診断フック)」を出荷構成の Worker ログ(`window.__app.engineLog()`)と読んだ | 診断フック付きの H ビルドを Chrome で走らせた記録が無い。代替: H ビルドの測定を要求(同じく未観測)/ 下流の観測で代える |
| S-ladder-g2title | 「`document.title` に反映できる」を、能力(読み A)と実反映(読み B)の両方で示し、判定は `AMBIG` | 「できる」の解釈が分かれる。PM が決める |
| S-ladder-g3timer | ホスト作成の試験用 `WM_TIMER`(決定 0136)を数えない | ゲストの `SetTimer` ではない刺激。代替: 配送の機構の検査として数える(G3 が到達に近づく) |
| S-ladder-g7time | 「一定時間」に数値を決めず、窓ごとの数(10 s・30 s・60 s・300 s)を報告した | 文書に数値が無い。代替: PM が 60 s か 300 s に決める |
| S-ladder-hover | 「ホバーの強調」を、ポップアップ窓の項目へ移動した後の窓 SHA の変化(対照が安定)で判定した | 強調の見た目を特定できない。代替: 目視、特定の色 |
| S-ladder-region | ホイール・ドラッグ・再生の判定を、スクリーンショット(1897x1067)の矩形領域の画素差 > 0(無入力の対照が 0)で行った。矩形は私が決めた(時計 [495,0,120,40]、テンポ [428,5,62,32]、フェーダー [430,580,70,430]、再生ボタン [338,0,30,40]) | 値(テンポの数字など)を読む手段が無い。代替: OCR、エンジンの内部状態 |
| S-ladder-input | CDP の trusted 入力を「ブラウザのマウス・キーボード・ホイール」として数えた | 人の手による入力は再現できない。代替: 実機の操作を依頼者に頼む |
| S-ladder-defconfig | 「既定の入口」= パラメータ無しで `/assets/manifest.json`(md5 `41d5486c83f4…` = SHIPH)を読む構成。`?audio=1` は opt-in なので `OBS-B-OTH` | 決定 0152 の「既定の入口で確認」。代替: opt-in も既定とみなす |
| S-ladder-owner | 実機の画面・音が要る項目を `OWNER` とし、到達に数えない | 比較対象が無い。代替: 目視で「それらしい」を到達に数える(過大になるので採らない) |

## 10. 確認できなかったこと

実機との照合、依頼者の機械の値、Firefox・Safari・headed・GPU・HiDPI、耳、現行 HEAD の Chrome での FRUITY 無効 a4 の 7 行(G0)、Worker ログ上の Run 入口(G1-7)、実エンジン自身の `WM_TIMER`(G3-4。再生中・ダイアログ中・Settings 中は未観測)、出荷構成の canvas と native の同状態の面の突き合わせ(G4-4)、ホイール・ドラッグで変わった値、`?audio=1` の 300 s 以上、メトロノームありの非無音(決定 0175 が D 構成で見た)の出荷構成での再現、同梱デモ `NewStuff.flp`、Settings・ピアノロール、プロジェクトの保存、メニュー項目ごとのホバーの値が 2 種になる理由、ウィザードのボタンのホバーが SHA を変えない理由。

## 11. 規則違反・逸脱の自己申告

- AGENTS.md の「作業はすべて task agent を通じて」: 私は task ツールを持たないサブエージェントなので、調査・再観測・文書作成を自分で行った。
- `bash` という名前の存在しないツールを 1 回呼び(§6)、`sleep` を 1 回使って拒否された(§6)。
- 他の担当の木の変更(`git status` に `lift/crates/fe-lift/src/lib.rs` ほか)は私のものではなく、触っていない。tracked ファイルの編集・git の変更系操作・commit・push は 0 回。配信中の 5173 には接続していない(自前の preview だけ)。
- Chrome は同時に 1 つ、`nice -n 15`、重い処理は 2 つ以内。`/tmp` は使っていない(作業は `lift/out-ladder2/`)。`lift/out-lat1` は読み取りも触れていない(道具は `app/test/e2e/` のものを使った)。
- R1 の最初の実行に `hoverChangedMainSha` という交絡した指標を入れた(§6-2)。判定には使っていない。

## 12. 成果物(`lift/out-git/ladder2-results/`)

`docs/0198-ladder-audit.md`(この文書)、`docs/README-0198-line.txt`、`tables/ladder-items.tsv`(項目表)、`tables/ladder-stage-summary.tsv`、`tables/ladder-reobserve.tsv`、`tables/ladder-stale.tsv`、`logs/`(各実行の `result.json`・`reobserve-*.log`・Worker ログ全文・音声の要約)、`images/`(スクリーンショット 5 枚)、`tools/`(再観測の道具と表の生成)、`ladder-update-proposal.diff`(goal-ladder.md の追記。`patch -p1 --dry-run` が HEAD の `git archive` に当たる)、`MERGE-NOTES.md`。作業ディレクトリ `lift/out-ladder2/`。

## PM の判断と取り込み（2026-11-04）

- **取り込み範囲:** 文書だけ（決定文書、`docs/investigations/ladder2/` の表・ログ・道具・画像 3 枚〔400 KB 超の 2 枚は除外〕、`goal-ladder.md` への追記 `ladder-update-proposal.diff`〔`-` 行 0、`+` 27 行。古い記述は消さない〕）。コード・テストの変更なし。
- **報告の書式（恒久。PM の判断）:** 各ターンの最高到達段階は **2 通りで書く**。(1) 『最も厳しい読み』= 連続・全項目・ブラウザ・実エンジン・既定の入口・P 規則・曖昧語は不利 = **G0**。(2) 『字義どおりの読み』= 段階ごとに自分の項目で判定、曖昧語は読み A（能力）= **G4**。ただし **G1・G2・G3 に各 1 項目の記録の欠け**（G1-7 Worker ログ上の Run 入口、G2-7 `document.title` への反映、G3-4 実エンジン自身の `WM_TIMER`）があり、『G1〜G3 を通過』と書くならこの 3 項目を明記する。ladder2 の評価では従来の『最高 G0、G1〜G4 は観測のみ』は過大ではなかった。逆に G7『未達』・G8『未測定』・G10『未測定』のラベルは 2026-11-03 の観測と合わず古い（`progress.md` と `roadmap-to-goal.md` の更新は docs の作業で行う）。
- **欠けている観測を埋める（小さな作業。起動する）:** (a) G0: 現行 HEAD の FRUITY 無効 wasm を Chrome の Worker で走らせ、a4 の 7 行を native と diff する（10-28 の観測は別構成）。(b) G1: Run（`f_86a660` の入口）で 1 行を出す opt-in のフックを wasm に入れ、Chrome の Worker ログに Run 入口が出ることを見る。(c) G2: **PM の読みは『実反映』を採る**（アプリは主窓の題を `document.title` に反映するのが自然で、manifest 名のままでは『エンジンの窓が反映される』という条件を字義どおりには満たさない）。`app/src/main.ts` の数行で、エンジンの主窓の題を `document.title` に反映する（開始前は従来の名）。(d) G3: 実エンジンの `SetTimer` が生きる場面で配送時刻を Chrome の Worker に出して間隔を測る（決定 0136 の試験用タイマーは数えない）。(e) G4: 出荷系の構成を native でも同じ仮想時刻まで走らせ、面の SHA（lid 14 `cf92b670…`、lid 48 `221e46db…`）を Chrome・node と突き合わせる。
- **依頼者にしか出せないもの:** 実機の起動直後のスクリーンショット（G5 の比較、G4 の読み C）、実機の書き出し WAV（G8 の比較）、耳での確認。
- **限界の再掲:** 再観測は load 16〜57 の機械の隔離 Chrome 156 ヘッドレス（速度は主張しない）。依頼者の機械の値は 0 件。実機 Windows との照合は 0 件。
