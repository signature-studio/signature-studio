# ci1 → PM 決定 0271 報告1: Firefox 158 での本番 bundle `fl-engine-ci-ff5ee429`（Worker a3fc8c43 のページ）の実測 — **P1: Firefox の first idle は他者のディスク I/O に強く依存する（静かな窓 4.6 s、I/O が混む窓 16 s 前後・最大 36.9 s）**

## 先頭の事実（実エンジン・実ブラウザ。依頼者の機械の値は 0 件。代役・再生・推測の成果物なし）
- **測定条件**: 08:15 のサーバー再起動の後の機械（16 CPU・RAM 31 GB。再起動前は MemAvailable 75 GB 台だった）。bundle は fl-web-assets から読み取りで取得（sha256 全一致）、局所の vite preview で配信。各 run は新しい Firefox プロファイル（cold）、`entry-check.mjs`（既定の入口、単独）。各 run の開始時と終了時の `load` と `/proc/pressure/io`（avg10 / avg60 / avg300）を記録（`data/first-idle-runs.json`）。
- **Firefox の first idle の中央値**（ff5ee429）: 静かな窓（I/O 圧 avg10 が開始・終了とも 1 未満、load 4〜8）で **4.63 s**（n=19、4.26〜5.17）。他者の I/O が混む窓（avg10 ≥ 10）で **16.07 s**（n=8、4.68〜**36.85**）。Chrome 156（同条件、ff5ee429）は静かな窓で **5.21 s**（n=15、4.71〜5.91）。静かな窓なら Firefox・Chrome とも P1（10 s）を満たす。
- **load ≤10 の組**（依頼の (3)）: 上記の静かな窓がそれ（load 4〜8。3 つの組の比較は下）。load の値だけでは決まらない: load 6〜7 でも I/O が混めば Firefox は 17〜37 s（`first-idle-runs.json`）。→ **load より I/O 圧を併記する**。
- **遅延の出所**（ページ側の性能マーク `timeline` と `assets` から、Firefox の中央値）:

| 段（ms） | Firefox 静か（n=15） | Firefox 混む（n=10） | Chrome 静か（n=12） |
|---|---|---|---|
| ページの script 開始（page_script） | 140 | **1,158** | 47 |
| manifest 取得（manifest_loaded） | 165 | **1,175** | 62 |
| 最初の取得の進捗（download_start） | 335 | **7,689** | 90 |
| 取得した資産の応答ヘッダー（wasm の `headers`） | 62 | **3,774** | 13 |
| wasm のコンパイル完了（compile_done） | 1,336 | 8,219 | 906 |
| 全資産の取得完了（download_done） | 1,335 | 8,667 | 1,078 |
| Worker の準備〜instantiate（worker_ready→wasm_instantiated） | 1,418→1,433 | 8,728→8,746 | 1,154→1,180 |
| エンジンの初期化（main_start → 最初の idle） | **3,170** | 3,248 | **3,909** |
| **first idle** | **4,606** | **11,999** | **5,102** |
  - wasm の取得内訳（Firefox 静か）: 応答ヘッダー 62 → 読み 654 → ハッシュ 137 → 複製 62 → **OPFS への書き込み 1,017（クリティカルパスの外: 取得完了後に背景で書く）** → コンパイル完了まで合計 1,085。混む窓では headers が 3.8 s（最大 24 s）、読みが 2.0 s に伸びる。**エンジンの初期化（約 3.2〜3.3 s）は混む窓でも変わらない**（CPU 側は無関係）。
  - 4 つの資産（wasm・DLL・seed・glue）の `headers` は混む窓で**ほぼ同時刻**に返る（例: 23,981 / 23,986 / 23,986 / 23,986 ms）→ 4 本の取得が同じ待ちに詰まっている。サーバー側は curl で TTFB 1.3 ms（19.6 MB を 57 ms）で、サーバーが遅いのではない。
- **cold と warm**: この報告の run は全て cold（新しいプロファイル、資産は network）。warm（同じプロファイルでの 2 回目、OPFS の保存済み資産）は**未測定**（最終報告で）。
- **df0512b0 と ff5ee429 の対**（静かな窓、Firefox）: df0512b0 4.84 / 5.17 / 5.03 / 5.03 s（中央値 5.03）、ff5ee429 4.56 / 4.63 / 4.80 / 4.64 s（4.64）。Chrome: df0512b0 5.91 / 5.70 / 5.50 s（5.70）、ff5ee429 5.11 / 5.10 / 5.04 s（5.10）。新が 0.4〜0.6 s 速い。
- **2 本並列の first idle**（静かな窓、Firefox と Chrome を同時に 3 回）: Firefox 4.61 / 4.73 / 4.82 s、Chrome 5.40 / 5.46 / 5.57 s（単独より +0.1〜+0.3 s）。

