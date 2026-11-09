# S3a の掃引(O17、実スレッド版の Worker 数): 手順書と run の準備(ci1、仮の番号 0326)

**実際に実行した物(先頭)**: 道具のファイルの作成と、読み取り・構文・オフラインの確認だけ。Chrome・Firefox は 0 回、サーバーの起動は 0 回、clang・emcc・Wine は 0 回(この手順書の作成に clang は使っていない)。実行したのは: `bash -n` と `node --check`(全スクリプト)、`DRY=1` の実行(コマンドを印刷するだけ)、`scn-lint.mjs`(新しい 2 本のシナリオ、628 op、問題 0)、B2 bundle の `sha256sum -c`(通った)、`mk-s3a-assets.sh`(manifest の複製と symlink だけ)、`s3a-summary.py` を**過去の E11/E13 の 60 秒の run**(archive)で動かして表が出ること。**測定値は 0 件**。Chrome の合図(PM)があるまで run は始めない。

## 1. 何を測るか(PM の指示)

依頼者の所見 O17: 本番(協調 1 Worker)では参照プロジェクトの再生が音切れで、Track 1-5,7,13,15-17,19,20 だけを Solo にすると辛うじて実時間になる。実スレッド版(B2: `fl-engine-ci-8b2eb5d7`)の Worker 数 `FLE_WORKERS` を 2/4/8/16 に変えて、本番 E13 を対照に、2 つの条件で比べる。

| 条件 | 内容 |
|---|---|
| song | 参照プロジェクト(`serina1-serum2.zip`)の Song Mode の 5 分再生(全トラック)。`scn-s3a-song.json` |
| solo12 | 依頼者の『Solo 12 本』(Track 1-5,7,13,15-17,19,20)。opt1 が作る `solo12.flp` を、同じ名前 `serina1-serum2.flp` で zip に入れた物。`scn-s3a-solo.json`(`openZipProject` が `proj-solo12/` の zip を使う) |

対象 5 条件: `e13`(本番、協調 1 Worker、port 5053 = 既に動いている)、`w2` `w4` `w8` `w16`(B2 の manifest の `engine.env` に `FLE_WORKERS=n` を 1 つ足しただけ。bundle の中身は同一、再 build なし。0312c §5c)。page は全条件で本番の page(`page-d0447902` = E13)。**B2 の既定(env なし)の挙動は未確認**(0312f: S3a の既定は n = 16、N = clamp(n−1,2,8) = 8 だが、B2 が同じかは確認していない)。測定の記録には `FLE_WORKERS` の値を必ず書く(run ごとの `out/s3a-*/` と表の condition 列)。

## 2. 指標と定義(`tools/s3a-summary.py` が各 run のファイルだけから出す。推定なし)

| 指標 | 定義 |
|---|---|
| 書き込み率 | ゲストが DirectSound の Unlock で書いたフレーム数 / (10 s × 44100)。`play` の印から 10 s の窓 × 最大 30。平均、最小、0.95 未満の窓の数 |
| 消費率 | ページの AudioWorklet が消費したフレーム(`consumed`)の同じ窓での率 |
| 音楽の位置 | 消費したフレーム / 44100 = 曲の位置(秒)。run の終わりの値(`music s / wall s`)。**曲の位置で揃えた比較**: 30 s 分の音楽が進むのに要した壁時計の秒(`wall_per30`)と、その逆数の実時間比 |
| underrun | AudioWorklet の underrun の量子数(1 量子 = 128 フレーム = 2.90 ms は **Web Audio の render quantum という仮定**)、episode 数、play 後に伸びた最長の連続(ms)= 聞こえる PCM の零の連続 |
| Unlock の間隔 | 連続する Unlock の時刻差の p99 と最大(ms) |
| 遅れの切り捨て | 仮想時計の `lagCuts`(play 後の増分) |
| Worker の CPU とコア数 | 5 分の区間(`p3-play`)の DedicatedWorker スレッドの CPU 秒の合計、コア数 = その / 区間の壁時計の秒、各 Worker の CPU 秒(新しい `workers` 配列)のうち壁時計の 10 % 以上の数 |
| 止まったか | `report.json` の `stopped` / `error` / `state`。止まった run は counted にしない。名前つき stop の本文は run のログ(`out/<id>.log`)と `out/<id>/` |
| Export の sha | 別の段(§6)。期待値 `381cdb5a…f979`(22,863,828 B) |

**counted の条件**(これまでと同じ): 開始時の load ≤ 8、I/O 圧 avg10 = 0、swap の入出力 0、他の Chrome 0 本、MemAvailable ≥ 8 GB(この機械は 16 コア、swap を 3〜4 GB 使っているので `vmstat` の si/so を run の直前に記録する)。条件を満たさない run は表に出すが counted にしない。

## 3. 準備(Chrome なし。済みの物と、これから要る物)

