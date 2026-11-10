# 0144: G8 — FLEngine に『プロジェクトを開かせ』、再生して、非ゼロの音声ブロックを出す(native)。入口 A(起動時のコマンドライン)と B(WM_COPYDATA)。G8a-0 / G8a / G8b / G8c の観測と、直した壁 4 件。仮の決定

状態: **仮の決定**。依頼・番号 0144 は PM(「決定 0144、仮」)。調査・設計・実装・ビルド・測定・テスト・文書は作業者(`g8a1`、2026-11-02 11:38〜 JST、`date` で測った経過)。作業は HEAD **cf3965e** の `git archive`(`lift/`・`docs/`・`tools/`・`app/`)で始め、PM の通知(306aaf6 = 決定 0140、5e48202 = 決定 0142、a1711c2 / d8aacf4 = 決定 0143)に従って **HEAD d8aacf4 の `git archive` に付け替えた**。clean な写しとワークツリーの `diff -ruN`(`out-*` と cargo の target は除く)が `g8a1.diff`(`patch -p1` の形、22 ファイル)。**測定の木**は d8aacf4 + 差分 v3(19 ファイル。native の各ステージ・a4・on-seh・変異検査。最終の差分 v5 との違いはテスト 3 ファイル `monitor.rs` `wall_s.rs` `vcall_thunk_dll.rs` だけで、runtime・crates・harness は同じ)で、**最終工程で HEAD が aee8877(決定 0145 = `lift/harness` の表示まわりと `app/` だけ。`lift/runtime/`・`lift/crates/` は d8aacf4 と同一)、さらに 7b82424(文書 1 ファイルだけ)に進んだので、全体テストは aee8877 + 差分(1 回目)、7b82424 + 差分(最終の通し)で走らせた**(`patch --dry-run` は最終の HEAD **27561e7**(決定 0146。7b82424 との差は文書のみ)に fuzz・offset・reject なしで当たる)。**実機 Windows との照合は 0 件。wasm・ブラウザは測っていない**(範囲外。`emcc -c` で変更した 4 つの runtime ファイルがコンパイルできることだけ確かめた)。

区分の凡例は決定 0121〜0143 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝逆アセンブルやログの解釈(確かめていない部分を含む)。**公式文書**＝今回取得した Microsoft Learn の頁(§9)。**出典なしの選択**＝頁にも実測にも値が無く作業者が選んだもの(`S-g8a-…`、§8 に撤回条件つきで一覧)。**未確認**＝確かめていない。

**肯定的に見せる点は限られる。** (1) 到達は『開く → 再生して非ゼロ』であって、**音の正しさの照合は 0 件**(実機の書き出し WAV が無い。音高の 7 段と周期は、調査書 §4.5 のノート表(鍵 24/36/…/96、各 0.25 s、位置 0〜1.5 s)から手で計算した期待と合う、という範囲)。(2) **実時間の連続性は条件に入れていない**(PM の決定 2): a-play の 2.5 仮想秒は native -O2 で壁 48 秒(約 1/20)。(3) チャンネルラックの『4 行』は**ステップ列と pan/vol の 4 組が描かれる**ことで、**名前ボタン('808 Kick' など)は最下行の '808 Snare' しか描かれない**(原因は調べていない)。文字は描かれるが、missing-sample のダイアログの一覧の文字は描かれない。(4) 入口 B は**ウィザードが出ている間(主窓が disabled)は無視される**(エンジン自身の前提条件。§6)。通常起動で B を使うにはウィザードを閉じる入力が先に要る(No thanks のクリック + Enter。決定 0136・0140 の経路)。(5) FRUITY 無効の a4 の入口数が **1,009,721 → 1,009,731**(+10)になった。原因は `GetDpiForMonitor` の行(§10.2、行を外した対照で 1,009,721 に戻ることを測った)。(6) `Empty.flp` の ReadFile の合計は依頼の想定(46,361 B)ではなく **33,027,459 B**(読み方の癖。§3.1)。(7) 依頼の `wall_fr18.rs` は HEAD に決定 0143 の同名ファイルがあるので **`wall_fr19.rs`** にした。(8) 自分の誤りがある(§13): 存在しない道具の呼び出し、古い fe-lift での生成、前景 20 分の中断、**既存テストの『この API に行は無い』の固定を事前に探さなかった(全体テストの 1 回目で 3 件落ち、通しをやり直した)**。

## 1. 先に結論

| 項目 | 結果(実測。HEAD d8aacf4 + この差分、native -O2・計装なし、仮想時計、VFS の種 `vfs-seed-g8.tsv`) |
|---|---|
| 到達した段階 | **G8a-0 / G8a / G8b / G8c のすべて**(止まる条件 (a))。**G8a は A と B の両方**、G8b は A と B(通常起動 → ウィザードを閉じる → B)、G8c は A と B |
| G8a-0 | 起動時の FLP/FST パーサ `f_f54760`・`f_f51fd0`・`f_f46490` は**各 2 回**入る(1 回目 = `PreviewTrack.fst`(704 B)、2 回目 = `Empty.flp`)。`Empty.flp` の guest の ReadFile は 1,420 回、返却の合計 33,027,459 B(最初の 1 回が 46,361 B = ファイル全体)。入口の通し番号は §3.1 |
| G8a(`Basic.flp`、A) | `Basic.flp` 49,457 B を読み(1,555 回の ReadFile)、4 つの PCM サンプル(74,468 / 46,492 / 28,180 / 32,992 B)を 1 回ずつ ReadFile + `CreateFileMappingW` + `MapViewOfFile` で読む。主窓の題 `Basic.flp - FL Studio 2026`、テンポ **130.000**、チャンネルラックに **4 行**(名前ボタンは最下行のみ)、**TMsgForm なし**。主窓 lid 13 の面 SHA `697acaf7f818a7b92115f3ac782e6d409e9f39d357d21f3192e4e7cff55e3bef`(**native の独立 2 回で一致**。窓表 103 行の SHA `b96648bf…` も一致) |
| G8b(`Sampler.flp`、A) | `Sampler.flp` 81,831 B(1,907 回)+ `SHP_Sawing Synth.wav` 8,336 B(1 回 + マップ)。題 `Sampler.flp - FL Studio 2026`、テンポ 120.000、チャンネルラックの『Sampler』の波形と、保存されていた Sampler の設定窓に `File SHP_Sawing Synth.wav` と波形。面 SHA `4260cfef11f93a5ebf2e8c8455805e985f12a911bd551a6bef5e858165956134`(独立 2 回一致) |
| G8c(Space で再生) | リングに**非ゼロ**: 2.574 秒分 2,936 ブロック(1 ブロック 1,024 フレームほか)のうち非零ブロック 2,409、非零フレーム 91,767、max\|x\| = **0.571655**。ブロックの SHA 列・f32 ダンプ(905,216 B)は**独立 2 回で完全一致**。音: 出力 0.2524 s から 7 音、**0.25 s ごとに 1 オクターブずつ上がる**(ゼロ交差の周波数 94.7 → 194.7 → 389.5 → 781.6 → 1568 → 3134 → 6268 Hz、隣の比 2.00〜2.06)、1 周目は 2.0124 s で終わり、2 周目が 2.2524 s から(**周期 2.0000 s**)。§3.4 |
| 入口 B(WM_COPYDATA) | `Empty.flp` を A で起動 → `fle_appfile_open` → `Basic.flp` が開く(題が `Basic.flp - …` に。受け手 `f_11d1c80` の入口 52,645,518 から 2,665 入口後の 52,648,183 で `CreateFileW`)。通常起動 → No thanks → Enter → B → `Sampler.flp` → Space → 非零の音(非零フレーム 77,594/1 周目 = A と同じ)が通る(b-flow)。**ウィザード表示中の B は無視される**(§6) |
| 直した壁 | **4 件**(§5)。①`shell32!SHAddToRecentDocs` 行なし ②`DefWindowProcW(WM_COPYDATA)` ③`UnionRect` の両空 ④`shcore!GetDpiForMonitor` の遅延ロード失敗(0xC06D007F) |
| 全体テスト | **最終の通し(1 回): 140 result・2,059 passed・0 failed・0 ignored、終了 0、45 分 56 秒**(7b82424 + 差分 v5)。その前の 1 回目(aee8877 + 差分 v4)は 140 / 2,056 passed / **3 failed**(私の行が既存の固定と衝突。直して通し直した)。§10.5 |
| a4(FRUITY 無効、独立 2 回) | §10.2: 942/942、895/895、functions 5,310、entries **1,009,731**(HEAD の木は 1,009,721) |
| FRUITY 有効 on-seh | §10.3 |
| 変異検査 | §10.4: 24 件(23 件は `wall_fr19` が、1 件は `a9_a10` が検出)、生存 0(最初の版で M21 が生存 → テストを足して検出、M13 はコンパイルエラーでの偽の検出 → 直して取り直した) |
| gencmp | §10.1: FLEngine の関数 62,326 のうち**本体が変わったのは 1 本**(`f_593320` = `SHAddToRecentDocs` の thunk)。モジュール 5 つは 0 本 |