## 原因の判定（(4)）
- **ページ側の作りではない**: 遅れる段は『ページの script が始まるまで』『資産の応答ヘッダーが返るまで』（ページが自分の仕事を始める前〜ブラウザの取得の入口）で、ページの同期処理（OPFS の書き込みはクリティカルパスの外、wasm のコンパイルは取得とストリーミング、エンジンの初期化は 3.2〜3.3 s で一定）には I/O 圧との相関が無い。app/src の修正案は無い。
- **ホスト + ブラウザ起動側の性質**（ツール側ではなく環境）: 同じ run の Chrome は I/O が混んでも劇的には伸びない（13 s が 1 回、他は 5〜8 s）が、Firefox は新しいプロファイル（`c5/ffprof` = 他の担当と共用の同じ LV のディスク）の作成・HTTP キャッシュ・Quota Manager・ネットワークの起動を、そのディスクの I/O 待ちの後ろでする。確認した相関: I/O 圧 avg10 が 0 付近なら 4.3〜5.2 s（19/19）、≥ 10 なら 4.7〜36.9 s（中央値 16.1 s）。因果の確認（プロファイルを I/O の混まない所に置く）は、tmpfs だと OPFS が作れない（'Could not create actor'）ため**できていない**。HTTP キャッシュのディスクを切る（`browser.cache.disk.enable=false`）、ネットワークの背景通信（プロキシ、captive portal、Nimbus など）を切る設定では、混む窓での遅れは消えなかった（3 + 3 回、改善は I/O が静かになったときだけ）。
- **依頼者の機械（Windows の Firefox 159）で起きるか**: **未確認**（この機械の値のみ。ディスクを他の作業が占有していない通常の環境では静かな窓の値 4.6 s に近いと推測するが、未測定）。

## 他の項目の状況
- (1) Mixer ドラッグ（df0512b0 と交互）・ピッカー定常 CPU・F9 の帯: 再起動前（load 20〜28）に Firefox で 4 往復を取った: Mixer ドラッグ Worker CPU/1 移動 — 1920: ff5ee429 **10.62** ms（9.83〜12.97）、df0512b0 11.01（8.32〜13.9）; 3240: 9.11（8.54〜10.75）、8.83（8.7〜9.62）。**差は見えない**。ピッカー定常 CPU 330 / 363 ms/s（範囲が重なる）。Chrome（6.4〜6.8 / 4.9〜5.1 ms）の約 1.6〜1.8 倍。F9 の帯は Firefox のゲスト画面が 1920x994（viewport の高さ 1116 − 122）で、帯の座標が Chrome 用のため**未算出**。**静かな窓で取り直している**（viewport 2260x1202 で 1920x1080 を得る。進行中）。
- (2) 入力遅延（gpu1 の方法）: Firefox で実行中（Firefox では 49 ファイルの一括アップロードが `AbortError` で失敗するため、プロジェクトを zip で開く経路に直した。zip の取り込みも 1 本の execute/async では失敗し、取得 → File → DataTransfer → input → change を別々の script にすると通る、という Firefox 固有の挙動を見つけた）。最終報告で。
- (4) beacon audio.detail と再生中の underrun: 最終報告で（first-idle の beacon には `audio.detail` が全て入っている）。

## 自己申告の逸脱
- 再起動前の Firefox の入力遅延ランは、openProject の 49 ファイルのアップロードが Firefox で失敗して途中で止まった（原因は上記）。
- 背景ジョブの待ちで `tail --pid` を使ったが、再起動で PID が消えていた（PM の連絡どおり、以後は ps の目視の PID のみ）。
- OPFS の分解測定（`scn-fftime.json`）は 2 回とも scenario が戻らず（Firefox の evalAsync が返らない）、強制終了した。結果なし（因果の確認はできていない）。
- `c5/ffprof`（geckodriver のプロファイル置き場）を後片付けで消していたため、再起動後の最初の Firefox の run が geckodriver 起動失敗（ECONNREFUSED）で 7 回空振りした（再作成して撮り直した。その 7 回は集計に入れていない）。

## 成果物（`lift/out-git/ci1-results/0271/`）
`REPORT-0271-1-to-PM.md`、`data/first-idle-runs.json`（段ごとの値・I/O 圧つき）、`data/first-idle-all.json`（全 run の first idle と開始・終了の load / I/O 圧）、`tools/`（idle-cmp.sh、stages.py、ab-ff*.sh、run-lat-ff.sh、tools-lat/ ほか）。
