# 0315e(仮): Frequency Shifter を 4・5 番目の実モジュール(m11 本体 + m12 の Engine_x64.dll)に足す準備(0313h 型の差分)と、GetModuleFileNameW の契約の穴(mt3。PM の instruction 11:0x)

状態: **仮**。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**、**未確認**。**実エンジン(wasm)の実行 0・Chrome 0・Wine 0**。実行したのは host の道具(PE の読み、find-hook-points、本物の抽出器 `extract` 段、lift 段 1 回、`ci/test-hybrid.sh`、native の cargo テストの一部)だけ。適用・push はしない。

## 0. 実際に実行した物

1. 2 つの DLL の PE を読んだ(pefile / capstone / 自作の読み手): `Frequency Shifter_x64.dll`(3,876,864 B、SHA-256 `35784e53…81cd`)と `Engine_x64.dll`(263,168 B、SHA-256 `1e14656b…7980`)。installer `flstudio_win64_26.1.7.5653.exe` の NSIS の install tree で、`Plugins\Fruity\Effects\Frequency Shifter\{Frequency Shifter_x64.dll, Engine_x64.dll, Plugin.nfo}` があり、**ストリームを読んで SHA-256 が上と一致**(実測)。
2. `docs/investigations/alpha20/tools/node-export/find-hook-points.py` を本体 DLL に: VirtualProtect の呼び元 0x5c3610、hook constructor 0x5c3490、3 つの target RVA **0x17e90 / 0x320b0 / 0x320d0**(先頭 5 byte は Balance・Filter・Delay 2 と同じ 3 つの Delphi runtime 関数。RVA は別。既存の `fruity-delphi-effect-hook-points.txt` のコメントにある値と一致)。
3. 差分 `0313/impl/0313h-frequency-shifter-on-delay2.diff`(7 hunk、74 行)を、RT-B の build の木 `tree-rtb-build` のコピー `tree-shf` に作った(`0315/tools/shifter-step.sh`)。`ci/test-hybrid.sh` **all checks hold**(Shifter の extract 行 2・seed 行 2 の検査と、『NEXTRA = EXTRA_MODULES の名前の数 = gen-direct の spec − 5、RM_MAX ≥ spec』を含む)。
4. その木で本物の CI の段 **`extract felift lift`**(`run-ci-build.sh`、nice 19、CI_JOBS=1、別の work dir `ci-shf`): **rc=0、lift 段 55 s**。RT-B の build(`ci-rtb`・`tree-rtb-build`)には触れていない。
5. native の cargo テスト(別の target dir): GetModuleFileNameW の runtime 差分(§4)のあとで `wall_wrp1`(新しいテストを含む 11 件)、`load`(38)、`load2_files`(4)、`loadimage`(33)、`modules`(17)、`realmod`(16)、`vfs`(22 pass + 1 fail:tree-shf は診断を getenv にした build 用の木なので `fle_vfs.c mentions getenv` の守りが落ちる=想定どおり、差分と無関係)。

## 1. 差分 `0313h-frequency-shifter-on-delay2.diff` の中身(Delay 2 の上。適用・push しない)

| ファイル | 変更 |
|---|---|
| `ci/extract-dlls.tsv` | 2 行: `Frequency Shifter_x64.dll → lift/out-plugins/frequency_shifter_x64.dll`、`Engine_x64.dll → lift/out-plugins/frequency_shifter_engine_x64.dll`(同名の `engine_x64.dll` は多くの effect にあるので、**SHA-256 が選ぶ**:`extract_fl.py` は同名の候補を順に試して SHA-256 が合う物を取る〔実測:『1 DLL』ではなく lift 段の入力として両方出た〕) |
| `lift/harness/seeds/vfs-seed-idle1.tsv` | guest `…\Plugins\Fruity\Effects\Frequency Shifter\{Frequency Shifter_x64.dll, Engine_x64.dll}` を volume に置く 2 行(`LoadLibraryW` は path 表に当たったあと volume にそのファイルがあるか調べる〔fle_load.c の `volume_file`〕ので、**Engine のファイルも volume に要る**) |
| `lift/harness/b2d2/gen-direct.sh` | spec 2 行: `frequency_shifter_x64.dll`、**base 0x8000000**(`--pdata` と**この DLL 用の** hook points ファイル、path 表の suffix `plugins/fruity/effects/frequency shifter/frequency shifter_x64.dll`)、`frequency_shifter_engine_x64.dll`、**base 0x8800000**(`--pdata` のみ。MSVC の DLL で Delphi の hook は無い、suffix `…/frequency shifter/engine_x64.dll`)。SizeOfImage 本体 0x3c8000 < 0x800000、Engine 0x46000、どちらも oggio 0xc000000・Elastique 0xb000000 より下。ImageBase は本体 0x400000、**Engine は 0x180000000**(lifter が 0x8800000 へ rebase した:実測 `new_base 0x8800000`、dir64 793、reloc_blocks 8) |
| `lift/harness/hooks/fruity-delphi-shifter-hook-points.txt` | 新規。上の 3 つの RVA(Balance 等と共通のファイルは使えない) |
| `app/scripts/build-engine-bundle.sh` | `EXTRA_MODULES="… fruity_delay_2 frequency_shifter frequency_shifter_engine"; NEXTRA=7`(DLL dir は 11 → 13 ファイル)、コメント。hyb_strip の glob は Delay 2 の差分が既に `m[0-9][0-9]-NN.c` を拾う |
| `ci/objkeys.sh` | m5 の群の glob を `m10-NN.c` → `m1[0-9]-NN.c`(m11・m12 の C が cache key から漏れないように) |
| `ci/test-hybrid.sh` | 上の 4 行の検査 |