## 2. PM の決定 1〜6(変えていない)と、解釈が分かれた点

1. 入口は **A(起動時のコマンドライン)と B(実行中の WM_COPYDATA)**。ファイルダイアログ・OLE ドロップは範囲外。E(文字列セル `0x16bcca8` への直接書き込み)は**使っていない**(成果に数えるものは 0)。
2. 最小デモは `Sampler.flp` + `SHP_Sawing Synth.wav`。段階 G8a-0 / G8a(`Basic.flp`)/ G8b / G8c(非零。連続性は条件に入れない)。G8d は範囲外。
3. ファイルは VFS の種(決定 0110/0129)。インストーラ `flstudio_win64_26.1.7.5653.exe`(SHA-256 `04393e79…5fc02`)から `tools/extract_nsis2.py` と `docs/investigations/g8-project-load/g8_extract.py` で**読み取りのみ**で `lift/out-g8a1/inputs/` に抽出した(8 ファイル、326,996 + 49,457 B、SHA-256 は `logs/inputs-sha256.txt`)。製品のファイルは repo に入れていない(`vfs-seed-g8.tsv` は host パスと SHA だけ)。
4. 新しい runtime 関数は新ファイル `fle_appfile.c/.h`。他のファイルの変更は局所(§4)。`imports_table.rs` は末尾追記のみ(2 行)。
5. 偽らない。Learn に値が書かれない選択は §8。
6. 止まる条件: (a) に達した。

解釈が分かれて即興で決めなかった点: (i) 『Play は F2 の入力経路で送る』は、native の台本から `fle_input_host_key(KDOWN/KUP, VK_SPACE=0x20)`(ページの `fle_g3_key` が呼ぶ C 関数そのもの)を呼ぶ形にした。**JS の `KeyboardEvent.code` → VK の表は通っていない**(S-g8a-key-script)。(ii) 『`fle_win_input_send` で届ける』は、**送り手のスレッドが待たない**ので `fle_win_input_send`(ブロックして待つ)でなく、`fle_thread_send` で受け手の sent キューに入れる形にした(§4、S-g8a-copydata-sent)。(iii) 『7 列』と指定された README の行は、実際の表が **5 列(6 本の `|`)** なのでそれに合わせた(`docs/README-0144-line.txt`)。(iv) 『ノート位置 0/0.25/…/1.5 s に対応する塊があるか』は、塊(音の切れ目)ではなく**音高の段**(ゼロ交差の周波数)で対応づけた(切れ目なく続くため。§3.4)。

## 3. 段階ごとの受け入れ条件と観測

観測の道具(`lift/harness/g8_probe.c`、`fruity_g3.c` の台本命令 `open` / `openstate` / `dump`、`tools/`): 台本(`FLE_G3_SCRIPT`)が idle ごとに 1 命令を実行する(決定 0136)。ファイル読みは `--wrap` した `CreateFileW` / `ReadFile` / `CreateFileMappingW` / `MapViewOfFile` の記録(`FLE_G8_FILETRACE=1`)、音声は `fle_audio_ring_append` の `--wrap`(`FLE_G8_AUDIOTRACE=1`、ブロックごとに最初のフレーム・最大絶対値・非零フレーム数・SHA-256、`FLE_G8_AUDIODUMP` で f32)。いずれも runtime の振る舞いを変えない(harness のリンクだけ。既定のビルドにも含まれない)。入口数は native -O0 + 計装(`trace_call_count()`)を gdb で読んだ。

### 3.1 G8a-0(測定のみ。起動時の `Empty.flp`)

native -O0 計装、既定の種(idle1)、ファイルなし。`MessageLoop: idle` までの壁 110.6 s、CreateFruityInstance から 52,821,093 入口(プログラム開始から + 1,052,550)。gdb の breakpoint で関数の先頭(プロローグの後)での `trace_call_count()` を読んだ(**フックの数え方と ±1 の差がありうる**):

| 関数 | 1 回目(`PreviewTrack.fst`)のプログラム開始からの入口 | 2 回目(`Empty.flp`) | 2 回目の CreateFruityInstance からの入口 |
|---|---:|---:|---:|
| `f_f54760`(FLP の読み込み入口) | 34,701,856 | 45,614,492 | 44,561,942 |
| `f_f51fd0`(FLP パーサ) | 34,702,253 | 45,614,889 | 44,562,339 |
| `f_f46490`(イベントのループ) | 34,761,093 | 45,615,204 | 44,562,654 |
| `f_11db420`(既定プロジェクトの置換) | - | 45,609,030(1 回) | 44,556,480 |
| `f_11d58c0`(プロジェクトを開く本体) | - | 45,611,167(1 回) | 44,558,617 |

