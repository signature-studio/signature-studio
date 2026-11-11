# 0310(仮)G9-a: Fruity Balance を開くための runtime 側の準備(行なし 74 の import と plugin の実行時の壁)— 読みと設計のみ(alpha1 = alpha22)

作業者 `alpha1`(成果物名 alpha22)、2026-11-07 JST 02:2x。分担(PM 01:2x): Balance を実モジュールとして足す手続き(build の行・基底・フック点・volume/pack・patch)は mt3(0313)。alpha1 は **runtime の未実装 import(0309 の『行なし 74』)の扱い**と、**plugin の実行時の壁(最初の process までの node テスト)の前提**。**この文書は新しい build も実行も含まない**(Elastique の追跡が優先で、G9-a の build は mt3 の bundle ができた後〔PM の info、約 03:10〜〕)。

**実エンジンについて(先頭)**: 本書に実エンジンの実行結果は無い。入力は (a) 0309 の表(`alpha21-results/tables/0309-imports-none.tsv`、fe-lift が lift した Fruity Balance の『行なしの import』)、(b) Win32 の契約の記憶(Learn の記述の要約。未照合の箇所は『読み』)。

## 1. 『行なし 74』の中身(`tables/0310-g9a-missing-import-rows-74.tsv`)

Fruity Balance の import のうち runtime に実装の行が無い物は 74 個(Balance・Filter・Delay 2・PEQ2・Frequency Shifter で同一の集合。FLEngine の lift も同じ集合を抱えており、**実行されなければ停止しない**)。DLL 別: user32 24、comctl32 22(ImageList_*)、imm32 7、kernel32 6(EnumCalendarInfoW・GetCPInfoExW・GlobalDeleteAtom・GlobalSize・HeapDestroy・RtlUnwind)、uxtheme 4、ole32 4、shell32 3、msimg32 1、windowscodecs 1、wtsapi32 1、CRT 1(memset)。到達の見込み(読み): 

| 分類 | 数 | 内容 | plugin の生成〔CreatePlugInstance〕までに実行されるか |
|---|---:|---|---|
| ui-event | 28 | スクロールバー、IsDialogMessage、OLE、shell、FindWindowExW など | 操作・表示まで不要(読み) |
| ui-or-init | 22 | ImageList_*(TImageList) | フォームが画像リストを持つ場合のみ。FL の plugin の UI は自前描画が多く、生成だけなら不要の見込み(未確認) |
| init-or-ui | 8 | タッチ・ジェスチャ・近接・IsProcessDPIAware(delay) | VCL が Touch を有効にした時のみ |
| ui-paint | 6 | AlphaBlend、uxtheme の Buffered paint、WIC | 描画まで不要 |
| init | 3 | memset、EnumCalendarInfoW、GetCPInfoExW | **DllMain / 初期化で実行されうる**(最有力)。FLEngine には同じ行なしがあって実行されていない=ロケールの era を要求しない経路 |
| lifecycle | 3 | HeapDestroy、GlobalDeleteAtom、WTSUnRegisterSessionNotification | DLL の解放・窓の破棄時 |
| exception | 1 | RtlUnwind | Delphi の例外が実際に投げられた時だけ。fle_seh との接続が要る |
| never | 3 | MDI(DefFrameProcW・DefMDIChildProcW・TranslateMDISysAccel) | plugin は MDI を使わない |

各行の契約と実装の方針は表の `contract_and_plan` 列(Win32 の契約の要約。**実装は壁に当たってから**。fidelity 方針: 余計な FL 固有の処理は足さない、外部 API の契約で実装する、新しい import の行は `ship6-imports.txt` の末尾に追記)。

## 2. 最初の反復の見込み(読み、未検証)

0309 の見積りと mt3 の 0313 の手続きで bundle ができたら、node の G3(`tools/open-problems.g3` の形: File → Open → Problems が出る → `dump`)で次を観測する: (1) `LoadLibraryW`(フルパス)が実モジュールを返すか(mt3 の診断 diff で `g9-diag: LoadLibraryW … -> 0x6800000`)、(2) `GetProcAddress("CreatePlugInstance")`、(3) `fle: unsupported`・`pruned`・`onAbort` が 0 行、(4) Problems 窓の件数が 7 → 6。最初の壁は **init の 3 行(memset・EnumCalendarInfoW・GetCPInfoExW)か、フックの無い経路(フック点の漏れ)か、plugin の ホスト vtable(TFruityPlugHost)の未実装の呼び**のどれか(読み)。壁ごとの反復は 12〜17 分(build 10〜13 + node 2〜3)。
**準備済み**: 私の tree(`/home/ubuntu/work/alpha22w/tree`、a977401d + Balance 用の編集)・x20 のコピー(`prodwork`)・Balance の stub 行入りの seed(`seed-g3.tsv`)・`tools/open-problems.g3`(0310 の初回作業の残り。mt3 の 0313 の patch と重複する部分は mt3 を正とする)。

## 2b. mt3 の 0313 との分担(重複しない)と、最初に当たる壁の契約の設計

- **mt3 の 0313(`mt3w-results/0313/docs/0313-g9a-balance-module.md` §2)が既に持つ物**: 74 個の出所と内訳、製品の行で lift した時の `fle_unsupported` の thunk 44 個(残り 30 個は thunk が lift 対象外)、ランタイムの扱い(呼ばれたら VA つきで `abort`。黙って 0 を返す既定の応答は無い)、lift 済み C の直接 call だけを辿った下限(`CreatePlugInstance` から 749 関数に到達し、行なしの thunk は **`ScrollWindow`(0x41e7f0)と `GetClassLongPtrW`(0x41eda0)の 2 つ**、行ありの `fle_imp_*` は 91 個)、『事前に 74 個を一括で足さない』方針。本書の表(`0310-g9a-missing-import-rows-74.tsv`)は**それに足す物**として、各行の契約の要約と実装の方針だけを持つ(到達の分類は読み)。
- **最初の壁の見込み(mt3 の下限に従う)**: `ScrollWindow` と `GetClassLongPtrW`。設計(Win32 の契約、未照合の箇所は読み):
  - **`GetClassLongPtrW(hWnd, nIndex)`**: ウィンドウが属するクラスの WNDCLASSEX の値を返す。nIndex は GCW_ATOM(−32)・GCL_CBCLSEXTRA(−20)・GCL_CBWNDEXTRA(−18)・GCL_HBRBACKGROUND(−10)・GCL_HCURSOR(−12)・GCL_HICON(−14)・GCL_HICONSM(−34)・GCL_HMODULE(−16)・GCL_MENUNAME(−8)・GCL_STYLE(−26)・GCL_WNDPROC(−24)、または 0 以上のクラス追加バイトへのオフセット。失敗は 0 + GetLastError(窓ハンドル不正〔ERROR_INVALID_WINDOW_HANDLE〕、nIndex 不正〔ERROR_INVALID_INDEX〕)。runtime に `SetClassLongPtrW` の行があるなら窓のクラスの記録は既にあるので、**同じ記録を読むだけ**(新しい状態は要らない)。
  - **`ScrollWindow(hWnd, XAmount, YAmount, lpRect, lpClipRect)`**: lpRect が NULL なら窓のクライアント領域全体を、そうでなければ lpRect の内側を (XAmount, YAmount) だけずらす(lpClipRect があればそれで切る)。ずらして露出した領域は**更新領域に加わり、後で WM_PAINT が来る**(その場では再描画しない)。lpRect が NULL の時は**子窓も同じ量だけ動く**。戻り値は成功で非 0。`ScrollWindowEx(…, SW_INVALIDATE | SW_SCROLLCHILDREN〔lpRect が NULL の時〕)` に帰着できるので、runtime に `ScrollWindowEx` があればその上に薄く、無ければ面の移動(同じ面の内部コピー)+ 更新領域の加算 + 子窓の位置の移動で足す。
- 次の壁が『行ありの API の引数の形の停止』なら、それは別の層(その実装の契約の拡張)。名前は `tools/thunk-names.py` か `tables/balance-unsupported-44-rebased.tsv`(mt3)で引く。

## 3. mt3 への回答(V2b の前提、01:35 に info 済み)

seed は実行時に渡るので bundle の再 build 無しで volume を変えられる(ただし stub の行は `LoadLibraryW` で停止する)、`dump <tag>` が全窓の面を `FLE_G8_DUMPDIR` に `<tag>-lid<id>-<w>x<h>.bgrx` で書く(Problems 窓の件数は画素の中)、`fle:` 行が 0 であることを毎回確認、など。

## 4. V2b(node の G3)の前提(03:5x、実エンジン〔現行の製品 bundle fl-engine-ci-0363b050 = E13〕を node で実際に走らせた実測)