| 物 | 状態 |
|---|---|
| B2 bundle `~/work/mt3w-s1/ci-b2/engine/bundle/fl-engine-ci-8b2eb5d7`(wasm 142,986,341 B) | あり。`SHA256SUMS` は通った。**mt3 が消すと run が始まらない**(`check` で分かる) |
| `assets-b2w{2,4,8,16}/`(manifest の `engine.env.FLE_WORKERS` だけ違う。bundle は symlink) | **作成済み**(`tools/mk-s3a-assets.sh`)。manifest の差は 3 行(`FLE_WORKERS` の追加) |
| 計画 `out/s3a-plan.tsv`(2 round × 5 条件 × 2 シナリオ = 20 run) | **作成済み**(`tools/s3a-plan.py 2`)。条件の順は round ごとに回転(e13,w8,w2,w16,w4 / w4,w16,w2,w8,e13)で、機械の変動が 1 条件に偏らない。1 条件の 2 シナリオは続けて走る |
| 4 つの B2 のサーバー(5072 w2、5074 w4、5078 w8、5079 w16) | **未起動**(Chrome の合図の後で起動: `run-s3a.sh servers` の 4 行を各々 background で `exec`)。e13 は 5053 で既に動いている |
| 観測器 `node tools/chrome-count.mjs out/chrome-count.tsv 2000` | 動いている(最終行が新しいこと。`run-s3a.sh` が確かめる) |
| `proj-solo12/serina1-serum2.zip` | **未**。opt1 の `solo12.flp` を受け取ったら: `python3 tools/mk-solo-zip.py proj/serina1-serum2.zip <solo12.flp> proj-solo12/serina1-serum2.zip`(元の zip の 49 個のメンバーのうち .flp だけを置き換える。ダミー置換で全メンバー一致を確認済み)。その後 `mk-s3a-assets.sh` をもう一度(`proj-solo12` の symlink を各 assets に足す) |
| 画面の確認 | 最初の run の `out/s3a-r1-*/s3a-mode.png`(プロジェクトを開いた直後の絵): **Song Mode になっているか、solo の状態**を人が見る(シナリオは Space を押すだけで、モードの切り替えはしない。**Song Mode の既定は未確認**) |

## 4. 実行(15 分刻みの枠。合図は PM)

1 枠 = `next 2`(同じ条件の song と solo12。1 run は起動 約 40 s + 再生 300 s + 後始末 ≒ 6.5〜7 分、2 run で 13〜14 分。gate の待ちが入ると 15 分を超えることがある)。

```
cd ~/work/signature-studio-develop/lift/out-ci1/kit
bash tools/run-s3a.sh check                  # 合図の前に。ファイル・manifest・サーバーの確認。Chrome なし
# 合図の後、最初の枠の前に 4 つのサーバーを起動(各行を background で): bash tools/run-s3a.sh servers
bash tools/run-s3a.sh next 2                 # 1 枠。計画の次の todo 2 本(例: round 1 の e13 の song と solo12)
bash tools/run-s3a.sh summary                # 表(counted の run と条件別の平均)
```

`next` の gate(どれか落ちたら run は始まらず、行は todo のまま): 他の Chrome が 0 でない、chrome-count の観測が 15 s より古い、swap の入出力が 0 でない、MemAvailable < 8 GB、サーバーが落ちている、`wait-quiet`(load ≤ 8・I/O 圧 0・swap 0)が MAXWAIT(15 分)内に来ない。gate の読み(free、vmstat、wait-quiet の行)は `out/steps.log` と各 run の `gate.txt`。

最小の計画 = round 1 の 10 run = 5 枠 ≒ 70〜75 分。余力があれば round 2(順序を逆にした 10 run)。1 round の結果が条件間で動かないなら、round 2 は ABBA の追加として使う。

## 5. 読み方(判断は PM)

- 条件ごとに song / solo12 の counted 行を並べ、**e13 に対する各 n の差**を見る。基準(PM の新しい関門): 5 分再生で PCM の零の連続(`longest run`)・書き込み率(0.95 未満の窓の数)・underrun・Unlock の間隔が協調より悪くならないこと、present の rate が協調より落ちないこと(この表には present の項目は無い。`out/s3a-*/report.json` の `timeline` と `snaps` の `presents` から取れる: 必要なら列を足す)。
- コア数(`cores`)が n に応じて増えるか、頭打ちか(ホストの 16 コアに対して)。`Workers >= 10 %` が n より少なければ、Worker を増やしても使われていない。
- 止まった run(名前つき stop、`W%u has waited 3 s for the lock` など)は、その n で FL が最後まで走らないことを示す。n = 16 は V8 の isolate が増えるので、MemAvailable の落ち方も `steps.log` で見る。
- 1 round の run 間の揺れは**まだ無い**(同じ条件の 2 回目が無い)。差が小さいときは round 2 の ABBA を足してから言う。

## 6. 任意の段: Export の sha