- **実測**: パーサは起動中に**2 回**動く。1 回目は **`Data\System\PreviewTrack.fst`**(704 B。FST = FLP と同じ形式の状態ファイル、は第三者(Holzchopf/flp-file の README)の記述で、パーサが同じ関数であることは今回の入口数で確かめた。`CreateFileW` が入口 34,702,235、guest の ReadFile 52 回、返却合計 23,092 B、最初が 704 B)、2 回目が `Empty.flp`(`CreateFileW` が入口 45,614,871、ReadFile **1,420 回**、返却合計 **33,027,459 B**、最初の 1 回が 46,361 B = ファイル全体、最後が 1 B)。依頼は『46,361 B のはず』だったが、**エンジンは 1 イベントごとに 1 MiB を要求し、シークの後に『残り全部』を返す**ので、返却の合計はファイルの大きさにならない(読み。1,420 回 ≒ Empty.flp のイベント 1,413 個 + α)。**ファイルの大きさ(一意のバイト)は 46,361 B**。
- 窓: 既定の起動はウィザード(lid 48)の idle で、主窓 lid 13 は disabled。面 SHA lid 13 `0133698f232bcb082545f18fa9a99919f0f2813bb46f7a0fa070a561aa6ad14b`(決定 0142 の idle の値と一致)、lid 48 `1223c73ff0b5451d013cf8e76bf3f4e387c07000b2c1dc152e49031a030b3018`(決定 0138 と一致)。lid 22(Settings)は**出ない**(決定 0138 は 3 面。決定 0139 の仮想デバイスのためと読むが、確かめていない)。
- **A(コマンドライン)だと `f_11db420` に入らない**(`f_11d58c0` が直接呼ばれる)。`f_f54760` は A でも 2 回(`PreviewTrack.fst` と指定したプロジェクト)。Basic: 34,730,806 / 45,638,911、Sampler: 34,730,792 / 45,638,897。

### 3.2 G8a(`Basic.flp` を開く)

| 受け入れ条件(§7) | 観測 |
|---|---|
| `CreateFileW(…Basic.flp)` と 49,457 B の読み | **A**: GENERIC_READ の open(入口 45,639,290、-O0)、ReadFile 1,555 回、最初が 49,457 B(返却合計 39,702,946 B)。**B**(Empty を A で起動 → `fle_appfile_open`): open が入口 52,648,183、同じ 1,555 回 |
| 4 サンプルの `CreateFileMappingW` / `MapViewOfFile` | **通った**(Kick 74,468 / Clap 46,492 / HiHat 28,180 / Snare 32,992 B。各 ReadFile 1 回 + マッピング + ビュー(全体)。ファイルのパスは `%FLStudioFactoryData%\Data\Patches\Packs\Legacy\Drums\Dance\…` で、種のツリーの同じ接頭辞に解決された = `S-g8-resolve` の 5 サンプルでの確認) |
| 面の SHA が Empty と異なり、native の独立 2 回で一致 | **A**: `697acaf7…`(独立 2 回一致。-O0 計装でも同じ)。B(Empty → Basic、開いた 107 仮想 ms 後): `2b066d2a695338e6accb37a55a951e2a45e7ea026f6e703c2f5da607f6be6a1f`(独立 2 回一致)。**A と B の面は同じプロジェクトでも同じではない**(標本の時刻が違う。B は読み込みの直後でレイヤード窓 lid 18 が出ている) |
| チャンネルラック 4 行、テンポ 130.000(画像) | **PNG で確認**(`png/g8a-basic-A-lid13-1930x1092.png`): テンポ **130.000**、ステップシーケンサの行が **4 行**。**名前ボタンは最下行の『4』『808 Snare』しか描かれず、行 1〜3 の名前と番号は無い**(pan/vol のつまみは 4 行ぶんある)。収束後の面(仮想 2〜6 s、SHA `41024c8f…`)でも同じ。原因は調べていない(読み: 窓の可視領域(決定 0142)か描画のモデル。**未確認**) |
| TMsgForm(エラーダイアログ)が出ていない | **出ていない**。窓表: lid 18 TLightweightLayeredControl(有効)、lid 13 TFruityLoopsMainForm `Basic.flp - FL Studio 2026`(有効)、lid 5 TApplication(有効)。**A では GDPR・ウィザードが出ない**(読み: ファイル引数があると初回起動の案内を出さない。確かめていない) |
| 止まらず idle に戻る | 戻る(native -O2 で壁 11.2 s、-O0 計装で 114 s) |

サンプルが無いとき(出典なしの選択ではなく観測): 種から `Basic 808 *.wav` を外して A で開くと、**`TMissingPluginsForm`『Problems loading the project』**(lid 113、691×287、有効)が出て主窓は disabled、題は `FL Studio` のまま(`png/g8-missing-basic-samples-lid113-691x287.png`、主窓 `98f3c4e6…`)。`SHP_Sawing Synth.wav` を外した Sampler.flp も同じダイアログ(lid 139、主窓 `2d549040…`)。**ダイアログの一覧の文字は描かれない**(青い選択行と空欄)。作者の環境の絶対パスを指す .flp は今回の 2 本には無い(どちらも `%FLStudioFactoryData%`)ので、『作者のパスが `Data\Patches\Packs\...` に解決される条件』は**観測していない**(`%FLStudioFactoryData%` = インストール先のディレクトリ、で解決する、という範囲)。

### 3.3 G8b(`Sampler.flp` を開く)

| 受け入れ条件 | 観測 |
|---|---|
| サンプルの読み(8,336 B) | **A**: `SHP_Sawing Synth.wav` の open → ReadFile 1 回で 8,336 B → `CreateFileMappingW` → `MapViewOfFile`。`Sampler.flp` は 1,907 回の ReadFile(最初が 81,831 B) |
| チャンネルラックにサンプル名 | チャンネルボタンの名前は **『Sampler』**(.flp に保存された名前)。**サンプル名は保存されていた Sampler の設定窓に出る**: `File SHP_Sawing Synth.wav`、波形、Loop 範囲、チャンネルラックの行の右にも波形のサムネイル(`png/g8b-sampler-A-lid13-1930x1092.png`)。テンポ **120.000**、題 `Sampler.flp - FL Studio 2026` |
| 面 | `4260cfef…`(独立 2 回一致)。窓表 120 行(SHA `a76ef4b4…`) |
| 通常起動 → ウィザードを閉じる → B | 面 `a4d21415…`(開いた 307 仮想 ms 後。独立 2 回一致)。題が `Sampler.flp - FL Studio 2026`、主窓有効、**エラーダイアログなし** |

### 3.4 G8c(再生。Space)

