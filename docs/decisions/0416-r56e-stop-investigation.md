# 0416: R5-6d と R5-6c の共通の停止の原因の調査(PEQ2 の 0 呼び出し・Soundgoodizer の Exception on open)。実装は原因が確かめられてから別の決定
分類: 測定(調査)。取り込み・製品の変更は、この決定の中では行わない。原因が b(契約の不足)と分かったら、その実装は別の決定(b)として書く。新しい旗・環境変数・`#ifdef` は 0 件(調査の計測は一時の差分で、commit しない)。
撤回条件: (1) 計測の差分を commit したら、この決定に違反する(commit しない。計測の bundle だけに入れる)。(2) 原因の仮説が、計測で外れたら、外れた事実と次の仮説を evidence に書く(仮説を残したまま実装しない)。
日付: 2026-11-17   ROADMAP: R5-6f   一覧: G9、S1   コミット: 調査の記録のみ(計測の差分は含めない)
訂正(2026-11-17): 本文の呼び先の番地 0x3500A0 は誤り。正しくは RVA 0x3503A0。原因と決定は末尾の「結論」に書く(H4・H5・H6 の判定を置き換える)。記録: docs/failures/0030。

## 0 範囲と件数(設計の前の確認)

- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md` は 263 件(`grep -c '^## seq'`)。ID を除いて `ildsp|GetProcAddress|LoadLibrary|DllMain|RaiseException|例外|Exception` に一致した発言は 0 件。FL のプラグインに関わる発言は、0408 の 14 件(seq 5229 他・34564・22192・55145・55453・95591・96849 の 2 件)と同じ。新しい依頼者の発言は、この調査にはない(main の指示だけ)。
- 読んだ記録: 決定 0409(全文)・0411(保留の記録)・evidence/R5-6c.md(全文)・evidence/R5-6d.md(全文)・docs/ROADMAP.md の R5-6c・R5-6d の行。
- 読んだ実装: lift/runtime/fle_load.c(load_by_name・load_with_path)、lift/runtime/fle_modules.c(fle_imp_GetProcAddress の NULL の経路)、lift/runtime/fle_realmod.c(call_dllmain・fle_realmod_load。DllMain の戻り値 0 で LoadLibrary は失敗する)、lift/runtime/fle_proc.c(RaiseException・RtlUnwindEx)。
- 検索の件数: runtime の DllMain の呼び出し 1 件(fle_realmod.c の call_dllmain)。LoadLibrary の実装 3 件(W・A・ExW)。PEQ2 と Soundgoodizer の UTF-16 の `ildsp` は、各 1 件(関数 0x1da550 が 1 件の参照。PEQ2 の RVA 0x1da594)。

## 仮説(測定の前に書く。main の仮説を含む)

- H1(main の仮説): 初期化(DllMain・unit の initialization)の途中で RaiseException が起き、RtlUnwind・SEH の経路が未対応で、初期化が抜ける。その結果、0x3500A0 が設定されない。
- H2(この調査の仮説。R5-6d の §5 から): 0x3500A0 は関数の表(または interface)の値で、ildsp(`Shared\ildsp_x64.dll`)を LoadLibrary し、GetProcAddress で関数を埋める。LoadLibrary が NULL を返すと、GetProcAddress(NULL, …) は NULL を返す(fle_modules.c の注釈の経路)。表のスロットが 0 のまま呼ばれる。
- H3: DllMain が呼ばれていない、または戻り値 0 で、realmod の LoadLibrary が失敗する(call_dllmain の結果)。

## 計測(一時の差分。commit しない)

- 計測点: LoadLibraryW・LoadLibraryA・LoadLibraryExW(名前と結果)、GetProcAddress(モジュールと名前と結果)、RaiseException・RtlUnwindEx(コード)、call_dllmain(番号・reason・戻り値)。すべて stderr への 1 行。engine の製品の挙動は変えない(戻り値・経路は同じ。出力だけを足す)。
- 結果の読み方: 停止の直前の数十行(報告の tail)に、H2 の LoadLibrary の名前と NULL の結果が出るかで判定する。出なければ、H1 と H3 を個別に見る。

## 撤回・次の仮説

- H2 が外れたら、H1(例外の経路)を計測する。H1 も外れたら、0x3500A0 への書き込み(動的な追跡)を計測する。

## 計測の結果(evidence/R5-6f.md。数字の出典はそこ)

- 計測 1(LoadLibrary・GetProcAddress・RaiseException・RtlUnwindEx・DllMain の一時の出力。bundle の tail は 4,000 行に広げた): PEQ2 の LoadLibrary → 0x9000000(成功)、GetProcAddress(CreatePlugInstance) 成功、LoadLibraryW(Frequency Splitter\engine_x64.dll) → 0、User32 成功、Delphi の例外(0xeedfade)3 回が RtlUnwindEx で処理されて続く、LoadLibraryW(`ffmpeg\x64\ilvideo_x64.dll`、相対名) → 0、直後に 0 への間接 call。
- H2(ildsp)は外れた: ildsp を LoadLibrary する呼び出しは、停止までの列に無い。
- H1(例外で初期化が抜ける)は外れた: 3 回の例外は RtlUnwindEx(fle_seh)で処理され、実行は続く。停止は、その後の ILVideo の読み込み失敗の直後。
- 新しい仮説 H4: 0x2155b0 は ILVideo の読み込み(0x1d9390)の戻り値を見ずに進み、0x3500A0 の値を使って 0 のスロットを呼ぶ。ILVideo の失敗は、直接の原因ではない(戻り値を見ていない)。0x1d9390 が書く変数(0x35f188〜0x35f240)は 0x3500A0 ではない。
- H5(新): 0x3500A0 の値が、実行時に 0 か、0 のスロットを指す。静的には、0x3500A0 は rebase で正しく 0x933a2e4 に直っている(reloc の確認)。書き込みは、直接の rip 参照では見つからない(pdata の関数の全件)。
- 次の計測(計測 2): fle_st で 0x93500A0 への書き込みを記録し、DllMain の直後に PEQ2 のイメージの値を表示する。これで、書き込みの有無と、写された値が分かる。build-lock が必要(取得待ち。worker r45d-diag のビルドの最中)。

## 結論(2026-11-17。詳細と出典は evidence/R5-6f.md §7)

- 原因(PEQ2・R5-6d): 呼び先の関数表(0x935F220、PEQ2 の RVA 0x35F220 の .bss)が、ILVideo_x64.dll の読み込み(LoadLibrary "ffmpeg\x64\ilvideo_x64.dll")に成功したときだけ GetProcAddress で埋まる。実行では読み込みが 0 を返し、表が 0 のまま呼ばれる(出典: `evidence/R5-6f.md` §6b。計測 3 の raw の 2295 行(呼び先の値 0)と 2301 行(rax = 0x935F220)。計測 1 の `LoadLibraryW(... ilvideo_x64.dll) -> 0` は trcfull-tail.txt の 3999 行)。0x2155B0 は戻り値を見ない。
- 原因(Soundgoodizer・R5-6c): 共通ではない。停止は 0x8806F80 の thunk(RaiseException)・RtlUnwindEx の着地・Exception on open。ILVideo の読み込みの関数は本体にもあるが、Exception on open との関係は未確認。
- 仮説の判定: H1 外れ(§1)。H2 外れ(§1)。H3 外れ(§1)。H4「ILVideo の失敗は直接の原因でない」は外れた(表は読み込みの成功でしか埋まらない)。H5 は番地が誤り(正しくは 0x3503A0 → 0x935F220)。H6(arena と像の重なり)は外れた(像の範囲に重なり 0 件。evidence §7.4)。
- 決定 1: PEQ2(e5786c48)は main に入れない(保留 0411 の継続)。根拠: 呼び出しの 0 を避ける手段は、ILVideo を実際に読めるようにすること以外に無い。0 を返す・表を埋めた値を置く・呼びを飛ばすのは代役で、進捗にならない(docs/ops/honesty.md)。
- 決定 2: 次の作業は、ILVideo_x64.dll と、その import の FFmpeg 6 本(avutil-56・avformat-58・avcodec-58・swresample-3・swscale-5・avdevice-58。docs/archive/decisions/0206-sample-decoders.md §10)の lift を、新しい R-ID として起こす。(訂正 2026-11-17: 閉包は avfilter-7 を含む 8 本。決定 0423 の決定 1・2 で、規模の判定の後に行う)ID は main が割り当てる。最初の一手は、prune 後の生成 C の大きさを測ること。根拠: 0206 §10 の見積もりは全量で約 770 MB の生成 C(oggio の 35 倍)で、prune が前提。
- 決定 3: Soundgoodizer(R5-6c)は main から外したまま(0409)。Exception on open の原因の候補として ILVideo の読み込みを残す(未確認)。
- 決定 4: 誤った番地を含む記録は、訂正の行を入れた(0411・evidence/R5-6d.md・evidence/R5-6f.md・ROADMAP の R5-6d・R5-6f の行)。是正の一覧は docs/failures/0030。
- 撤回条件: 実機の PEQ2 がこの経路(0x2148B0 の PNG の分岐)を通らないと確かめたら、決定 1・2 を見直す。ILVideo が読めた版で、表の呼び出しが通ることを確かめたら、0411 の再開条件を満たす。