- **製品の wasm は診断 env を読まない**(`lift/runtime/fle_test_env.h`、決定 0243): E13 に G3 の script と `FLE_FILEDLG_PICK` を渡して node で走らせると、エンジンは起動し最初の描画〔lid 8、334×476〕まで行くが、**script は実行されず**(`g3-snap` 0 行)約 4 秒で終わる。
- **製品の bundle は `-sFILESYSTEM=0`**(glue に FS が無い): 既存の node 駆動(`lift/harness/mt2/node-bundle-run-sh.mjs`、私の `node-run.mjs`)は `opts.FS.mkdirTree` で落ちる(`Cannot read properties of undefined (reading 'mkdirTree')`)。`dump` も書き込みが要る。新形式の bundle の駆動は `app/src/engine/hostfiles.ts` の `HostFiles`(read-only)で可能(`tools/node-run-hf.mjs`、`docs/investigations/memfs1/tools/trace-hostio.mjs` が先例)だが、診断 env を読む TEST build でないと script は走らない。
- **正しい道 = wasm TEST build**(`lift/harness/parity/make-test-tree.sh PRODUCT_TREE PRODUCT_WORK TEST_TREE TEST_WORK`): 製品の木の複製で `fle_test_env.h` を getenv 版に置換、`-sFORCE_FILESYSTEM=1` と `FS` の export、そのヘッダを include する unit だけ再コンパイル、link と hybrid merge をやり直す。mt3 の冷間 build(製品の構成 + 診断 diff、`tree-g9a`・`ci-g9a/engine`)の後に `tools/g9a-test-build-a1.sh prep|build`(準備済み・未実行)で 12〜17 分(読み)。V2b はその TEST bundle で `tools/v2b-run.sh BUNDLE NAME`(`dump` で Problems 窓の面を出す)、V3(Chrome)は製品形式の bundle。
- seed = `/home/ubuntu/work/alpha22w/seed-nested-balance.tsv`(mt3 の `mk-nested-balance-seed.sh` で作成、289 行 = nested + Balance の 1 行)。

## 5. V2b・G9-b の node 実測(04:5x、実エンジン〔lift 済み FL + 実モジュール Balance、wasm TEST bundle `fl-engine-test-g9a`〕を node で実際に走らせた実測。代役・再生なし)

- **V2b**(`tools/v2b-run.sh`、seed = nested + Balance 行): `LoadLibraryW "…\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`、`GetProcAddress(0x6800000,"CreatePlugInstance") -> 0x6abdde0`、`fle:` 行は act1-net の `InternetOpenUrlW` 2 行 + 通常の `MessageLoop: idle` のみ(unsupported・abort 無し)。Problems 窓は対照(E12c TEST bundle = Balance 無し)の「8 plugin(s)」691x343 から G9-a の「7 plugin(s)」691x327 に減った(Balance が 1 件、読み込めたと FL が見た、と読める。件数の差は実測、理由は推測)。表と PNG は `tables/0310-v2b-g9a-vs-control.txt`、`tables/v2b-problems-*.png`。
- **G9-b**(Export、`tools/g3-run-nested.sh` + `export-plain.g3` / `export-g9b.g3`): 2 回とも約 151 秒で `done` まで到達。`fle:` は act1-net の 2 行 + diag 行のみ、unsupported・abort 無し。**WAV は取り出せていない**(下記)。
- **`vfsdump` が出力しない理由**: main の `lift/harness/fruity_g3.c` では `vfsdump` が `#if !defined(__EMSCRIPTEN__)`(517–545 行)の中にあり**ネイティブ限定**。wasm の TEST bundle ではその op は未知の op として読み飛ばされる。0308 で wasm から WAV を取り出した `vfsdumpdir` は私の TEST 専用 patch で、mt3 の tree(main 基点)には無い。
- **再基点した TEST 専用 diff**: `tools/0310-g9b-vfsdumpdir.test-only.diff`(`fle_vfs.c` に `fle_vfs_walk_dir`、`fle_vfs.h` に宣言、`fruity_g3.c` に `vfsdumpdir GUESTDIR` を `#if` の外に追加。g9a-test-tree の原本のコピーに対し `patch -p1 --dry-run` 成功)。製品の木には当てない(push しない)。スクリプトは `tools/export-g9b-v2.g3`(末尾が `vfsdumpdir C:\Users\user\Downloads`)。
- **FLP の読み**(`serina1-serum2.flp` のイベント列を読んだ): Fruity Balance は **Insert 1 の slot 1**(event 212 の先頭 2 DWORD = insert 1 / slot 0)に 1 つだけ(event 212 の構造は FL の公開仕様を確認していない読み。未検証)。
- **Balance が呼ばれたことの証拠の取り方**: (A) mt3 の `diag-g9-count.diff`(`fle_callback.c` のみ。g9a-test-tree に `patch --dry-run` 成功)で、image 枠への間接呼び出しの数を stderr に出す。(B) 再 build 無しの UI 証拠: node の G3 でミキサー(F9)を開き Insert 1 の slot 1 を開いて Balance の窓の面を `dump`(反復が要る)。(C) WAV の一致は呼び出しの証拠にならない: 0308 で Wine の原本(Balance あり)と wasm(Balance 無し)の Export が rms 0.0(CPUID 揃え後、1 LSB の差 4/11.4M)だったので、この project の Balance は出力に対して透過。G9-b の WAV は『壊していない』の回帰確認。

## 6. G9-b の事実(Chrome の Export)と、Balance の automation の表(05:1x。読みと CPU 軽い numpy のみ。node/Chrome/Wine の Export は走らせていない)

**先頭**: 次の数字は、実エンジン(lift 済み FL + 実モジュール Balance、mt3 の test bundle `fl-engine-test-g9a-28f6f631`)を ci1 が実 Chrome 156 で走らせた Export の WAV(`lift/out-ci1/kit/out/export-g9a/export.wav`)と、基準(製品 E13 + P+V7 の page の Export、`…/export/export.wav`)を **私が numpy で読んだ実測**。代役・再生・推測した出力は含まない。Export を走らせたのは ci1(出典 `ci1-results/0277/e14/g9a-export/RESULTS-g9a-export.md`)。sha256 は私も `sha256sum` で確認した(g9a `94c93666…4090`、基準 `381cdb5a…f979`)。

### 6.1 事実(実測)
- 完走(ci1: 310 s、Start→WAV 約 74 s)、`fle: unsupported` 0 行、`g9-diag` 2 行(`LoadLibraryW … -> 0x6800000`、`GetProcAddress(…,"CreatePlugInstance") -> 0x6abdde0`)。
- WAV: 22,863,828 B・5,715,911 フレーム(基準と同じ)、**sha が違う**。差のあるフレーム 640,995(11.21%)、同一のフレーム 5,074,916(88.79%)。最初の差 25.79 s、最後 119.78 s。
- 私の node の G9-b(自前の TEST bundle `fl-engine-test-g9a`。ci1 の `…-28f6f631` とは別の build)は、vfsdump が wasm で未知の op のため WAV を取れなかった(§5)。Chrome の結果が G9-b の WAV 側の事実。

### 6.2 参照プロジェクトの Balance の automation(FLP の event 234、チャンネル iid 5。表 = `tables/0310-g9a-balance-automation-vs-diff.txt`)
- 24 バイトの記録 × 7 点(21 バイト目から。float64 の d0、float64 の値、float32 の tension、u32 のフラグ。最後の点だけフラグ 0x02000000 = 終端、と読む)。
- d0 の累積: 0、0、0.13932、0.20180、0.44900、**0.50000(ちょうど)**、4.55208。d0 は『前の点からの差』と読む(累積が 0.5 ちょうどになるため。単位は未確認)。
- 値: P0 0(ダミー)、P1 0.8、P2 0.8、**P3 0.0**、P4 0.8、P5 0.8、P6 0.8。tension: −1、−0.504、−1、−0.403、−0.638。0.8 は Balance の既定(Volume 0..320 の 256 = 100% = 0.8)と読む(未確認、FL SDK の一般知識)。= **各クリップは、既定の 0.8 のまま保ち、P2→P3 で 0 まで落ち、P3→P4 で 0.8 に戻る形**(0 = 無音の可能性。法則は未確認)。
- プレイリストに 256 個(長さ 45 tick × 123、36 tick × 132、19 tick × 1。start/end offset は FLP の正確な値でそれぞれ 0〜45 tick、7〜43 tick、0〜19 tick〔拍で 0〜0.46875、0.0729〜0.4479、0〜0.1979。**`playlist_items.tsv` は 0.1 刻みに丸めており、『0.0–0.5』『0.1–0.4』『0.0–0.2』は丸めた値**。§10 の訂正〕)。最初の開始 25.741 s、最後の終端 119.781 s(テンポ 200 BPM・PPQ 96 の読み。時刻は WAV の差の範囲と一致)。クリップの無い区間: **0〜25.74 s、42.98〜45.56 s(2.6 s)、81.53〜103.14 s(21.6 s)、119.78 s 以降**。

### 6.3 差の区間は automation のクリップと一致する(実測)
- 差のあるフレーム 640,995 のうち **626,143 はクリップの内側**、残りの **14,852 はすべて『直前のクリップの終端から 0〜137 フレーム(最大 3.11 ms)後』**(クリップの終端の尾。クリップ間の空きは最小 414 フレーム、次のクリップの開始まで 1,517 フレーム以上)。クリップの無い区間に差は 0。
- 10 s ごとの帯(クリップが占める割合 → 差のあるフレームの割合): 20–30 s 21.7%→9.7%、30–40 s 50.6%→22.9%、40–50 s 33.5%→15.3%、50–60 s 40.9%→18.9%、60–70 s 40.1%→17.8%、70–80 s 41.8%→18.2%、80–90 s 7.8%→3.5%、**90–100 s 0%→0%(クリップが無いのは 81.5〜103.1 s の空き。90–100 s が差 0 なのは帯の切り方の偶然ではなく空きの内側)**、100–110 s 35.2%→15.9%、110–120 s 50.6%→23.2%。**クリップの内側で差のあるフレームの割合は全帯で 41〜46%**(一定)。
- = **差が出るのは Balance の automation が動いている所だけ**(読み: Balance の実モジュールが呼ばれ、automation の値を受けて出力が変わった)。