台本 `scripts/play.g3`(`snap` → `tap 0x20` → 仮想 300 ms → `snap` → 仮想 2,200 ms → `snap`)。Space は押下と離すを同時刻に置く(決定 0136 の `tap`)。主窓の面は 300 ms 後 `19dc6dd1…`、2.5 s 後 `4b6ee275…`(独立 2 回一致)。`png/g8c-play-A-at-1s-lid13-1930x1092.png`(仮想 1 s 後)で、**トランスポートの再生ボタンが一時停止の形(オレンジ)、時間表示が 1:08:10 に進み、マスターのメーターが点灯、チャンネルラックの波形に再生位置**(目視)。

| 項目(実測。`logs/stages/a-play-*.err` と `tools/audio-summary.py`、`tools/notes-zcr.py`) | 値 |
|---|---|
| リングへの append | 2,936 回、113,507 フレーム(2.574 s)。最初の 257 ブロック(0.252 s)はゼロ(デバイスが再生を始めてからエンジンが音を出すまで) |
| 非零 | 非零ブロック 2,409、非零フレーム 91,767、max\|x\| **0.571655**。非零の最初のフレームは出力 11,131(0.2524 s)、そのブロックの仮想時刻 3,600,194 ms |
| 塊 | 『音』**0.2524〜2.0124 s**(77,594 非零フレーム)→ ゼロ 2.0124〜2.2524 s → 2 周目の音 2.2524 s〜(周期 **2.0000 s** = 120 BPM の 1 小節) |
| 7 ノートとの対応 | ノート表(`docs/investigations/g8-project-load.md` §4.5 = `Sampler.flp` のイベント 224 の読み、PyFLP の名前。**私は再読していない**): 鍵 24/36/48/60/72/84/96、各 48 tick(PPQ 96、120 BPM で 0.25 s)、位置 0 / 0.25 / … / 1.5 s。音が切れ目なく続く(サンプルのループ + 次のノートが前のを置き換える)ので、**音高の段**で対応づけた。再生開始からの 0.03〜0.22 s ごとの区間のゼロ交差の周波数: **94.7, 194.7, 389.5, 781.6, 1568.4, 3134.2, 6268.4 Hz**(隣の比 2.06, 2.00, 2.01, 2.01, 2.00, 2.00)= 1 ノートごとに 1 オクターブ。7 段 × 0.25 s で終わりが 2.0124 s(最後のノートの 0.25 s + 0.01 s) |
| 独立 2 回 | ブロックごとの SHA-256 の列(2,936 行)、f32 ダンプ全体(905,216 B、SHA-256 先頭 `8ba30634e674dd63`)が**完全に一致** |
| 通常起動 + B(`b-flow`) | リング 7,542 ブロック、非零 4,665、非零フレーム 96,803、max 0.571655、非零の最初が出力 2.0458 s。1 周目の非零フレーム数 77,594(A と同じ)。独立 2 回一致 |

**アンダーラン**: 仮想時計の下では測れない(リングの消費は仮想時計。決定 0139)。a-play の壁時間は 2.5 仮想秒に 48〜50 秒、b-flow 全体で 75 秒。G8c-RT(連続)は別立てで、ここでは測っていない。

## 4. 実装した場所

| ファイル | 内容 |
|---|---|
| **新規 `lift/runtime/fle_appfile.c/.h`** | (A)`fle_appfile_cmdline_w/a`(`FLE_CMDLINE`。UTF-8 → UTF-16、サロゲートペア、不正な UTF-8 は停止)。(B)`fle_appfile_open(path_utf8)`: 主窓(class `TFruityLoopsMainForm`、窓表の最上位窓)を探し、**calloc した領域**に COPYDATASTRUCT(24 B)+ UTF-16 のパス + NUL を作り、`fle_sent` を**窓の所有スレッドの sent キュー**に入れる(`fle_thread_send`。**待たない**)。受け手のスレッドが次にメッセージ取得(Get/PeekMessage 等)をしたときに、既存の `fle_win_process_sent` が窓手続きをそのスレッドの文脈で呼ぶ(決定 0122 の SendMessageW の規則)。`fle_appfile_state(request, &result)`、`fle_appfile_main_window()`、wasm 用の export `fle_app_open` / `fle_app_open_state`(`EMSCRIPTEN_KEEPALIVE`、`__EMSCRIPTEN__` のときだけ)。(API)`fle_imp_SHAddToRecentDocs`、`fle_imp_GetDpiForMonitor` |
| `lift/runtime/fle_env.c`(局所) | `GetCommandLineW/A` が weak の `fle_appfile_cmdline_w/a` を見る(各 1 か所)。**未設定・空は従来の値のまま**。fle_appfile.c をリンクしないプログラム(既存テストの多く)は影響なし |
| `lift/runtime/fle_input.c`(局所 1 case) | `fle_input_defproc` に `WM_COPYDATA`(0x4A)= 0(S-g8a-copydata-default) |
| `lift/runtime/fle_user_nowin.c`(局所) | `UnionRect` の両空を停止から (0,0,0,0) と 0 に(S-g8a-unionrect-empty) |
| `lift/crates/fe-lift/src/imports_table.rs` | **末尾に 2 行**: `shell32!SHAddToRecentDocs`、`shcore!GetDpiForMonitor`(決定 0140 の 4 行の後) |
| `lift/harness/g8_probe.c`、`harness/g8/link-flags.txt`、`harness/fruity_g3.c`、`harness/seeds/vfs-seed-g8.tsv` | 観測の道具(§3)、台本命令 `open` / `openstate` / `dump`、`g3_sha256_hex`、VFS の種(idle1 + 7 ファイル) |
| テスト | 新規 `tests/wall_fr19.rs`(13 件)、`harness/web/g8/g8_scenarios.c`(12 シナリオ)、`tests/rows_0144/mod.rs`。既存の直し 8 ファイル: `wall_fr4/5/13/15/16.rs` の『後続行数』に決定 0144 の 2 行を足した、`a9_a10.rs`(UnionRect の両空の期待)、**全体テストの 1 回目で落ちた 3 件の直し**: `monitor.rs`(『GetDpiForMonitor に行は無い』の固定を外した)、`wall_s.rs`(shcore の GetProcAddress の期待。GetDpiForMonitor は found、行なしの例は GetScaleFactorForMonitor に替え、テスト名を `shcore_has_only_the_two_dpi_rows` に)、`vcall_thunk_dll.rs`(未対応の import thunk の数 227 → 226) |

**触っていない**: `fle_win.c`、`fle_thread.c`、`fle_gdi.c`、`fle_gdc.c`、`fle_layered.c`、`fle_visrgn.c`(テストが『これらに `fle_appfile` の語が無い』を確かめる)、`fe-lift` / `fe-decode` のソース、`app/`、`a4-fruity.sh`。wasm のビルド・リンクの設定。`fle_*.c` の一部だけをリンクする既存テストのリンク列に新ファイルは**要らない**(weak のため。確認: `wall_fr14` ほか全体テスト)。