**runtime の変更は無い**(RM_MAX = 16 に 12 が収まる、cursor の共有 handle の上限 16 は 0313f2 が入れた物)。**ただし下の §4 を入れないと Engine の path が表に当たらない**(読み。実測は下の lift までで、wasm 実行は未)。

## 2. lift 段の結果(実測)

- 全体: `TAG=tb2d2; 23 fle units, 48 module units`(Delay 2 の木で 44)。realmod の表は **12 行**(…、`fruity_delay_2_x64.dll` 0x7800000、`frequency_shifter_x64.dll` **0x8000000**、`frequency_shifter_engine_x64.dll` **0x8800000**)、path 表は **5 行**、`fle_realmod_count = 12`。DLL dir は 12 の rebase 済み module(`frequency_shifter_x64.8000000.dll`、`frequency_shifter_engine_x64.8800000.dll` を含む)。
- **本体(m11)**: lift した関数 **12,966**、4 units、C 10,314,792 + 10,316,359 + 10,317,667 + 8,120,150 B = **約 39.1 MB**(Delay 2: 12,573 関数、36.9 MB)。`unsupported(…)` の停止点は m11-00 に 17、m11-01 に 4、他 0(読み:Balance 等と同じ類)。
- **Engine(m12)**: lift した関数 **501**、1 unit、C **2,399,662 B**、**`unsupported(…)` の停止点 692**。= alpha1 の 0309 の数(VEX スカラー倍精度・FMA 692 箇所)と**一致**。これらは**実行されたときだけ**名前つきで停止する。実行されるかは Engine の CPUID / XGETBV の判定(4 / 2 箇所)の先で決まる(**未確認**。runtime の CPUID は SSE2 固定。Wine の SSE2 固定の参照は大きな効果を出した〔RMS 21,867〕ので SSE2 の経路は有る、が alpha1 の読み)。
- 他の module の C(m1〜m10)は変わらない(新しい module は `common.c` と表だけを変える。読み)。

## 3. 予測(読み。実 build の値ではない)

- V3 の観測: `LoadLibraryW "…\Frequency Shifter\Frequency Shifter_x64.dll" → 0x8000000`、`GetProcAddress(0x8000000, "CreatePlugInstance") → 0x8000000 + 0x2dc420 = **0x82dc420**`(本体の export RVA 0x2dc420)、**Engine**: `LoadLibraryW "…\Frequency Shifter\Engine_x64.dll" → 0x8800000`、`GetProcAddress(0x8800000, "dispatch") → 0x8800000 + 0x1000 = **0x8801000**`(Engine の唯一の export は `dispatch` RVA 0x1000:実測)。Problems 窓の件数は Delay 2 の 5 の続きで **4**(Wine の参照に Shifter の行があれば。**未確認**)。
- S1 への寄与: 本体 C 39.1 MB は Delay 2 の 36.9 MB の +6% なので wasm の増分は Delay 2 の +7.6〜7.8 MB の 6% 増し = **約 +8.0〜8.2 MB**、Engine の C 2.4 MB は **約 +0.5 MB**。合計 **約 +8.5〜8.7 MB**。native の合計は Delay 2 後の 88,674,176 + 3,876,864 + 263,168 = **92,814,208**。RT-B の wasm を W とすると全体の比は (W + 8.6 MB) / 92.81 MB(例: W = 166.1 MB なら **約 1.88**)。**限界の比 約 2.07**。B-FLE の方式 B(共有本体)が入ると 2 本目以降は小さい(0312g の読み)。
- 起動の取得量: seed pack に +4.1 MB(本体 3.88 + Engine 0.26)、DLL pack に同じだけ。

## 4. 見つけた契約の穴: GetModuleFileNameW が、path で読み込んだ plug-in に『Shared』の path を返す(分類 **(b)** 外部 API の契約の不足)