### 6.4 クリップの内側の立ち上がりは決定的(実測)
- 差の最初のフレーム(クリップの開始から): 45 tick の 123 個のうち **119 個が 2205〜2209**(中央値 2207)、36 tick の 132 個のうち **128 個が 1240〜1244**(中央値 1242)、19 tick の 1 個が 2208。例外は 4+4 個(音が無い所の後ろで 1245/1654 などへ遅れる。`tables/0310-g9a-balance-clips.tsv`)。最後の差は 45 tick で 6062(クリップ 6202 フレーム中)。
- 開始 offset が 0.0 のもの(45 tick と 19 tick)は **クリップの長さが違っても立ち上がりが同じ(2207/2208)** = automation はクリップの長さに伸縮されず、自然な速さで開始位置から再生されている(読み)。

### 6.5 クリップ揃えの平均(45 tick の 123 個、256 フレームの窓、両側が clip していない標本のみ、r = Σ⟨b,x⟩/Σ⟨b,b⟩)
- 実測: 1.000(〜1792 フレーム)→ 0.953(2304)→ 0.78(2560)→ 0.46(2816)→ **最小 0.39(3328)** → 0.58、0.61、0.51、0.55、0.59、0.71、0.74、0.75、0.86(5632)→ **0.997(5888)** → 1.000(6144〜)。36 tick の 132 個は 1.000(〜1024)→ 0.70(1536、最小 0.685)→ 0.72〜0.86 → 0.93(4608)→ 1.000(4864〜)。
- 形は『0.8 のまま → P2→P3 の速い落ち込み → P3→P4 の遅い立ち上がり(tension −1)→ 0.8』と定性的に合う。**(訂正 05:5x、§10: 初版の『時間軸は合っていない』『単位は未解決』は、`playlist_items.tsv` の丸めた offset を使った私の誤りだった。正確な offset 〔0〜45 tick、7〜43 tick、0〜19 tick〕では単位 = 拍〔1 拍 = 96 tick〕で、落ち込みの開始は P2 から一定の 364 フレーム遅れ〔クラス A 2207 − 1843.1 = 363.9、クラス B 1242 − 878.5 = 363.5〕と 2 クラスで一致する。FLP の線形補間〔tension 未適用〕との相関は、クラス B 0.867・クラス A 0.769。初版の 0.74・0.558 は誤った offset による〕**
- r は『Insert 1 の寄与の割合 s を含む比』で利得そのものではない: r = 1 − s(1 − g) と読むと、最小 r = 0.39 は g = 0(P3 の値 0)かつ s ≈ 0.61 と整合する(読み。s は未測定)。期待利得『0.93〜0.99』は、私の測定からは再現できなかった(この r の範囲は 0.39〜1.0。出所を確認したい)。

### 6.6 clip(±32767)と符号の反転 — 差の最大 65,534 は異常ではない(実測 + 読み)
- 基準の WAV は **30.83% のフレームに ±32767 の標本がある**(1,762,122 フレーム。g9a は 29.64%)。差のあるフレームのうち基準が clip していたのは 273,779、g9a は 205,568。符号が反対の標本は 143,316 個(うち両側が |20000| 超は 3,796 個、|差| > 32768 は 31,585 個)。= 16 ビットの整数化で clip している WAV では、Insert 1 の利得が下がると clip していた標本が『大きな値 → 小さな逆符号の値』に変わりうる(読み: FL のミキサーは内部 float で、clip は最後の整数化で起きる、は未確認)。clip していない標本だけの差: 328,774 フレームで、|差| の中央値 3,663・p90 18,703・p99 36,802、基準に対する相対 |差|/|b| の中央値 0.317。
- よって **この WAV どうしの rms や最大差は、利得の正しさの尺度にならない**。

### 6.7 faithful の判定の案(未実行。窓の後)
1. **Wine『Balance のみ有効』(opt1 の elog15)との比較**: 同じ道具(`a22-balance-diff.py` / `a22-balance-shape.py`、引数の WAV を替えるだけ)で、(a) 同一フレームの割合と、不一致の位置(クリップの内側か外側か)、(b) clip していない標本の |差|(CPUID の AVX/SSE2 差は 0308 で 1 LSB の程度)、(c) 6.4 の立ち上がり(2207/1242)と 6.5 のクリップ揃えの平均の一致、を出す。(a) が全一致なら完了。
2. **32 ビット float の Export**(Render ダイアログの bit depth。あれば): 基準と G9-a と Wine の 3 本を float で書けば、clip が消え、差は Insert 1 の利得に対して線形に近くなる(読み。ダイアログに float があるかは未確認)。
3. **Split mixer tracks の Export**(0308 の道具): Insert 1 の stem どうしの比 = Balance の利得 g(t) を、master の合算なしで直接得る(Insert 1 の中の後段エフェクトが線形であれば。未確認)。

## 7. Wine『Balance のみ有効』の WAV(opt1 の elog15b)が出たら走らせる比較(準備のみ。05:2x)

- **道具**: `tools/a22-balance-compare.py WINE.wav G9A.wav [--base BASE.wav] [--eps N] [--csv-prefix P]`(読み専用、numpy 1 スレッド。G9A の既定 = `lift/out-ci1/kit/out/export-g9a/export.wav`〔sha 94c93666…〕、BASE の既定 = `…/export/export.wav`〔Balance なしの wasm、sha 381cdb5a…〕。FLP は `~/ダウンロード/serina1-serum2.zip` から、クリップは `proj1-results/tables/playlist_items.tsv` から読む。WAV は PCM 16/24/32 と float32 を読み、16 ビット相当に揃える)。
- **見出しは固定**: 0 入力(frames・形式・sha256)/ 1 全体(組 = WINE-G9A〔核〕・BASE-WINE・BASE-G9A の、差のフレーム数・同一フレーム・全標本の RMS・最大差)/ 2 10 s 別(差の割合と RMS を 3 組)/ 3 クリップの内側・尾・それ以外 / 4 クリップ終了後の尾(直前のクリップの終端からのフレーム数)/ 5 立ち上がり(クラスごとの最初の差のフレーム)/ 6 クラスごとのクリップ揃えの平均 r(t)(r_WINE と r_G9A を並べる)/ 7 Wine と G9-a の形の相関(クラス平均の相関・最大差・クリップごとの相関の中央値と RMS の中央値、FLP の形との相関)/ 8 clip していない標本だけの差 / 9 判定の案(J1〜J5)。
- **判定の案(決めるのは PM)**: J1 全フレーム一致(核。PASS なら faithful)/ J2 尾の外に差が無い / J3 clip していない標本の最大差が 2 LSB 以下 / J4 立ち上がりの中央値の差が 2 フレーム以内 / J5 クラス平均の r の最大差が 0.01 以下。J1 が FAIL でも J2〜J5 が PASS で J1 の差が 2 LSB 級なら『CPUID 級の差のみ』(`--eps 2` で J2・J4 を再計算)。J2・J4・J5 が FAIL なら Balance の実装(または automation の受け方)の差。
- **自己テスト**(実測。Wine の結果ではない): WINE の位置に (a) G9-a のコピーを渡す → J1〜J5 すべて PASS、(b) G9-a に 1/10,000 の標本へ ±1 LSB の雑音を足した物を渡す → J1 FAIL、`--eps 2` で J2〜J5 PASS、(c) 基準(Balance なし)を渡す → J1・J3・J4・J5 FAIL。出力の見本 = `tables/0310-compare-SELFTEST-standin-not-wine.txt`(先頭に『Wine の結果ではない』と書いてある)。
- 旧 `a22-balance-diff.py` / `a22-balance-shape.py` はこの script に統合済み(結果の再現用に残す)。

## 8. G9-c 用: 残り 6 種の『行なし import』の増分(Balance の 74 に無い物だけ。05:3x、読みと CPU 軽い python のみ)

**先頭**: PE の import 表(pefile)と、現行の行の集合(`imports_table.rs` の TABLE 609 + `ship6-imports.txt` = 819。0309 の表は ship5 だった)を突き合わせた**静的な実測**。実行・lift・build はしていない。『reach class』は API の用途からの**読み**で、実行して踏んだ観測ではない。表 = `tables/0310-g9c-extra-imports-by-plugin.tsv`(増分の 1 行ずつ)、`tables/0310-g9c-extra-imports-summary.tsv`(種別ごとの集計と追加順の累計)、道具 `tools/a22-g9c-extra-imports.py`。

| 種別(DLL) | import | 行なし | うち Balance の 74 | **増分** | 増分の DLL 別 | 増分の分類(ui / init / process / まず踏まない) |
|---|---:|---:|---:|---:|---|---|
| Fruity Filter | 543 | 74 | 74 | **0**(集合が Balance と同一) | - | - |
| Fruity Delay 2 | 543 | 74 | 74 | **0**(同一) | - | - |
| Fruity Parametric EQ 2 | 548 | 74 | 74 | **0**(同一) | - | - |
| Frequency Shifter | 548 | 74 | 74 | **0**(同一) | - | - |
| 　└ Engine_x64.dll(補助) | 73 | **0** | 0 | 0 | - | - |
| Soundgoodizer | 638 | 107 | 74 | **33**(すべて FLEngine も import) | gdi32 9、kernel32 8、winspool.drv 5、ole32 3、shell32 2、comdlg32 2、user32 2、imagehlp 1、comctl32 1 | ui 16 / init 3 / process 0 / まず踏まない 14(印刷 12〔gdi32 の StartDoc 系 7 + winspool 5〕、例外 2) |
| FLEX | 618 | 88 | 74 | **14**(すべて FLEngine も import。うち 7 は Soundgoodizer と共通) | ole32 4、gdi32 3、comdlg32 2、shell32 1、kernel32 1、user32 1、comctl32 1、advapi32 1 | ui 7 / init 1 / process 0 / まず踏まない 6(メタファイル・OLE 埋め込み) |
| 　└ engine_x64.dll(補助、`.flexpack`) | 338 | 107 | 5 | **102**(FLEngine も import 7、**95 は FLEngine にも無い**) | opengl32 36、user32 27、kernel32 12、shell32 8、dbghelp 6、gdi32 4、shlwapi 3、comdlg32 2、winmm 2、version 1、d3d9 1 | ui 81(GL 描画 43 を含む)/ init 10 / process 4 / まず踏まない 7 |
| 　└ engineV2_x64.dll(補助、`.flex2pack`) | 342 | 109 | 6 | **103**(FLEngine も import 8、95 は FLEngine にも無い) | 同上 + advapi32 1 | ui 81 / init 9 / process 4 / まず踏まない 9 |

- **追加順の累計**(Balance → Filter → Delay 2 → PEQ2 → Frequency Shifter → Soundgoodizer → FLEX): 74 → 74 → 74 → 74 → 74 → **107**(+33)→ **114**(+7、プラグイン本体のみ)/ 補助 DLL まで入れると **213**(+106、うち FLEngine に無い物 97)。= **Filter・Delay 2・PEQ2・Frequency Shifter は Balance の 74 で足りる**(新しい壁の種類は増えない。壁を一つずつ潰す作業は Balance で済む分が再利用できる)。
- **Soundgoodizer の +33 の中身**(読み): 印刷(VCL TPrinter: StartDoc/EndDoc/StartPage/EndPage/SetAbortProc/CreateDCW/CreateICW + winspool 5)12 は plugin が印刷しないのでまず踏まない。残りは Open/Save ダイアログ、`TextOutW`・`EnumFontsW`(VCL canvas)、`SHFileOperationW`・`SHGetFileInfoW`、`CreateProcessW` 系、`OleGetClipboard`、`AttachThreadInput`・`GetDlgItem`(いずれも操作時)。`AddVectoredExceptionHandler`・`ImageDirectoryEntryToData` は Delphi RTL の例外まわり(例外が実際に投げられた時だけ。`RtlUnwind`〔74 側〕と同じ層)。
- **FLEX の補助 engine DLL**は別格: OpenGL(36)・Direct3D 9・dbghelp・winmm のタイマ(`timeSetEvent`)・`WaitForSingleObjectEx`・`SetThreadAffinityMask`(process)を持つ。FLEX_x64.dll の文字列表に `.flexpack`・`.flex2pack`・`Engine_x64.dll`・`EngineV2_x64.dll` がこの順で並ぶ(UTF-16)。パックの種類(.flexpack / .flex2pack)で読む DLL が決まる、と読む(**対応は読み。生成までに読まれるか、どの時点か、は未確認**)。先に FLEX 本体だけ(+7)で止めるか、補助まで入れるか(+106)は G9-c の設計(mt3)の判断。
- **mt3 の静的な下限との関係**: Filter・Delay 2 は Balance と同じ 2 か所(`ScrollWindow` 0x41e7f0・`GetClassLongPtrW` 0x41eda0)が `CreatePlugInstance` から直接に届く(mt3 の `tables/g9c/*-reach-unsup.txt`)。Soundgoodizer・FLEX・PEQ2・Frequency Shifter の同じ解析は未実施(この表は import の集合の差だけ。到達の解析は fe-lift を回す必要があり、窓の後)。

## 9. Wine『Balance のみ有効』(opt1 の elog15b)対 wasm G9-a(Chrome の Export)— `a22-balance-compare.py` の結果(05:5x)

**先頭**: 実エンジン 2 つの Export の WAV を numpy で読んだ実測。WINE = opt1 の elog15b(**original FL を Wine で走らせ、Fruity Balance だけを有効にした Export**、mp3wav-all、SSE2 cpuid、`~/work/opt1-wine/exports/runs/elog15b.wav`、22,863,828 B、sha256 `1cad2739…d7ae`)。G9A = ci1 の Chrome の Export(実エンジン + 実モジュール Balance の wasm test bundle、sha `94c93666…4090`)。BASE = Balance なしの wasm(`381cdb5a…f979`)。代役・再生なし。走らせたのは読み専用の python のみ(Wine・Chrome・Node は走らせていない)。出力の全文 = `tables/0310-compare-elog15b-vs-g9a-eps0.txt`(既定)と `…-eps2.txt`(`--eps 2`)、差の 5 フレーム = `tables/0310-compare-elog15b-diff-frames.txt`。

### 9.1 結論
- **WINE と G9A は 5,715,911 フレーム中 5 フレームだけが ±1 LSB 違う**(5 標本、RMS 0.0007 LSB、最大差 1)。基準(Balance なし)から値が変わった標本は WINE で 1,120,679、G9A で 1,120,675。**両方で変わった 1,120,675 標本のうち 1,120,674 で WINE = G9A**(違うのは 1 標本、1 LSB)。WINE だけが変わった 4 標本は、Balance に関係しない Wine 側の残差(§9.3)で、G9A は基準と同値。G9A だけが変わった標本は 0。
- **立ち上がりとクラス平均の r(t) は Wine と wasm で一致**(r_WINE − r_G9A の最大 0.00000、クリップごとの相関の中央値 1.00000)。= 6.4〜6.5 の読み(『Balance が automation を受けて出力を変える』)を、original FL(Wine)の実測が裏づけた。
- 判定の案: **J1 FAIL(5 フレーム)/ J2 FAIL(既定)→ `--eps 2` で PASS / J3・J4・J5 PASS**。= 『CPUID 級の差のみ』(案の基準。決めるのは PM)。

### 9.2 見出し順
- **0 入力**: 3 本とも 5,715,911 フレーム・PCM16・44.1 kHz。
- **1 全体**: WINE-G9A 差のフレーム **5**(0.0001%)、RMS 0.0007、最大差 1。BASE-WINE 640,999 フレーム(RMS 3,986.6)、BASE-G9A 640,995(RMS 3,986.6)。遅れの検査(±4096 フレーム)は 0、相関 1.0。
- **2 10 s 別**: WINE-G9A は全帯で 0.00%。BASE-WINE と BASE-G9A は同じ帯・同じ割合(例: 30–40 s 22.90%、RMS 6801.92 が両者で一致)。
- **3 クリップの内外**: WINE-G9A の 5 フレームは クリップの内側 1、尾 0、それ以外 4。BASE-WINE は 内側 626,143・尾 14,852・それ以外 4(BASE-G9A は 626,143・14,852・0)。
- **4 尾**: BASE-WINE の尾は 0〜137 フレーム(最大 3.11 ms)で BASE-G9A と同じ。尾の外の 4 フレームは §9.3。
- **5 立ち上がり**: 45 tick(123 個)は BASE-WINE も BASE-G9A も p10/p50/p90 = **2206/2207/2208**、36 tick(132 個)は **1241/1242/1243**、19 tick(1 個)は 2208。WINE-G9A の差が出るクリップは 45 tick の 1 個だけ(§9.3)。`--eps 2` では 1243/1244/1247 と 2207/2257/2258 で、これも Wine と wasm が同一。
- **6 クラス平均の r(t)**: 窓 256 フレームの全 28+22 窓で r_WINE = r_G9A(差 0.0000)。
- **7 相関**: クラス平均の相関 1.00000、最大差 0.00000、クリップごとの相関の中央値 1.00000・RMS の中央値 0.00000(132 個と 123 個すべて)。
- **8 clip していない標本**: 8,792,172 標本中、差 5(実は 5 標本とも clip していない)、最大差 1。
- **9 判定(案)**: J1 FAIL(5 フレーム)、J2 FAIL〔既定の eps 0〕→ PASS〔`--eps 2`〕、J3 PASS(最大 1 LSB)、J4 PASS、J5 PASS。

### 9.3 差の 5 フレーム(`tools/a22-balance-diffframes.py`。`--also elog14.wav` = Wine で Balance を隠した Export)
| フレーム | 時刻 | ch | WINE | G9A | BASE | elog14(Wine、Balance なし) | 位置 |
|---:|---:|---:|---:|---:|---:|---:|---|
| 1,891,929 | 42.901 s | L | −30008 | −30009 | −25324 | −25324 | **クリップの内側**(#62、開始から 2,657 フレーム後 = 落ち込みの中。Balance が基準から約 −4,684 動かした標本で、WINE と G9A が 1 LSB 違う) |
| 3,709,012 | 84.105 s | L | 18772 | 18771 | 18771 | 18771 | 尾の外(#194 の終端 81.5 s の 2.6 s 後、次のクリップ 103.1 s の 19 s 前)|
| 5,333,434 | 120.940 s | R | −12841 | −12842 | −12842 | −12841 | 最後のクリップ(119.78 s)の後 1.16 s |
| 5,333,941 | 120.951 s | L | −19488 | −19489 | −19489 | −19488 | 同 1.17 s |
| 5,422,135 | 122.951 s | L | 24609 | 24608 | 24608 | 24609 | 同 3.17 s |

- **後ろの 3 つ(120.9 s 以降)は Balance に関係しない Wine 側の残差**: elog14(Wine、Balance を隠した Export)でも同じ 3 フレームが同じ値で BASE と 1 LSB 違う(elog14 対 BASE の差の 4 フレーム = 495,392〔11.2 s〕+ この 3 つ。0308 の『Wine と wasm の残差 4 / 11.4M 標本』と同じ種類)。曲の後の残響の尾の領域(読み)。
- **84.105 s の 1 つ**: elog15b だけに出た(elog14 では BASE と同値)。automation が動いていない所(最寄りのクリップの終端から 113,760 フレーム後)なので、Balance の automation の受け方の差ではなく、run ごとに位置が変わる Wine 側の残差と同じ種類と読む(elog14 の 11.2 s の 1 フレームは elog15b には無い = 位置は run ごとに違う)。
- **クリップの内側の 1 つ(42.901 s)**: 両方で変わった 1,120,675 標本のうち 1 標本だけが 1 LSB 違う。5 フレームがクリップの内側に入る確率は、クリップが全体の 24.8%(1,420,296 / 5,715,911 フレーム)なので期待 1.24 個 = 5 個中 1 個は偶然の範囲(読み)。
- **未確認**: これらの 1 LSB の原因(Wine の残差の正体は 0308 でも未解決。CPUID・スレッド・浮動小数の丸めのどれか、は特定していない)。G9-a の Balance の実装が原因でない、とは言い切れないが、WINE 側にも同種の残差が Balance なしで出る(elog14)ので、Balance 固有の差の証拠は無い。

### 9.4 『貸し借り』
- 6.4〜6.5 の読み(『差が出るのは automation の動く所だけ』『立ち上がり 2207/1242』)は Wine 側でも同じ値で再現された。6.5 の『r は利得でなく Insert 1 の寄与の割合を含む』の読みは未検証のまま(r の一致は『利得の一致』を意味しない可能性は残るが、WAV が 5,715,906 / 5,715,911 フレームで一致するので、利得も実質的に一致している。= 両者の出力が ±1 LSB で同じ)。opt1 も独立に同じ 5 標本(位置・符号とも一致)を確認している〔opt1 の 05:55 の report〕。
- 前報の『faithful の判定は WINE との比較で』は、この結果で **『この Export の範囲で、wasm の Balance は original FL(Wine)の Balance と ±1 LSB(5 標本)で同じ』**。範囲外(未確認): 他のプロジェクト・他の Export 形式・再生時の挙動・CPUID の AVX 経路(elog15b は SSE2 cpuid)。

## 10. 訂正(offset の単位)と Fruity Filter の automation(insert 47)の FLP の表(05:5x、読みのみ)

**訂正**: §6.2・§6.5 の初版は、`proj1-results/tables/playlist_items.tsv` の start/end offset(0.1 刻みに丸められた値)を使い、『offset は 0.0–0.5、0.1–0.4、0.0–0.2』『時間軸が合わない』『単位は未解決(1 単位 9,650 フレーム)』と書いた。**これは私の誤り**。FLP の playlist 項目から正確な値を読むと、offset は **拍(1 拍 = 96 tick)** で、クラス別に 0〜45 tick(0〜0.46875 拍)、**7〜43 tick**(0.0729〜0.4479 拍)、0〜19 tick(0〜0.1979 拍)。裏づけ 2 つ(実測): (1) Filter の automation clip の長さ 3444 tick = end_off 35.875 拍 × 96(各クラスでも end − start = 長さ tick)、(2) 落ち込みの開始(最初の差のフレーム)は、P2(0.1393 拍 = 13.375 tick)から**一定の 364 フレーム遅れ**: クラス A(開始 0 tick)は 2207 − 1843.1 = 363.9、クラス B(開始 7 tick)は 1242 − 878.5 = 363.5。2 クラスで一致する = 時間軸(単位・開始位置)は合っている。FLP の線形補間〔tension 未適用〕との相関は、クラス B 0.867・クラス A 0.769(初版の 0.558・0.744 は誤った offset による)。`tools/a22-balance-compare.py` は FLP から正確な offset を読む版に直した(見出しは同じ)。`tables/0310-compare-elog15b-*.txt` と `0310-compare-SELFTEST-…` は直した版で再生成した(§9 の数字は変わらない)。`tables/0310-g9a-balance-automation-vs-diff.txt`(旧版)の先頭に訂正の注記を足した。

**Fruity Filter の automation(`Filter - Insert 47 - Cutoff frequency`、iid 62)** — `tools/a22-automation-table.py`、表 = `tables/0310-automation-tables-balance-filter.txt`(Balance〔iid 5〕と同じ形)。
- 対象: Fruity Filter(native)は **Insert 47 の slot 1 に 1 つだけ**(event 212 の先頭 2 DWORD = insert 47 / slot 0。Balance の 1/0 と同じ構造)。Insert 47 の名前は「Pattern 7 (consolidated)」、出力は **Insert 1**(Balance の insert。47 → 1 → 0)。
- 点(4 点、単位 = 拍): P0 0 拍 値 1.000(ダミー)、P1 4.0 拍(384 tick = 1.2 s)値 0.3553 tension −0.371、P2 35.875 拍(3444 tick = 10.76 s)値 **0.000** tension +0.037、P3 90.25 拍(8664 tick)値 1.000(終端)。= cutoff を 1.0 から 1.2 s で 0.36 へ、続けて 10.76 s かけて 0 へ。
- プレイリスト: **1 クリップだけ**(開始 38016 tick = **118.800 s**、長さ 3444 tick = 10.762 s、終端 **129.562 s**、track 24、offset 0〜35.875 拍)。= P0〜P2 だけを再生し、曲の最後の 10.8 s(レンダリング 129.6 s の終わり)で cutoff を絞る。Balance の automation(25.74〜119.78 s)とは 118.8〜119.78 s の約 1 s だけ重なる。
- Fruity Filter の保存状態(event 213、29 バイト): 先頭 DWORD 2、次が 0x400(1024)、0xCD(205)、0x400(1024)…(cutoff = 1024〔最大〕、resonance = 205 と読む。**FL の公開仕様を確認していない読み**)。
- **重要(読み)**: **Insert 47 に入る音は Insert 23(Agogo の channel 36)だけ**(Insert 46 には何も入らず、47 に直接置かれたチャンネルも無い)。Agogo を鳴らす Pattern 7 のクリップは **45.3〜61.2 s(track 15)と 62.4〜81.6 s(track 8)**。つまり **cutoff の automation の区間(118.8〜129.56 s)に Insert 47 へ入る信号は無い**(残響などの尾を除く)。Export に出る Filter の効果は『静的な設定の Filter が Agogo(45.3〜約 82 s)に掛かる分』だけで、cutoff の automation は Export の差に出ない見込み。= Filter の参照 Export で『insert 47 の cutoff automation』を判定の道具にしても、その区間の信号が無いので何も区別できない(Wine の Filter のみ有効 Export と wasm の Export を比べる場合の差は 45.3〜約 82 s に限る、と予想する)。**未確認**: 信号の有無は FLP の読み(routing と pattern の clip)で、WAV では確かめていない。
- Balance の automation(25.74〜119.78 s、Insert 1)は Insert 47 の出力にも掛かる(47 → 1)ので、Filter の Export の差には Balance の automation も重なる(Balance が wasm で実モジュールになっているので、Filter の参照 Export は Balance 有効の Wine 参照〔elog15b の構成〕に Filter を足した形が自然)。

## 11. 『Wine の同一構成の再実行』の設計(実行しない。窓の外、PM の許可後)

**目的**: elog15b 対 G9-a の ±1 LSB の 5 フレーム(1,891,929〔42.901 s、クリップの内側〕、3,709,012〔84.105 s〕、5,333,434〔120.940 s〕、5,333,941〔120.951 s〕、5,422,135〔122.951 s〕)が、(N) Wine の run 間の揺れ、(D) Wine と wasm の安定した差、(B) Balance の実装の差、のどれかを切り分ける。

**手がかり(実測)**: Balance なしの Wine(elog14)対 Balance なしの wasm の差は 4 フレーム〔495,392、5,333,434、5,333,941、5,422,135〕で、Balance ありの elog15b 対 G9-a の 5 フレームと **3 フレームが同じ位置・同じ値**(120.9 s 以降)。残りは run により違う(495,392 は elog15b に無く、1,891,929・3,709,012 は elog14 に無い)。つまり『3 つは安定、残りは揺れ』が一番ありそうな読み(未確認)。同一構成の Wine の 2 本目がまだ無いので、どちらの場合も言い切れない。wasm 側は決定的(同じ sha が何度も出る)が、G9-a の Export は 1 本しか無い。

**必要な本数と時間**
| 本 | 内容 | 時間(読み) | 備考 |
|---|---|---|---|
| Wine ×3 | elog15b と同じ構成(prefix-e2、Fruity Balance のみ有効、mp3wav-all、SSE2 cpuid、ELOG hook 28 本)を **3 回**(elog15c・15d・15e) | elog15b は 05:47:50〜05:51:33 の 3 分 43 秒(driver が 7 件の窓を認識できず 150 s 待ってから fallback した分を含む。認識を直したので短くなる見込み、未確認)。1 本 約 6〜7 分(PM の見積り)×3 = **約 20 分**、直列 | Wine の重い job は 1 本ずつ。各 run の前に ps・`free -m`・`vmstat 1 3`(swap in/out 0、MemAvailable ≥ 8 GB)。窓の間は走らせない |
| wasm G9-a ×1 | ci1 の Chrome の Export をもう 1 本(非計時、約 74 s) | 5 分 | G9-a の Export 自体の決定性(sha 94c93666… が再現するか)。Wine と重ねない |
| 解析 | `a22-balance-compare.py` を Wine ごとに、加えて下の 3 指標 | 各 数秒(CPU 軽い) | numpy 1 スレッド nice 19 |
最小構成は Wine ×2(elog15b と合わせて 3 本)。推奨は ×3(合わせて 4 本、多数決ができる)。

**解析の指標と判定**
1. **M1 Wine 同士の差**: 4 本の Wine WAV の 6 組の差のフレーム数。全組で 0(sha 同一)なら Wine はこの構成で決定的。
2. **M2 位置の出現表**: 『Wine ≠ G9-a』のフレームごとに、4 本のうち何本で出るか。4/4 は安定した差、1〜2/4 は揺れ。
3. **M3 多数決**: 4 本の標本ごとの中央値(または多数決)の WAV 対 G9-a の差のフレーム数。0 なら『G9-a は Wine と bit 一致、1 LSB は Wine の揺れ』(J1 を多数決で PASS)。
| 結果 | 読み | 次 |
|---|---|---|
| M1 = 0(4 本とも同一)で 5 フレームが 4/4 | (D) Wine と wasm の安定した差 | 位置ごとに原因を探る: 120.9 s 以降の 3 つは Balance なしでも出る旧い残差(0308 の延長)、84.105 s と 42.901 s は elog14 に無いので『Balance あり』の構成に固有 → ELOG の hook(pad f_a97930 など)で Balance の出力を直接比べる(Wine の ELOG 対 wasm のダンプ) |
| M1 ≠ 0 で 3 つが 4/4、他が 1〜2/4 | (N) 揺れ + 旧い 3 つ | 多数決(M3)が G9-a と 3 つ以外で一致するかを見る。3 つは Balance と無関係なので G9-a の判定から外す(J2 の eps 2 と同じ扱い) |
| M1 ≠ 0 で 5 つすべてが 1〜2/4、M3 = 0 | (N) 揺れのみ | G9-a は Wine の揺れの範囲で bit 一致 |
| 42.901 s(クリップの内側)が 4/4 | (B) の疑い | Balance の出力(Insert 1 の stem)を Split mixer tracks の Export で Wine 対 wasm に比べる(0308 の道具)。float の丸め(Balance の gain 計算の libm など)を疑う |
注意: (a) Wine の再実行は 1 LSB 級の話で、G9-c(Filter など)の作業を止める理由にならない。優先度は低〜中。(b) 揺れの原因(スレッド、CPUID、denormal など)の特定は別の作業。この設計は『Balance 由来か』だけを切る。(c) 本数を増やせば『位置が揺れる』確率の見積りが良くなるが、4 本で判定の方向は決まる。

## 12. Fruity Filter の参照 Export 用: 比較 script の 3 本 WAV への一般化(準備のみ。06:2x)

**結論(読み)**: `a22-balance-compare.py` は Balance の 256 クリップ(クラス、立ち上がり、クリップ揃えの r)に結びついているので、そのままは使えない。**既存の script は変えず**(取り込み済みの版・§9 の再現を壊さない)、`readwav`・`rms`・`lag_search`・clip の閾値を共有する別 script **`tools/a22-filter-compare.py`** を足した。入力 = **WINE_AB**(Balance + Filter 有効の Wine、opt1 の elog16 予定)、**WASM_AB**(Filter の test bundle の Chrome Export)、**BASE** = Balance のみの wasm(既定 = ci1 の G9-a の Export、sha 94c93666…)。

**何を判定するか(§10 の読み)**: Insert 47 に入る音は Agogo(Insert 23)だけで、pattern 7 のクリップは 45.3〜61.2 s と 62.4〜81.6 s(FLP から 116 クリップを読む)。Filter の効果が Export に出るのは『静的な設定が Agogo に掛かる分』。cutoff の automation(118.8〜129.562 s)は入力が無いので差は 0 のはず。

**見出し(固定)**: 0 入力 / 1 全体(WINE_AB-WASM_AB 〔核〕、BASE-WINE_AB、BASE-WASM_AB の差のフレーム・RMS・最大)/ 2 10 s 別 / **3 Agogo の帯(45.3〜81.6 s)の内側・尾(帯の後 3 s、`--tail-s`)・それ以外**(既知の Wine の残差 3 フレーム 5,333,434・5,333,941・5,422,135 は別に数える)/ **4 Cutoff automation の窓(118.8〜129.56 s)の差(0 のはず)** / 5 帯の中の立ち上がり / 6 Agogo のノート揃えの平均 r(t)(pattern 7 の各クリップの開始から、窓 512 フレーム)/ 7 Wine と wasm の形の相関 / 8 clip していない標本 / 9 判定の案 F1〜F6。

**判定の案**(決めるのは PM): **F1** 帯の中で WINE_AB と WASM_AB が一致(J1 相当)/ **F2** 帯と尾の外に差が無い(既知の残差を除く)/ F3 WINE_AB-WASM_AB の最大差 ≤ 2 LSB / **F4** cutoff automation の窓で差が 0(読みの確認)/ F5 r(t) の最大差 ≤ 0.01 / **F6** 帯の中で BASE と違う(Filter が出力に効いている。0 なら Filter が読めていない/効いていない側がある = 比較は空)。`--eps N` で F1・F2・F4 と立ち上がりを |diff| > N LSB で数える(1 LSB 級の揺れの扱い、§9 と同じ)。

**自己テスト**(実測。**Wine でも Filter でもない代役**。`tables/0310-filter-compare-SELFTEST-standin-not-wine-not-filter.txt`): BASE は実物(G9-a の Export)。WASM_AB の位置に『Agogo の帯だけを 2 タップで平滑化した G9-a の WAV』(Filter ではない代役)、WINE_AB に (a) そのコピー → F1〜F6 すべて PASS、(b) 代役 + 60 s(帯の中)・100 s(帯の外)・125 s(cutoff の窓)に +1 LSB → F1 FAIL・F2 FAIL・F4 FAIL、F3 PASS、`--eps 2` で全 PASS、(c) BASE と同じ(Filter が効いていない側)→ F6 FAIL・F1 FAIL・F3 FAIL・F5 FAIL。

**注意(読み、未確認)**
1. **Filter の静的な効果が小さいかもしれない**: 保存状態は cutoff = 1024(最大)、resonance = 205 と読んだ。cutoff が最大のとき Fruity Filter がどれだけ音色を変えるかは未確認で、変化が 1 LSB 未満なら F6 が FAIL する(比較が空になる)。その場合は次の変種が有効。
2. **変種 FLP(任意、Wine と wasm の両方に同じ変更)**: cutoff の automation クリップ(playlist の iid 62 の項目、開始 38016 tick)を Agogo の帯の中(例: 62.4 s = 19968 tick)へ動かす。= cutoff の絞りが Agogo に掛かり、**automation の法則**(値 1.0 → 0.355 → 0、tension)まで Wine と wasm で比べられる。0307 の変種の道具(`flp-nostretch.py` の型)で作れる見込み。Wine 側は opt1、wasm 側は seed の差し替えで開く(mp3wav-all の変種と同じ手順)。必要かは elog16 の F6 の結果を見てから決める。
3. **帯の読み**は FLP の routing と pattern の clip から(WAV では未確認)。F2 が FAIL で差の位置が帯の外にあれば、この読み(Insert 47 の入力は Agogo だけ)自体を疑う。
4. 既知の残差の位置(5,333,434・5,333,941・5,422,135)は elog14/elog15b で見えた物で、elog16 では変わる可能性がある(§11)。`--known-residue` で替えられる。

## 13. Balance の窓(Insert 1 の slot 0 の名前を 1 回クリック)の node の G3 の観測(06:5x、実エンジンを node で実行した実測)

**先頭**: 実エンジン(lift 済み FL + 実モジュール Balance、wasm TEST bundle `fl-engine-test-g9a`)を node の G3 で 1 本走らせた実測(`tools/balance-gui.g3` + `tools/bgui-run.sh`、06:49:26〜06:49:44、wall 18.5 s、nice 19)。代役・再生なし。全文 = `tables/0310-balance-gui-node-g3.txt`、窓の表 = `tables/bgui/`。

**結論**: node でも Chrome(gpu1 の rt-g9c-s3)と同じ『Balance の枠は出るが client が描かれない』が再現した(**画素が、Balance の枠 237×179 と client 領域 227×148 の両方で 100% 一致**。全画面でも 99.87%)。窓の表(全窓)では、FL 自身の枠(`TPluginForm` lid 164〔絶対 (478,230) 237×179、visible 1〕+ `TNewCaption` + `TVectorSheet` + `TWPControlForm` lid 168〔(483,256) 227×148、visible 1〕)が作られているが、**`TWPControlForm` に子窓が 0**で、Balance の DLL 由来の form/panel は**生きた窓として存在しない**。`fle: unsupported` / abort / 名前つき停止は 0 行(`fle:` は act1-net の 2 行と、end の後の `MessageLoop: idle`)。クリックの 1 s/3 s/10 s 後の窓の表は同一(時間待ちで増えない)。
- 見る点への答え: 『作られて不可視』ではなく『**生きた形では作られていない**』。ただしクリックの間に **17 個の窓(lid 154〜170)が作られ、そのうち 8 個(154〜156、159〜163)は破棄された**。表には生きている窓しか出ないので、破棄された 8 個が Balance の editor の form(作った後で破棄)か FL の一時窓かは今の表では区別できない。
- Chrome との差: 最上位の窓の幾何は同一(lid 14 (0,0) 1920×1080、Problems 窓 (614,376) 691×327)。画素の差 2,647 は、ミキサー右のスロット欄とツールバー(clock)で、クリック前(02-insert1-selected 対 insert1)にも同じ 2,660 画素あり、クリックと無関係。
- 次の手(提案、再 build が要るので PM の許可後): (a) `fle_win.c` の TEST 専用の診断で、CreateWindowEx と DestroyWindow ごとに class・title・親・rect と呼び出し元のアドレス(Balance の image 枠 0x6800000〜0x6c00000 の中か)を stderr に出す。(b) mt3 の `diag-g9-count.diff` で、クリックの後に Balance の image の中へ間接呼び出しが入るか(Balance の editor を作る関数が呼ばれたか)を数える。(a)(b)は 1 回の再 build にまとめられる(12〜17 分の見込み、読み)。

### 12.1 Filter の比較の事前の規則(判定の前に固定。07:3x、PM の依頼)
入力 = WINE_AB(opt1 の elog16。Balance + Filter 有効の Wine)、WASM_AB(Filter の test bundle の Chrome Export)、BASE = `lift/out-ci1/kit/out/export-g9a/export.wav`(Balance 版、sha 94c93666…)。
1. **効果の窓**: Insert 47 に入る音は Agogo(Pattern 7)だけ(§10 の FLP の読み)= **45.3〜61.2 s と 62.4〜81.6 s(+ 尾 3 s)**。Filter の効果が WAV に出るのは『静的な設定の Filter が Agogo に掛かる分』だけ。帯の外(と尾の外)に WASM_AB-BASE・WINE_AB-BASE の差は期待しない。
2. **cutoff の automation(118.8〜129.56 s)は判別力が無い**: その区間に Insert 47 へ入る信号が無いので、cutoff の法則(1.0 → 0.355 → 0、tension)の正否は WAV の差に出ない。使い方は『差が 0 であることの確認(F4)』だけ。0 でなければ『Insert 47 の入力は Agogo だけ』の読み(routing と pattern の clip)が誤り。
3. **先に F6(Filter が効いているか)**: BASE と違うフレームが帯の中に無ければ(静的な設定 cutoff = 1024〔最大〕・resonance = 205 が 1 LSB 未満しか変えない場合)、比較は空で何も言えない。その場合は変種 FLP(cutoff の automation クリップを帯の中、例 62.4 s = 19968 tick へ動かす。Wine と wasm の両方に同じ変更)に切り替える。
4. **判定**: F1(帯の中で WINE_AB = WASM_AB)が本体。1 LSB 級の揺れは `--eps 2` で F1・F2・F4 を数え直す(Balance の §9 と同じ『CPUID 級の差のみ』の扱い)。**既知の Wine の残差**(5,333,434・5,333,941・5,422,135 = 120.9 s 以降)は、WINE_AB でも同じ位置に出るなら外す(`--known-residue`)。出る位置は run ごとに変わりうる(§11)。
5. **既知の限界**: 帯の読みは FLP の routing と pattern の clip から(WAV では未確認)。Filter の効果の大きさは未測定。自己テストは代役で行った(`tables/0310-filter-compare-SELFTEST-standin-not-wine-not-filter.txt`)。


## PM の決定と取り込み(2026-11-07 02:38、docs のみ。製品の差分なし)

取り込み: alpha1 の 0310(G9-a: Fruity Balance を開くための runtime 側の準備)と表 `docs/investigations/alpha22/tables/0310-g9a-missing-import-rows-74.tsv`。**読みと設計のみ。build も実行も無く、実エンジン・Chrome は 0 件。依頼者の機械は 0 件。** 数字は 0309 の表(fe-lift が lift した Balance の『行なしの import』)と Win32 の契約の記憶(未照合の箇所は『読み』)。

判断:
1. **分担を確定する**: Balance を 8 つ目の実モジュールとして足す手続き(build の行・基底・フック点・volume・pack・patch)は mt3 の 0313(diff `0313-g9a-balance-module.diff`、10 ファイル・224 行、main に dry-run rc 0、静的な検査だけ)。runtime の『行なし 74』の扱いと plugin の実行時の壁(最初の process までの node テスト)は alpha1 の 0310。**実装は壁に当たってから**(Win32 の契約で `ship6-imports.txt` の末尾に追記、黙って成功を返す既定は置かない。呼ばれたら VA 名つきで停止する `fle_unsupported` の現行の規則のまま)。FLEngine の lift も同じ集合を抱えていて実行されなければ停止しないので、74 個を先回りで全部実装しない。
2. 最有力の壁(読み): init の `memset`・`EnumCalendarInfoW`・`GetCPInfoExW`。`ImageList_*`(22)はフォームが画像リストを持つ場合のみ。MDI は不要の見込み。いずれも **mt3 の冷間 build(約 03:10 以降、PM の info)の後の V2b**(alpha1 の node テストで LoadLibraryW → GetProcAddress("CreatePlugInstance") の期待 (i)〜(v))で確かめる。
3. **未確認**: 74 個のうち実際に Balance の生成までに実行される物、plugin の実行時の壁の順序、実 wasm の大きさ(B-FLE は 0312g〜i のとおり 2 つ目の前)。

### 追記(2026-11-07 05:15、alpha1 の 0310 の最終〔§5 V2b・§6 G9-b の自動化の解析〕の取り込み、docs のみ。製品の差分なし)

取り込み: alpha1 の 0310 の最終版と表・道具(`docs/investigations/alpha22/`。PNG 2 枚〔各 15 KB〕を含む)。**実エンジン(wasm のテスト bundle の node G3 と ci1 の Chrome の Export の WAV)に対する numpy の実測と FLP の読み。** V2b(node G3)では Problems の面が 7 件(Balance の行なし、691×327)で、対照の E12c の 8 件(691×343)と異なり、LoadLibraryW → 0x6800000 と GetProcAddress → 0x6abdde0 が出た。G9-b の node Export は WAV が dump に出なかった(`vfsdump` が native の fruity_g3.c にしか無いため)ので、Chrome の Export で代えた(0313 の PM の追記を参照)。alpha1 の解析の読み(自動化 clip の 256 個、差は clip の中と 3.11 ms の尾だけ)は PM が `a22-wavcmp2.py` で見た 10 s 別の差の分布と矛盾しない。**私(PM)が以前に伝えた「ゲイン 0.93〜0.99」は私の粗い最小二乗(飽和フレームを含む)で根拠ではなく、alpha1 の実測が優先する**(alpha1 に訂正を通知済み)。Balance の出力が正しいかは Wine の参照との比較で判定する(未実施)。


### 追記(2026-11-07 05:37、alpha1 の 0310 最終〔§7 比較 script、§8 残り 6 種の行なし import の増分〕の取り込み、docs のみ。製品の差分なし)

取り込み: 本文を alpha1 の最終版に差し替え(私の 02:38 の節と 05:15 の追記は残す)。表・道具: `docs/investigations/alpha22/`(`a22-balance-compare.py`、`a22-g9c-extra-imports.py`、`0310-g9c-extra-imports-by-plugin.tsv`・`-summary.tsv`、`0310-compare-SELFTEST-standin-not-wine.txt`)。

事実と限界(PM が読んだ範囲): (1) **比較 script `a22-balance-compare.py` の自己テストは代役の入力(G9-a のコピー、雑音を足した物、Balance なしの基準)で行われ、Wine の結果ではない**(ファイル名と先頭に明記)。実エンジンの結果は ci1 の Chrome の Export の WAV(sha256 `94c93666…4090`)だけで、Wine の「Balance のみ有効」の WAV(opt1 の elog15b、窓の後)が出るまで、Balance の忠実性は未判定。判定の案 J1〜J5 は案で、決めるのは PM(Wine と wasm の r(t) の一致が核)。(2) **行なし import の増分は静的な数え方**(PE の import 表と現行の行の集合の突き合わせ): Fruity Filter・Delay 2・Parametric EQ 2・Frequency Shifter は Balance の 74 と集合が同一(増分 0)、Soundgoodizer +33、FLEX +14(FLEX の補助 engine DLL は +102/+103、FLEngine にも無い物 95。生成までに読まれるかは未確認)。用途の分類(ui/init/process)は API の用途からの読みで、踏んだ観測ではない。mt3 の G9-c の設計(Filter を次の 1 本)と矛盾しない。


### 追記(2026-11-07 05:59、alpha1 の 0310 最終〔§9 Wine『Balance のみ有効』対 wasm の G9-a の比較〕の取り込みと PM の確認、docs のみ。製品の差分なし)

取り込み: alpha1 の 0310 の最終版(§9 を含む)、比較 script `a22-balance-compare.py`・`a22-balance-diffframes.py`、表(`0310-compare-elog15b-*.txt`)。`*.log` なし、400 KB 超なし。**実エンジン 2 つの実測: 原版 x86 FL(Wine 10、prefix-e2、Balance のみ有効、MP3 入力は WAV、CPUID は SSE2、opt1 の elog15b、sha256 `1cad2739b3a6fefd367c9fbe5e1357f09cb6a4c6dcb6f61627a6208fadc2d7ae`)と、lift した FL + 実モジュール Fruity Balance の wasm(G9-a の test bundle、ci1 の Chrome の Export、sha256 `94c9366687076a410f48c6fbe1485e8e22f6b528592fcc5ceba2d665d1554090`)。依頼者の機械は 0 件。**

**PM が自分で確認した事実**: `a22-wavcmp2.py`(nice 19、1 スレッド)で Wine の Balance 有効の WAV と wasm の G9-a の WAV を比べると、5,715,911 フレームで同長、**一致フレーム 5,715,906(5 フレームだけが ±1 LSB 違う)、差の RMS 0.0(−149.8 dB)**。対照: Balance なしの wasm(`381cdb5a…f979`)と Wine の Balance 有効の WAV は一致 88.8%・RMS 3,986.6・最大差 65,534。= **Balance の有無で出力が変わる大きさ(RMS 3,986.6208)は Wine と wasm で 4 桁まで同じで、変わり方(立ち上がり 2207/1242 フレーム、クリップ揃えの r(t)、尾の最大 3.11 ms)も Wine と wasm で一致する**。alpha1 の位置表では、5 フレームのうち 4 は Wine の run 間の残差(3 つは Balance を隠した elog14 と同じ位置・同じ値)で、自動化 clip の内側の 1 つ(42.901 s)は期待される個数(1.24)の範囲内。

**判断**: (1) **実モジュールの Fruity Balance は、この project の Export で、原版の FL の Balance と ±1 LSB(5 フレーム/5.7M)で一致する**。lift の忠実性の区分では欠陥なし(MP3 復号器と CPUID を揃えた条件で、Elastique・IPP・ミキサ・出力・Balance の自動化まで)。alpha1 の J1〜J5 の案は『CPUID 級の差のみ』(eps 2 で J2〜J5 が PASS)の読みを採る。(2) **G9 は未到達のまま**: 7 種のうち 1 種のみ実モジュール化、他 6 種(FLEX・Frequency Shifter・Soundgoodizer・Fruity Parametric EQ 2・Fruity Delay 2・Fruity Filter)と Fruity Wrapper は未実装、Balance の GUI(ImageList_* ほか)は未観測、test bundle は診断入りで push しない(push するのは `0313-g9a-balance-module.diff` だけ)。(3) 範囲の限界: 1 つの project、SSE2 の CPUID、Export のみ(再生・GUI なし)、Wine の同一構成の再実行による run 間の揺れは未測定、実 Windows(AVX2/FMA)との差は未確認。(4) 次: mt3 の Fruity Filter の test build を ci1 の計時の窓の後に開始(Filter の insert 47 のカットオフの自動化が出力の検証に使える)。



### 追記(2026-11-07 06:20、alpha1 の 0310 最終〔§10 offset の訂正と Filter の automation の表、§11 Wine 同一構成の再実行の設計〕の取り込み、docs のみ。製品の差分なし)

取り込み: alpha1 の 0310 の現行版(§10・§11)、表(`0310-automation-tables-balance-filter.txt`、再生成した `0310-compare-*.txt`)、道具(`a22-automation-table.py`、更新した `a22-balance-compare.py`)。`*.log` なし、400 KB 超なし。実エンジンの実行は無く、FLP の読みと既存の WAV の numpy の再計算だけ。**依頼者の機械は 0 件。**

判断: (1) **alpha1 の訂正(offset の単位)を受理する**。§6.5 の『単位が未解決・形が合わない』は誤りで、正確な offset(拍)で FLP の線形補間との相関は class B 0.867・class A 0.769、立ち上がりは P2 の後に 364 フレームの一定の遅れで Wine と wasm が一致する。**§9 の結論(差は automation が動く所だけ、Wine と wasm は ±1 LSB で一致)は変わらない。** (2) **Filter の automation の読み**: insert 47 の Cutoff の automation クリップは 1 つ(118.800〜129.562 s)で、insert 47 に入る信号は Agogo の Pattern 7(45.3〜81.6 s)だけという読み(WAV では未確認)なので、**Filter の Wine 参照では automation は判別力を持たず、Agogo に掛かる静的な設定(Cutoff 最大・Resonance 205 の読み)の効果が判定の本体になる**。Filter の参照は『Balance 有効 + Filter 有効』の累積構成(A)でよい(opt1 に伝達済み)。この読みは、Filter の test bundle の Export が Balance のみの Export と 45〜82 s で異なるかを見て確認する。(3) **§11 の同一構成の再実行は承認するが、優先度は低〜中**(Filter の bundle の作業の後、Wine と Chrome が重ならない窓で。Wine ×3(elog15c/d/e)+ Chrome ×1)。判定は M1(Wine どうしの差)・M2(『Wine ≠ G9-a』のフレームごとの回数)・M3(4 本の多数決の WAV 対 G9-a)。(4) README の行は、alpha1 の行が日付・状態の列を欠いていた(縦線 4 本)ので、既存の NF=7 の行の要約に §10・§11 を追記して作り直した。


### 追記(2026-11-07 06:23、alpha1 の 0310 最終〔§12 Filter 用の比較 script `a22-filter-compare.py`〕の取り込み、docs のみ。製品の差分なし)

取り込み: alpha1 の §12 と `tools/a22-filter-compare.py`、自己テストの表(`0310-filter-compare-SELFTEST-standin-not-wine-not-filter.txt`)。**自己テストは Wine でも Filter でもない代役の入力での script の動作確認で、実エンジンの結果ではない。** PM の判断: (1) script を受理する(WINE_AB・WASM_AB・BASE の 3 入力、Agogo の帯 45.3〜81.6 s +3 s の尾、cutoff の automation 窓 118.8〜129.56 s、F1〜F6)。(2) 読みの注意を引き継ぐ: Filter の静的な効果が小さい場合は F6(帯が BASE と違うか)が FAIL し比較が空になりうる。その場合だけ、同じ変更を Wine と wasm の両方に入れた変種 FLP(cutoff の automation clip を Agogo の帯へ移す)を作る。判断は elog16 の F6 を見てから。(3) Insert 47 への入力が Agogo だけという読みは FLP の配線とパターンの clip による読みで、WAV では未確認(F2 が帯の外の差で FAIL したらここを疑う)。(4) Wine の実行は Filter の bundle ができ、ci1 の Chrome の Export が取れた後(Wine と Chrome は重ねない)。依頼者の機械は 0 件。G9 は未到達のまま。


### PM の追記(07:07、alpha1 の §13〔node の G3、Balance の窓〕の取り込み、docs のみ。製品の差分なし)

本文を alpha1 の最終版(§13 まで)に差し替え、上の私の節(02:38 ほか)は残した。表・道具: `docs/investigations/alpha22/tables/0310-balance-gui-node-g3.txt`、`tables/bgui/`(窓の表 2 本と g3 の行。PNG はコピーしない)、`tools/{balance-gui.g3, bgui-run.sh, a22-chrome-node-pixels.py}`。

**PM が読んだ事実(alpha1 の §13、実エンジン〔G9-a のテスト bundle〕を node の G3 で 1 本実行、06:49:26〜06:49:44、依頼者の機械は 0 件)**: (1) node でも Chrome(gpu1 の stage 3)と同じく『Balance の窓の枠は出るが client は描かれない』が出た。1920×1080 で全面の 99.87% が Chrome と一致、Balance の枠(237×179)と client(227×148)は 100% 一致。(2) 窓の表: `TPluginForm`『Fruity Balance (Insert 1)』(lid 164)と、client の入れ物の `TWPControlForm`(lid 168、visible で 227×148)は**ある**が、その中に**子窓が 0**。クリックの間に 17 個の窓が作られ、8 個は作られた後で破棄された(class は不明)。(3) unsupported・abort・名前つき停止は 0 行(mt3 の §15 の読みと合う: ImageList は失敗の経路にない)。

**PM の判断**: Balance の GUI が描かれない原因の切り分け(FL が ShowEditor を呼ぶか、親付け・可視化の契約、描画の破棄、Delphi の例外の握りつぶし)には、診断入りのテスト build が要る(mt3 の §15.3 と alpha1 の §13 の提案を 1 回に束ねる: CreateWindowEx/DestroyWindow/SetParent/ShowWindow の class・title・親・rect と呼び出し元、Balance の image への間接呼びの最初の 64 回、破棄された画素数)。**許可の条件**: mt3 の Filter の build が終わり、その成果物の検証が済んだ後に 1 回だけ。テスト専用・push しない。**これは G9 ではない**(7 種のうち 1 種の Export の一致のみ、GUI は枠だけ)。lift の忠実性の分類は『契約の不足(外部 API の窓の親子・可視・更新領域の伝播)か、FL が呼ばない』のどちらかで、画面ごとの対症は足さない。