## 5. 壁の記録(4 件。40 壁の枠には遠い)

| # | 停止 | 場所・原因(実測 / 読み) | 直し | 規模 |
|---|---|---|---|---|
| 1 | `unsupported instruction at 0x593320`(A、Basic.flp、起動 10 秒) | `f_593320` = `jmp [IAT shell32!SHAddToRecentDocs]`(行なし)。呼び元は静的調査(`g8-project-load.md` §3.2)の読みでは `f_11d9340`(読み込み後)で『条件付き』と予想されていた。**実際の呼び元は確かめていない** | 行 + `fle_imp_SHAddToRecentDocs`(§4。Learn: `void f(UINT, LPCVOID)`、戻りなし。S-g8a-recentdocs) | S |
| 2 | `DefWindowProcW: message 0x4a: the default processing is not answered`(B) | エンジンの受け手 `f_11d1c80` は最後に VCL の `DefaultHandler`(`[vtable-0x38]`)を呼び、`CallWindowProc` → `DefWindowProcW(0x4A)`。runtime の defproc に WM_COPYDATA が無かった | `fle_input_defproc` に case(0 を返す。Learn: 処理しないなら FALSE) | S |
| 3 | `UnionRect: both source rectangles are empty: the page does not say what is stored in lprcDst`(A、Sampler.flp) | Learn は『空矩形は無視、非空でなければ戻り値 0』だけ。IntersectRect の空の結果と同じ値にした | (0,0,0,0)と 0(S-g8a-unionrect-empty)。`a9_a10.rs` の『両空は停止』を『ゼロ矩形』に直した | S |
| 4 | `RaiseException: ControlPc is in no mapped image, so no unwind data exists for it (ControlPc 0x10005000107e5)`(A、Sampler.flp) | シャドウスタックの先頭 `RaiseException(0xC06D007F)` ← `f_41d9a0`(遅延ロードヘルパー)。`GetProcAddress(shcore, "GetDpiForMonitor")` が NULL(`GetLastError` を取ってから raise)。呼び元は `MonitorFromWindow` の直後(`f_85dfcd`、プロジェクトに保存された窓の位置の復元と読む) | 行 + `fle_imp_GetDpiForMonitor`(Learn: 非 DPI 認識のプロセスは 96、dpiX = dpiY。S-g8a-dpi)。**raise が unwinder で停止すること自体(ControlPc が不正)は直していない**: 別の遅延ロードの欠落でも同じ停止になる(未調査) | S |

壁ではないが記録する自分の誤り: 付け替えの直後の最初の生成で、cargo を `CARGO_TARGET_DIR` なしで回して**古い fe-lift で生成**し、決定 0140 の行が抜けた版を作った(ウィザードを閉じる経路で `indirect call to 0` が再び出て気づいた)。作り直した(§13)。

## 6. WM_COPYDATA を誰が消費するか(PM が『未追跡』とした点の追跡結果)

native -O0 計装 + gdb(`tools/copydata2.py`)、Empty を A で起動 → 台本の `open` → Basic.flp。プログラム開始からの入口(実測):

| 入口 | 何が起きたか |
|---|---:|
| 52,645,510 | 台本の `open`(`fle_appfile_open`。idle フックの中) |
| 52,645,518 | **`f_11d1c80`**(主窓のメッセージ表の 0x4A の受け手)。thread 0(主窓の所有者)が sent メッセージを取り出して窓手続きを呼んだ |
| 52,645,527 | **`f_1262b10`**(前提条件 `[[[0x15e4790]]+8]->vtable[0x80]`) |
| 52,645,542 | **`f_92d0f0`**(スレッド安全なキューへ `TLoadFlpAction`(0x18 B、+8 にパス)を追加) |
| 52,645,555 | `PostMessageW`(キューの通知イベント `[queue+0xc]` が呼ぶ) |
| 52,645,607 | `DispatchMessageW`(thread 0 自身のメッセージ処理) |
| 52,645,610 | **`f_92d710`**(読み: キューを空にするメッセージ窓の手続き。決定 0129 の壁 16 のスタックにもいる) |
| 52,645,613 → 614 → 643 → 728 | `f_1180b50`(`TLoadFlpAction.Execute`)→ `f_ed5d40`(`TFLProject` の仮想メソッド)→ `f_1262b10`(もう一度)→ **`f_11d58c0`**(プロジェクトを開く本体) |
| 52,648,182 | `CreateFileW(Basic.flp)` |

**消費者は thread 0 自身のメッセージループ**: 受け手が付けたキューの通知が `PostMessageW` し、その `DispatchMessageW` で `f_92d710` が `Execute` を呼ぶ。外から何かを足す必要はなかった。

**前提条件 `f_1262b10`**(読み): `*[0x15e04a8]` と `*[0x15e0ff0]` が 0、`vtable[0x48]`(= `f_1262950`、`*[0x15e1890]`)が 0、`[[0x15e30f8]]+0x168`(モーダルのレベルと読む)が 0 以下、そして `IsWindowEnabled(主窓)` が真、のときだけ真。**ウィザードが出ている(主窓 disabled)とき `fle_appfile_open` は届くが、エンジンが無視する**(実測、5e48202 の木の native -O0: B1 = 通常起動の idle に `open`、state 2 / result 0、`Basic.flp` の `CreateFileW` が起きない。最終の木では B1 を取り直していない)。**これはエンジンの挙動で、偽っていない**。アプリ側は『ウィザードを閉じてから B』にする(実測: 通常起動 → `click 1036 561`(No thanks)→ 仮想 600 ms → Enter → 仮想 1,000 ms → B → 開く)。

結果の値: 窓手続きの戻り(TMessage.Result)は、(読み)ハンドラが 1 を入れた後に呼ぶ `DefaultHandler` が `DefWindowProcW` の 0 で上書きするので、**`fle_appfile_state` の result は 0**(実測: どの B でも 0)(エンジンが開いたかどうかの印ではない。開いたかは `CreateFileW` と題で見る)。

## 7. 受け入れ条件の総括