`bash tools/run-s3a.sh export w8`(条件を変えて):約 20 分/回。期待値 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`(22,863,828 B)。実スレッドで音の内容が変わらないことの確認。全条件では 80 分かかるので、**掃引の後に、通った条件のうち 1〜2 本(w8 と成績の良い n)**を提案する。

## 7. 限界・未確認

1. B2 の既定(env なし)と `FLE_WORKERS=1` の腕(協調と同じと 0289 が言う)は計画に入れていない(PM の指定は 2/4/8/16 と E13)。入れるなら `mk-s3a-assets.sh` に `n=1` を足す。
2. solo12 は opt1 の `solo12.flp` に依存する。Solo の状態が FL の再生に反映されていること(音が出るトラックの数)を `s3a-mode.png` で人が確かめる。
3. `FLE_PROCESSORS` は固定しない(n と N の効果は分離されない。0312f)。分離するなら別の掃引。
4. 音楽の位置は DirectSound の消費フレームから出した値で、FL の再生位置そのものではない。
5. 道具の変更(`sdriveg2-lat.mjs`): `openZipProject` に `dir`(zip のある /assets/ 下のディレクトリ、既定 proj)、`cpuDiff` に各 Worker スレッドの CPU 秒(`workers`)を足した。どちらも既定の挙動は変わらない(`node --check` と lint のみ)。
6. 実行した確認は、過去の 60 秒の run への `s3a-summary.py` の適用だけ。5 分の run の表は未出力。

## 8. 道具

`tools/run-s3a.sh`(plan / servers / check / next / summary / export)、`tools/s3a-plan.py`、`tools/s3a-summary.py`、`tools/mk-s3a-assets.sh`、`tools/mk-solo-zip.py`、`tools/scn-s3a-song.json`、`tools/scn-s3a-solo.json`、`tools/sdriveg2-lat.mjs`(上の 2 点)。コピーは `lift/out-git/ci1-results/0326/tools/` と `0277/kit/tools/`。

## 9. 追記(2026-11-07 08:4x、PM の指示): 掃引は保留。Worker 数だけでは割れない見込み

**PM の指示**: S3a の掃引は保留に変える。理由(PM の info: alpha1 の読み、mt3 の読みと合わせて): runtime が `GetSystemInfo` で『プロセッサ 1 個』を答えるので、FL のミキサーのスレッド数がそれに従う可能性があり、B2 で `FLE_WORKERS` だけを増やしても割れない見込み。alpha1 が `FLE_NUM_CPU`(= Worker 数)に揃える差分を 09:00 までに作り、**RT-A の bundle(09:40 ごろ)でその軸も含めて掃引する**。それまでは準備(手順書の整理、solo12 の zip)に留める。

**ci1 が確かめた事実(読みだけ。実行 0)**:

| 確かめた物 | 内容 | 場所 |
|---|---|---|
| `GetSystemInfo` の応答 | `dwNumberOfProcessors` = 1、`dwActiveProcessorMask` = 1(コードの注釈は『run が単一の guest thread だから。選択であり、ホストのコア数を返すと機械ごとに出力が違う』) | `lift/runtime/fle_sys.c` の `fle_imp_GetSystemInfo`(`nproc = 1`、`mask = 1`) |
| `GetLogicalProcessorInformation` の応答 | 論理プロセッサ 1 個(`FLE_SYSINFO_PROC_MASK` = 1。『setting』と注記) | `lift/runtime/fle_sysinfo.h`・`fle_sysinfo.c` |
| `FLE_NUM_CPU` | runtime に該当の文字列は無い(grep 0 件)。alpha1 の差分の入口になる値 | `lift/runtime/*.c *.h` |

読み: Worker の数(`FLE_WORKERS`)は host 側の設定で、guest が数えるプロセッサは 1 のまま。FL が混合のスレッド数を CPU 数から決めるなら、Worker を増やしても guest のスレッドは増えない。これは lift の忠実性の方針でいう (b) 外部 API(GetSystemInfo / GetLogicalProcessorInformation / affinity の mask)の応答が『1 個』という選択であることの帰結の疑い。確認は RT-A の bundle での掃引(軸: `FLE_NUM_CPU` × `FLE_WORKERS`)で行う(**未確認**)。決定性(Export の sha が機械で変わらないこと)との両立は alpha1 の差分の設計(CPUID の SSE2 固定と同じ『run の値として渡す』の型)に依る。

**進めた準備**:
- `proj-solo12/serina1-serum2.zip` を作った: `python3 tools/mk-solo-zip.py proj/serina1-serum2.zip ~/work/opt1-wine/variants/solo12.flp proj-solo12/serina1-serum2.zip`(解凍後の 49 メンバーは opt1 の `variants/solo12/serina1-serum2-solo12.zip` と内容が同一〔メンバー名・大きさ・sha256 を python で比べた〕、zip 自体の sha256 先頭 `a0db9ed966f03e80`)。`solo12.flp` の sha256 は `8bfab468…0c085`(opt1 の `variants/solo12/SHA256SUMS` と同じ)。
- 掃引の assets(`mk-s3a-assets.sh`)の作り直しは、RT-A の bundle が来てから(軸が増えるため)。`s3a-plan.py` の条件表を `FLE_NUM_CPU` の軸に拡張する必要がある(**未実施**)。
- 本節の時点でも **S3a の測定は 0**。