- Shifter の本体は、Engine を次のように読む(逆アセンブルして確認、実測): 初期化で `GetModuleFileNameW(HInstance, buf, 0x104)`(RVA 0x2d41c0〜)→ `ExtractFilePath` → 大域の変数、後で(RVA 0x2de590)`その変数 + 'Engine_x64.dll'` を `LoadLibraryW`(Delphi の `SafeLoadLibrary` 相当)→ `GetProcAddress(h, "dispatch")` → 呼ぶ。
- **今の runtime**:real module の `GetModuleFileNameW` は `FLE_VIRTUAL_SHARED_DIR` + **export directory の Name**(決定 0104、provisional)。つまり Balance・Filter・Delay 2・Shifter は **`C:\Program Files\Image-Line\FL Studio 21\Shared\<ファイル名>`** を自分の path として見る。FL が実際に渡した path は `…\Plugins\Fruity\Effects\<名前>\<名前>_x64.dll`(決定 0183)。**Learn(GetModuleFileNameW): そのモジュールを含むファイルの完全な path** = FL が `LoadLibraryW` に渡した path。
- 影響(読み・一部は実測の裏づけ):(i) Shifter は Engine の path を `…\Shared\Engine_x64.dll` と作り、path 表の suffix(`…\frequency shifter\engine_x64.dll`)に**当たらない**(= LoadLibraryW が失敗するか、別の経路〔search_volume〕に落ちる)。(ii) 回避策の `shared/engine_x64.dll` の行は PEQ2(`Frequency Splitter\engine_x64.dll`)など次の engine と**衝突する**ので採らない。(iii) 実測の裏づけ: gpu1 の cand1 の log に **`Shared\Artwork\Fonts\Fruity microfont.ttf` を探す miss**(plug-in が自分の folder の Artwork を探しているのに、folder が Shared になっている)がある。
- **直し案**(runtime のみ、`0315/impl/0315-getmodulefilename-loaded-path.diff`、119 行、tree-rtb-build に `patch --dry-run` rc 0):`fle_realmod_set_loaded_path(idx, u, n)`(`fle_load.c` の path 表に当たった `LoadLibraryW` で、volume の確認のあと、最初の 1 回だけ ASCII の path を '/'→'\' で記録)と `fle_realmod_loaded_path(idx)`。`fle_vfs.c` の `module_path` は記録があればそれを返し、無ければ従来どおり(名前で読んだ real module は変わらない)。native テスト: `wall_wrp1` に新しい 1 件(最初の path が勝つ・'/'→'\'・非 ASCII と範囲外は記録しない)、`wall_wrp1`11・`load`・`modules`・`realmod`・`loadimage` が通る。**wasm では未確認**。
- **alpha1 の契約レビュー(docs/0319 §19、PM 11:5x)の 3 点を反映した(11:5x、差分は 297 行に更新)**: (i) 記録は**ドライブ付き(`X:\` / `X:/`)か UNC(`\\server\share\…`)の絶対パスだけ**(名前だけ・相対・ドライブ無しの rooted・`X:x`・デバイス `\\.\` は記録せず、今までの答え)。path table は tail 一致なので名前だけの `LoadLibraryW` も表に当たるため。(ii) `.`・`..`・連続した区切りを畳む(Wine / `RtlGetFullPathName_U` と同じ。ルートより上に出る `..` は記録しない。`//server/share` は区切りを `\` にしてから読むので UNC)。(iii) `\\?\X:\…` と `\\?\UNC\…` は**そのまま**返す(Learn: 『読み込み時に指定された形式』。畳まない)。**最初の要求だけ**数える(最初の要求が記録できない形〔名前だけ〕でも、2 回目に絶対パスで要求されても記録しない=最初の形のまま)。native テスト: `wall_wrp1` 13 件(新しい 4 件:最初の要求・絶対パスの畳み・名前/相対/rooted の不記録・`\\?\` の保持。alpha1 の §19.3 の 7 行を含む)が通る(**実測**)。`fle_vfs.c` に触るので、RT-C の入力にする前に vfs・load・modules・realmod・loadimage・wall_fr* ほか fle_vfs / fle_load を使うテストを cargo で通す(RT-B の build が終わってから)。
- この直しを入れると、既存の 3 plug-in(Balance・Filter・Delay 2)が見る自分の path も `Plugins\…` になる(= Windows と同じ)。**plug-in が自分の folder の物(`Artwork\`、`Plugin.nfo` など)を探す先が変わる**ので、miss の path が変わる(seed に足す物の見立ても変わる)。入れる時は runtime のみの増分 build(型 C)。

## 5. 未確認 / 次

- wasm の実行(Shifter の V3、Engine の `dispatch` が呼ばれるか、VEX の 692 箇所に当たるか)は RT-B の bundle のあと。**§4 の runtime の直しが無いと Engine は読めない**(読み)ので、Shifter を build に載せる時は §4 とセット。
- 未決の点: (1) §4 の直し(契約の穴で、既存 3 plug-in の挙動も Windows に寄る)を RT-B の増分に入れるか。(2) Shifter を RT-B の結果(Delay 2 の V3 と Export が Wine の elog17 に一致するか)のあとに載せる、で良いか(PM の指示どおり)。