| 段階 | 条件 | 結果 |
|---|---|---|
| G8a-0 | `f_f54760`／`f_f51fd0` の入口数、`Empty.flp` の ReadFile 合計、入口の通し番号 | 入口数 2(PreviewTrack.fst と Empty.flp)、合計 33,027,459 B(46,361 B ではない。理由 §3.1)、通し番号 §3.1 |
| G8a | `Basic.flp` の読み、ラック 4 行、テンポ 130.000、面 SHA を native 独立 2 回で一致、TMsgForm なし | **満たした**(ただし名前ボタンは最下行のみ) |
| G8b | サンプルの読み 8,336 B、ラックにサンプル名 | 読みは満たした。**名前はチャンネルボタンに出ず、Sampler の設定窓に出る**(§3.3) |
| G8c | 再生後、リングに max\|x\|>0、ブロック SHA の独立 2 回一致、塊と 7 ノートの対応 | **満たした**(max 0.5717、2,936 ブロックの SHA 列一致、7 段の音高と 2.0 s 周期) |
| A と B の両方で G8a | B は起動後に別のファイル | **満たした**(B = Empty を A で起動 → Basic。通常起動 → ウィザードを閉じる → B → Sampler も) |

## 8. 出典なしの選択(`S-g8a-…`)

| 名前 | 値・内容 | 理由 | 撤回条件 |
|---|---|---|---|
| `S-g8a-cmdline-env` | `GetCommandLineW/A` の値 = 環境変数 `FLE_CMDLINE` の全文(UTF-8)。未設定・空は従来の値(仮想 DLL パスの引用符つき、引数なし) | PM の決定 4。wasm でも `getenv` が使える(manifest の `engine.env`、決定 0137) | harness の引数など別の与え方が決まったとき |
| `S-g8a-cmdline-ascii` | `GetCommandLineA` で 0x7F を超える文字があれば名前つきで停止 | 8 ビットのコードページ(1252)への変換表を持たない | 変換表のモデルを作るとき |
| `S-g8a-copydata-sender` | 送り手 = ホスト: `wParam` = 0、`dwData` = 10(経路。エンジン自身の送り手 `f_117b930` は 0xA)、`lpData` = UTF-16 + NUL、`cbData` = (単位数 + 1) × 2 | Learn の WM_COPYDATA・COPYDATASTRUCT の形。送り手の窓が無い | 送り手の窓を持つモデルを作るとき |
| `S-g8a-copydata-sent` | sent メッセージとして受け手スレッドのキューに入れ、**送り手は待たない**。結果は `fle_appfile_state` で後から読む | Learn の WM_COPYDATA の頁は SendMessage を前提にする(『SendMessage が返った後でデータが要るならコピーする』)。『処理されるのは受け手が取得関数を呼んだとき』は決定 0122 が読んだ SendMessage の頁。ホストにスレッドがない | 同期の結果が要るとき |
| `S-g8a-copydata-limits` | 待ち 8 件、パスは 520 UTF-16 単位まで、要求番号は通し | 領域の上限。MAX_PATH 相当の仮想ボリュームのパスより長い | 上限が足りないとき |
| `S-g8a-copydata-default` | `DefWindowProcW(WM_COPYDATA)` = 0 | Learn: 『処理しないなら FALSE』 | 実機で測ったとき |
| `S-g8a-recentdocs` | `SHAddToRecentDocs` は何もしない(SHARD_PATHA 2、SHARD_PATHW 3、NULL だけ受け、他は名前つきで停止) | 仮想マシンに『最近の項目』が無く、ゲストから見える効果が無い | 最近のファイルの一覧を作るとき |
| `S-g8a-dpi` | `GetDpiForMonitor`: 仮想モニター 1 つ、MDT 0/1/2 すべて 96/96、他は E_INVALIDARG(0x80070057)。出力は失敗時に変えない | Learn: 非 DPI 認識は 96、dpiX = dpiY。ANGULAR/RAW の非認識での値は頁に無い | 実機の値、DPI 認識を持つとき |
| `S-g8a-unionrect-empty` | `UnionRect` の両空 → (0,0,0,0) と 0 | Learn に dst の値が無い。IntersectRect の空の結果と同じ | 実機で測ったとき |
| `S-g8a-place` | プロジェクトとサンプルはインストールツリーの本来の位置(`C:\Program Files\Image-Line\FL Studio 21\Data\…`)に置く | `%FLStudioFactoryData%` = インストール先、で 5 つのサンプルと 2 つのプロジェクトが解決した(観測) | 実機の定義が示されたとき |
| `S-g8a-key-script` | Space = `fle_input_host_key(VK 0x20)` の KDOWN/KUP(同時刻) | ページの `fle_g3_key` が呼ぶ C 関数。JS の `KeyboardEvent.code` → VK は通っていない | ブラウザでの測定 |
| `S-g8a-wizard-dismiss` | 通常起動でウィザードを閉じる = (1036,561) のクリック + VK_RETURN(決定 0136・0141 の位置・経路) | 主窓が有効になる最短の入力 | ウィザードの別の閉じ方 |
| `S-g8a-nonzero` | G8c の合格 = リングに max\|x\| > 0 のブロック + 音高の段が 0.25 s ごとに 2 倍、周期 2.0 s | 実機の波形が無い | PM が別の条件にしたとき |

## 9. 出典(今回取得。Microsoft Learn)

- WM_COPYDATA(`.../dataxchg/wm-copydata`): wParam = 『データを渡す窓のハンドル』、lParam = COPYDATASTRUCT へのポインタ、戻り値 = 『処理したら TRUE、そうでなければ FALSE』、『受け手はデータを読み取り専用として扱う。lParam は処理中だけ有効』。
- SHAddToRecentDocs(`.../shlobj_core/nf-shlobj_core-shaddtorecentdocs`): `void SHAddToRecentDocs(UINT uFlags, LPCVOID pv)`、『pv を NULL にすると全部の使用データを消す』、『戻り値なし』。
- GetDpiForMonitor(`.../shellscalingapi/nf-shellscalingapi-getdpiformonitor`): `HRESULT GetDpiForMonitor(HMONITOR, MONITOR_DPI_TYPE, UINT*, UINT*)`、S_OK / E_INVALIDARG、『PROCESS_DPI_UNAWARE: 96』、『dpiX と dpiY は同じ』。
- 既存の頁の読み(決定 0136 ほか)に追加はない。

## 10. テストと検証

### 10.1 gencmp(生成 C の差。`tools/genc_diff.py` = 決定 0136 の道具、clean = HEAD d8aacf4)

| 対象 | 関数 | 本体が変わった | 追加 / 削除 |
|---|---:|---:|---:|
| FLEngine | 62,326 | **1**(`f_593320` = `SHAddToRecentDocs` の thunk。『unsupported』から `fle_imp_SHAddToRecentDocs()` の呼びに) | 0 / 0 |
| m1(dsp_ippv2)/ m2(Blend2D)/ m3(QFC)/ m4(freetype)/ m5(ILTools) | 25,370 / 3,093 / 6,723 / 1,747 / 12,931 | 0 / 0 / 0 / 0 / 0 | 0 |

先頭のプロトタイプ行(`void fle_imp_…`)が 605 → 607 行、`common.c` のディスパッチ表・応答表が 2 行増えた(全ユニットの先頭に API のプロトタイプが入るので、ユニットのテキストはすべて変わるが、オブジェクトコードは変わらない)。FLEngine の C は 225,546,487 B、SHA-256 `b27a6ba94635278c1c40b83198f5c4b2178e773bec09b92419b6be8869601b13`(HEAD の木は 225,546,059 B)。

### 10.2 FRUITY 無効の a4(`a4-fruity.sh off`、独立 2 回、この木)

| | OUT off1 | OUT off2 | HEAD の木(clean) |
|---|---|---|---|
| init_count | 942 / 942 | 942 / 942 | 942 / 942 |
| run_rank | 895 / 895(returned) | 895 / 895 | 895 / 895 |
| functions | 5,310 | 5,310 | 5,310 |
| entries | **1,009,731** | **1,009,731** | **1,009,721** |

**+10 の理由(実測)**: `GetDpiForMonitor` の行だけを外した対照の木で **1,009,721 に戻った**。行があると初期化の中で遅延ロードの `GetProcAddress("GetDpiForMonitor")` が成功し、ゲストが DPI の分岐でその先の lifted 関数を 10 個余分に踏む(読み。どの関数かは調べていない)。ほかの値は変わらない。

### 10.3 FRUITY 有効 on-seh(最終構成、独立 2 回 + HEAD の木 1 回)

FRUITY 有効の最終構成(`a4-fruity.sh on-seh`、独立 2 回 + HEAD d8aacf4 の木 1 回。3 本を同時に走らせた)。`logs/a4/`:

| | seh1(この木) | seh2(この木) | seh-clean(HEAD d8aacf4) |
|---|---|---|---|
| 終了 | `limit: wall time 900 s`(harness 終了 4。壁ではない。決定 0138 と同じ) | 同 | 同 |
| 最初の非背景画素(プログラム開始からの入口、lid 13) | 43,930,575 | 43,930,575 | 43,930,563 |
| CreateFruityInstance からの入口(900 s の時点。機械の速さで変わる) | 84,434,136 | 84,573,881 | 82,995,234 |
| 新しい関数 | 6,717 | 6,717 | 6,717 |
| 呼ぶ前の入口 | 1,052,550 | 1,052,550 | 1,052,540 |
| gen / build | 21 s / 116 s | 20 s / 117 s | 20 s / 118 s |

- a4 の出力は面の SHA を出さないので、**3 本が作った -O0 計装の native を、台本 `base.g3`(最初の idle で `snap`)で別に走らせて**比べた(`logs/a4/default-idle-*.txt`)。ファイルを与えない既定の起動: lid 13 `0133698f232bcb082545f18fa9a99919f0f2813bb46f7a0fa070a561aa6ad14b`、lid 48 `1223c73ff0b5451d013cf8e76bf3f4e387c07000b2c1dc152e49031a030b3018`、窓表 116 行の SHA `8769cd65…`、窓(TWelcomeWizard 有効 / 主窓 disabled / TApplication disabled)が**この木と HEAD の木で一致**。入口は 53,873,643 と 53,873,631(**+12**)。この木の native -O2 で仮想 5 s・8 s 進めた収束後の lid 13 は `eb4443d24f0f96377049588de9523a8ec5baf107fa7966c5ff62bc6ce2fb350a`(PM が決定 0142 の値として知らせたものと同じ。HEAD の木の収束後は私は測っていない)。
- **決定 0138 の値(idle `721b792e…`、収束 `7ea69083…`)とは一致しない**: 決定 0142 が窓 DC の可視領域を変えたため(PM の通知どおり)。この木の既定の挙動が HEAD の木と変わらないことは、上の対で確かめた。
- 入口の +10(呼ぶ前)と +12(idle・最初の画素)は `GetDpiForMonitor` の行によるもの(§10.2)。

### 10.4 変異検査(`tools/mutate.py`: runtime のコピーの 1 か所を書き換え、`wall_fr19` が落ちることを確かめ、対照のコピーが通ることを確かめる)

`tools/mutate.py`: runtime のコピー(`FLE_FR19_RT`)の 1 か所を書き換え、`wall_fr19`(13 件)が落ちることを確かめ、対照(無変更のコピー)が通ることを確かめる。`logs/mutate-final.jsonl`。

| # | 変異 | 結果(落ちたテスト数) |
|---|---|---|
| M00 | 無変更(対照) | 通る(13 / 13) |
| M01 | cbData から NUL の 1 単位を落とす | 検出(4) |
| M02 | dwData を 12(URL の形)に | 検出(4) |
| M03 | wParam を受け手の窓に | 検出(4) |
| M04 | サロゲートの上位を 0xD400 に | 検出(2) |
| M05 | 冗長な 2 バイト形を受け入れる | 検出(1) |
| M06 | サロゲートの符号位置を受け入れる | 検出(1) |
| M07 | パスの上限を 1 単位手前に | 検出(1) |
| M08 | 待ちの上限を 9 に | 検出(1) |
| M09 | 『T』で始まるクラスを主窓とみなす | 検出(4) |
| M10 | 処理済みを 1 のまま返す | 検出(2) |
| M11 | 96 DPI を 120 に | 検出(1) |
| M12 | MDT の型 3 を受け入れる | 検出(1) |
| M13 | NULL でないモニターハンドルをすべて受け入れる | 検出(1)(最初の版は未使用の引数で**コンパイルエラー**になり、全件が落ちて偽の『検出』だった。コンパイルが通る変異に直して取り直した) |
| M14 | SHARD のどの形も受け入れる(停止しない) | 検出(1) |
| M15 | `DefWindowProcW(WM_COPYDATA)` が TRUE | 検出(1) |
| M16 | `GetCommandLineW` が `FLE_CMDLINE` を無視 | 検出(2) |
| M17 | ASCII の検査が 0xC3 を通す | 検出(1) |
| M18 | 要求番号が戻る | 検出(1) |
| M19 | 空の `FLE_CMDLINE` を有効とみなす | 検出(1) |
| M20 | 処理済みのスロットを再利用しない | 検出(1) |
| M21 | メッセージを窓の所有スレッドでなくスレッド 0 に送る | **最初の版では生存**(テストの主窓がスレッド 0 の所有だったため等価)。シナリオ `open_other_thread`(主窓を別のゲストスレッドが持つ)とテストを足して**検出(1)** |
| M22 | 未知の要求番号を『待ち』と返す | 検出(1) |
| M23 | `GetCommandLineA` が `FLE_CMDLINE` を無視 | 検出(2) |
| M24 | `UnionRect` の両空が (1,1,1,1) | 検出: `a9_a10` の `rect_functions_match_the_model_on_a_grid_of_small_values`(runtime のコピーを持つ木のコピー + `cargo test` で) |

24 件中 24 件検出、生存 0(M21 は強化して検出、M13 は取り直し)。

### 10.5 全体テスト

**1 回目の通し(HEAD aee8877 + 差分 v4。`logs/fulltest-run1-failed/`)**: `cargo test --workspace --no-fail-fast --offline`、2026-11-02 06:27:01Z〜07:13:33Z(46 分 32 秒)、**140 result、2,056 passed、3 failed**。落ちた 3 件は**どれも私の行の追加が既存の固定と衝突したもの**(私が事前に grep していれば見つけられた。§13): ①`monitor.rs::the_rows_are_in_the_table_and_declared_in_the_header`(決定 0065 の『GetDpiForMonitor に行は無い』)②`wall_s.rs::shcore_has_only_the_get_process_dpi_awareness_row`(『GetDpiForMonitor は行なし』)③`vcall_thunk_dll.rs::after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported`(未対応 thunk の数 227 が `f_593320` の分で 226)。3 件の期待を直した(決定 0136・0139 までの流儀: 『その決定が答えた』と注記)。

**最終の通し(HEAD 7b82424 = aee8877 + 文書のみの差、+ 差分 v5(22 ファイル)。`logs/fulltest-final/`)**: `cargo test --workspace --no-fail-fast --offline`、環境変数は `docs/investigations/verify-0127-0129/tools/fulltest.sh` と同じ(`logs/fulltest-final/env-passed.txt`)、cgroup `MemoryMax=8G` `MemorySwapMax=0`、2026-11-02 07:14:23Z〜08:00:19Z(**45 分 56 秒**)、終了 0: **140 result、2,059 passed、0 failed、0 ignored**(`logs/fulltest-final/test-result-lines.txt` の 140 行を足した値)。直した 3 件を含めて全件通った。件数の筋: 1 回目も 140 / 2,056 + 3 = 2,059(直した 3 件の期待は件数を変えない)。**基準(計算。HEAD の木そのものは測っていない)**: 決定 0136 の基準 135 result・1,984 passed に、決定 0139(+1 result・+18、`wall_fr15`)、0140(+1・+17、`wall_fr16`)、0142(+1・+20、`wall_fr17`)、0143(+1・+7、`wall_fr18`)を足すと **139 result・2,046 passed**(各決定の報告の増分の和。決定 0140 は 136/2,001、0142 は 136/2,004、0143 は 136/1,991 で、それぞれ基準が違う)。この木 = 139 + 1 result(`wall_fr19`)、2,046 + 13 件 = 2,059 と合う。**HEAD の木の全体テストは別に走らせていない**(基準の数は計算)。

## 11. 止めた理由

PM の停止条件 **(a) G8c に達した**。壁 4 件は小さい応答(API の行・既定動作・境界の値)で、大きなモデル・lift の穴・PM の判断値の壁には当たっていない(fe-lift / fe-decode は変更していない)。

## 12. PM の判断点

1. **B はウィザードが出ている間は無視される**(エンジンの前提条件)。アプリの『ファイルを開く』は、(a) 起動時に A で渡す、(b) 通常起動ならウィザード/GDPR を閉じてから B を送る、(c) 『busy なら待つ』を `fle_appfile_state` で見せる、のどれにするか。
2. `fle_app_open` / `fle_app_open_state` は export したが**ブラウザ(wasm)側は未実装・未測定**。ファイルをブラウザから VFS に入れる `fle_app_put_file` は**作っていない**(PM の決定の範囲外)。
3. 遅延ロードの欠落 → `RaiseException(0xC06D007F)` → unwinder が `ControlPc` で停止、は**直していない**(別の欠落でも再発する)。調べるか。
4. チャンネルラックの名前ボタンが最下行だけ描かれる件(§3.2)を調べるか(決定 0142 の可視領域か、描画のモデルか)。
5. `wall_fr18.rs`(依頼)→ `wall_fr19.rs`(決定 0143 が使用済み)の改名。
6. S- 選択(§8)を仮のまま採るか。

## 13. 限界・規則違反・不備の自己申告

**限界(確認できなかったこと)**: 実機 Windows との照合なし。wasm・ブラウザ・Firefox/Safari は未測定(`emcc -c` で `fle_appfile.c`・`fle_env.c`・`fle_input.c`・`fle_user_nowin.c` がコンパイルできることだけ確認)。音の正しさ(実機の書き出しとの比較)なし。G8c-RT(実時間の連続性)なし。oggio(G8d)・プラグイン(G9)なし。`KeyboardEvent.code` → VK の経路は通っていない。作者の絶対パスを指す .flp の解決は観測していない。`GetCommandLineA` は ASCII のみ。ウィザードが A で出ない理由は未確認。入口数は gdb の breakpoint の読み(±1)。

**規則違反・不備**: ①存在しない `bash` ツールを 1 回呼んだ(エラーで終わり、実行されていない)。②cargo を `CARGO_TARGET_DIR` なしで回して古い fe-lift で生成し、決定 0140 の行が抜けた版を作った(気づいて作り直した)。③前景のコマンド(`inc-build`)が 20 分の timeout を超えて 1 回中断された(プロセスは止まった。再開した)。④出力を絞らないコマンドを数回走らせ、長い出力を読み込んだ。⑤`sleep` / `nohup` / 末尾 `&` / `pgrep -f` / while ループは書いていない。⑥git の変更系操作なし、commit なし。`rm` は `lift/out-g8a1/` の中だけ。`/tmp` 直下には書いていない(`/tmp/opencode/g8a1/`)。⑦ツールの出力・取得ページに依頼者の指示ではない文は**見当たらなかった**。⑧**全体テストの 1 回目で 3 件落ちた**: 行を足すときに、その行が『無い』ことを固定している既存テストを grep しなかった(`monitor.rs`・`wall_s.rs`・`vcall_thunk_dll.rs`。しかも途中報告 4 に、grep で確認する前に『失敗の行なし』と書いてしまった。同じ日に訂正を送った)。直して全体テストをもう 1 回通した。⑨同時に別の担当のビルドが走っていた(メモリの余裕は `free -m` で確認、最小でも 9 GB)。

## 14. 再現

`MERGE-NOTES.md` と `tools/`。要点: HEAD d8aacf4 の `git archive` に `g8a1.diff` を `patch -p1` → `cd lift && cargo build --release -p fe-lift -p fe-pe` → `tools/full-build.sh N`(生成 → 10 MB の単位 → native -O2 → リンク。`tools/env.sh` のパスを直す)→ 実行: `BIN=…/two-native-o2 SEED=…/vfs-seed-g8.tsv FLE_CMDLINE='"C:\Program Files\Image-Line\FL Studio 21\FL64.exe" "C:\Program Files\Image-Line\FL Studio 21\Data\Templates\Minimal\Basic\Basic.flp"' FLE_G3_SCRIPT=scripts/dump.g3 tools/run.sh LOG`。入力のファイルは `g8_extract.py` で抽出(`logs/inputs-sha256.txt`)。
