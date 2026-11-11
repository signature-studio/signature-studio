# 決定 0289(仮): 実スレッド版(UI = Worker 0、ミキサー = Worker 1)の最初の go/no-go 測定 — E9 の木(main 38efb398)に mt3 の stage-B 足場を載せ直した使い捨ての木

状態: **仮の決定・測定(出荷しない、製品の木・PM の木には書かない)**。作業者 `gpu1`(成果物名は gpu7)、2026-11-06 JST 12:40〜。依頼: PM 12:50(0289)、12:58(Unlock の first の差の度数を 10 s 窓ごとに)。基準: **main 38efb398**(E9 の push)。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=解釈、**未確認**。出典なしの選択=`S-gpu7-N`(§10)。**この機械(16 vCPU の Zen 5 VM、実 GPU なし = SwiftShader、headless Chrome 156、共有で load 1.4〜15。各 run の load は表に併記、>12 は『参考』)の値で、依頼者の機械(GTX 980)の値は 0 件。**

## 冒頭の三点

1. **何を実行したか**: 実 Chrome 156 headless で**実エンジン**(wasm)を動かした測定だけ(代役・再生・モックなし)。(a) main 38efb398(E9)に mt3 の stage-B 足場(実スレッド版: 2 Worker、giant lock、thread-local のレジスタ)を載せ直した**使い捨ての木**を作り(§1)、冷間 CI 段階で bundle `fl-engine-ci-377215a9` を作って(§2)、`FLE_WORKERS=2` で**実際に 2 本の Worker で参照プロジェクトを 60 s 再生**した。(b) 本番の E9 の bundle(072efa6a)と**同じ page・同じ driver で交互に 3 往復**(§3)。(c) 同じ実スレッド bundle を `FLE_WORKERS=1`(協調・仮想時計)で動かす 1 本(§4、税の内訳)。(d) FL の CS・GIL・Sleep(1)・DirectSound の個数を見るストップ統計の build の 1 本(§5)。(e) 入力の真の T を実スレッドと E9 協調で 1 本ずつ(§7)。**推測・読みが入る所は『読み』と書いた**。**音声はミキサーの Worker が page の ring を使えるように glue を post-link で補修した測定専用の物**(§2、製品形ではない)。
2. **結論**: 下の『結論』。
3. **未確認**: 依頼者の機械(実 GPU)の値(0 件)、**5 分の再生(Chrome の窓が取れず未取得)**、入力の真の T の交互(1 本ずつ)、ストップ統計の 60 s の系列(1 つの 10 s 窓だけ)、実スレッドの FL の UI の tick(スライス)そのもの、Firefox、FL の並列ミキシング(FLE_PROCESSORS > 1)、実機 Windows の DirectSound の列、ゲストの lead の実行値、vitest / test.sh / a4(使い捨ての木で未実施)。

## 結論(go/no-go 1 の判定: **音楽の軽い区間では実スレッドが効くが、重い区間で 1 コアの壁。S2 以降にはまだ進まず、E11 の上の再測定〔5 分必須〕から**)

1. **P3: 音楽の軽い区間では実スレッドが効くが、重い区間で 1 コアの壁に当たる(E9 の速度で)**(§3c・§4。**info 4 の『同じ』『税 +40〜70%』は音楽の位置の交絡で訂正**): 同じ音楽位置(10〜30 s)で、実スレッドは書き込み(重ならない frame ÷ 壁 s)が **0.75〜0.90、E9 協調は 0.65〜0.78**、実時間(Unlock 9.4〜10.9/s)で進む。ミキサー側の CPU / Unlock は 実スレッド 76〜97 ms、協調の按分 63〜76 ms = **税 +20〜30%**(0〜10 s では 0〜5%)で、**GIL・TLS・アトミックのコードの費用ではない**(同じ bundle を `FLE_WORKERS=1` で動かすと E9 協調と同じ)。しかし音楽は後半ほど重く、**実スレッドは音楽 50〜60 s で 118〜128 ms / Unlock(r = 1 に 1.27〜1.38 コア)**に達して 6.7〜7.3 Unlock/s に落ちる。**r = 1 を保つには最も重い区間で ≤ 92.9 ms / Unlock が要り、E9 の実スレッドは約 −27% 足りない**(E11 のミキサー −15% でも約 −12% 足りない)。協調は 60 s で音楽 37 s までしか進まず、重い区間に到達しない。**5 分の再生は未取得**(曲全体の最も重い区間は不明)。CPU の合計(UI 0.6 + ミキサー 0.9 = 1.5 コア、他に SwiftShader 1.7〜2.5)は 2 コア予算に収まる。
2. **遅れの原因は待ちではなくミキサー自身の計算量**(§5、実測 1 窓): FL の Sleep(1) 12%(100 回/秒)、GIL の待ち 3.6%(CPU のスピンと重なる)、FL の CS の待ち ≈ 0(UI は再生中 CS を保持しない: UI の保持は再生前の 188 s だけ)、run-queue 待ち 3〜7%。= PM の問い (1)『待ちなら本命の梃子』— **待ちではなかった**。
3. **『3 点セット』(+3 → -1 → +1)**(§4・§9): 遅れた回に FL が書き込み位置の先へ飛び、埋め戻して書き直す。1 回の遅れで新しい frame は 2 ブロック分のうち 1 ブロック(約 93 ms)が無音、書き直しは無駄なミキシング 14〜20%。実スレッドは 12288 が 21%(約 23% の quanta が無音)、**60 s の中では純粋な 3 ブロック周期にはならない**(12288 は 4〜10% から 23〜31% へ時間とともに上がる。両腕とも)。契約の戻り値(play / write cursor)に逸脱は見つからない。
4. **P2 にも効かない(E9 の速度で)**: UI Worker は 54〜63% の CPU・73% は `WaitMessage`(FL が仕事を持たない待ち)で、UI の起床は 24〜37 回/秒。desk presents は 25〜36/s(協調 11〜14/s)で改善するが、**入力の真の T は協調より悪い**(File クリック 37.2 対 21.5 ms、Esc 18.0 対 4.3 ms。同じ page、交互ではなく 1 本ずつ。B +8 ms 前後)。
5. **この機械の SwiftShader**: 実スレッドで UI が 2.5 倍描くので、描画の CPU が +1.4 コア。ミキサーの run-queue 待ちは 3〜7% なので、これを除いてもミキサー thread が戻せる時間は小さい(読み、未測定)。
6. **推奨**: **S2 以降(音声 ring・export の包み・テストの作り直しなど約 17〜19 人日)には、まだ進まない**。**E11 の上の再測定(go/no-go #2)を、5 分の再生(重い区間)を必須にして行う**: E11(0285。opt1 の実測で純再生の書き込み率 0.704 → 0.888、alpha1 の 0296 でミキサー −15%)で実スレッドのミキサー側の CPU / Unlock は 最も重い区間で 118〜128 → 約 100〜109 ms(推定)、**r = 1 の必要時間は 1.08〜1.17 コアで、なお 1 コアを超える**(軽い区間 10〜40 s では 0.7〜0.9 コアで足りる)。= **E11 だけでは足りず、重い区間のさらに約 −12% が要る**(ミキサー側の更なる梃子、または FL の並列ミキシング `FLE_PROCESSORS > 1` の検討。mt3 の 0234s は n を増やしても総 CPU が増えなかった)。**税の内訳のうち『コードの費用』は除外できた**(w1)。残る税(同じ位置で +20〜30%)は実スレッド自体・同居スレッド・環境(§4、§11)。go/no-go #2 の計画と E11 への rebase の dry-run は §11。
7. **未確認 / 取れていない**: 冒頭の三点の 3 に同じ。5 分の再生と T の交互は Chrome の窓(ci1 の counted run が優先)が取れず**未取得**。


## 1. 載せ直し(rebase): 足場を E9 の木(38efb398)に当てた結果と、手で解いた 6 hunk

木: `lift/out-gpu2/srcRT` = `git archive 38efb398 -- lift app ci tools .github docs/investigations/prune1` + `docs/investigations/mt3/patches/mt3-stage-b-prototype-onto-4f054ecd.diff`(`patch -p1`、log `logs/rt-patch-stageb.log`)。**58 ファイルのうち 54 は offset のみ(fuzz 0)、4 ファイルの 6 hunk が当たらなかった**(0283b の dry-run は 10 hunk と数えたが、実際の `patch` では 6。`fle_thread.c` の 2 hunk のうち 1 つは実適用で当たった)。

| ファイル / hunk | 当たらなかった理由 | 解き方 | L1c / alpha10 との関係 |
|---|---|---|---|
| `fe-wasmgen/src/lib.rs` hunk 4・5(`Ctx { … }` のリテラル 2 か所) | 0254/0270/0270b で `Ctx` に `x87_ld` `x87_st` の欄が増え、行の文脈が違う | 両方のリテラルに `mx_tls: conf.hybrid_mx_tls,` を `mx_buf` の直後に足した(`lower.rs` の `Ctx.mx_tls` は足場で当たっている) | なし |
| `harness/fruity_idle.c` hunk 2(`fle_thread_idle_hook`) | L1c が `fle_input_ring_poll` / `RING-POLL` のブロックを同じ場所に足した | `fle_idle_wait` の宣言を 4 引数(`wake_addr, wake_seq`)にし、フックの待ちを `fle_gil_release()` / `fle_gil_acquire()` で挟んだ。`#ifdef FLE_GIL` で `fle_idle_wake_word/seq` を渡す。L1c のブロックは無変更 | **L1c の `fle_input_ring_poll()` は GIL を持った `schedule_ex` の中で呼ばれる**ので包まない。`fle_js_input_poll/done` は `fle_imp_*` ではない普通の同期 import |
| `harness/fruity_idle.c` hunk 3(`fle_g3_*` の export) | alpha10 が `fle_g3_taskbar_iconic` / `fle_g3_taskbar_syscommand` を足した | 足場の 8 export(mouse/key/display/pending/ticks/clock_to/snapshot/probe_timer/timers)の `fle_gil_enter()` / `fle_gil_leave()` の包みを写し、**新しい 2 つ(taskbar)と `fle_host_post_input` も包んだ**(`fle_g3_taskbar_iconic` は return が 1 か所になるよう組み替え)。I1 の検査の対象 | alpha10 の export が増えた分の穴(包まない export は Worker 0 が GIL なしで runtime の表を触る)を塞いだ |
| `harness/web/fruity/fle_idle_lib.js` hunk 1 | L1c が `fle_js_input_poll/done` の 2 import を `fle_idle_wait` の前に足した | `fle_idle_wait` を `dddii`(`wakeAddr, wakeSeq`)にし、`runtimeKeepalivePush/Pop` + `Atomics.waitAsync(wake word)` を足場のまま重ねた。L1c の 2 import は無変更 | L1c の import は pthread Worker でも定義されるが、`Module.fleInputPoll` は Worker 0 にしか無い(→ 0 を返す)。**加えて下の `MYW == 0` の条件でそもそも呼ばない** |
| `runtime/fle_thread.c` hunk 11(`schedule_ex` の先頭) | L1c が `schedule_ex` の先頭に `fle_input_ring_poll()` を足した | 足場の `wseq`(起床語を**スレッドを見る前に**読む)・`fle_gil_assert("schedule")`・`dbg_report_maybe()` の本体を写した。**`FLE_CS_HOLD_STATS` の `schedule_ex` の包みと `DBG1` の 3 行は写していない**(診断のみ) | **L1c の poll は `MYW == 0`(Worker 0 = UI の Worker)のときだけ呼ぶ**よう `#ifdef FLE_GIL` で条件を足した(下) |

**L1c の scheduler 点の poll が UI 側(Worker 0)にだけあって良いか(PM の問い)**: 良い(読み + 構造の確認)。(i) 入力 ring を読む JS(`Module.fleInputPoll`/`fleInputDone`)は Worker 0 の `Module` にだけある。(ii) 読んだ入力を積む `fle_input_host_*` は GIL の下の `evq`/`outq`(`fle_input.c`)で、`fle_input_settled()` は他 Worker のスレッド表を読むが GIL の下(`schedule_ex` は GIL を持つ)。(iii) UI のスレッドは Worker 0 に固定(`FLE_WORKERS=2`: 最初のスレッドは Worker 0、他は全部 Worker 1)なので、入力を処理するスレッドの scheduler 点は Worker 0 のものだけ。**Worker 1 が ring を読むと**『Worker 0 のスレッドが処理した』という settled の意味が崩れ、かつ `tail` の更新が非原子的なため二重に配送しうる。→ `MYW == 0` の条件は**除去ではなく契約(読み手は 1 人)の実装**。製品に入れるときは `#ifdef FLE_GIL` ではなく『Worker 0 のとき』の条件だけにする。

その他の自動で当たった物の意味の確認(0283b §1.2): 直 wasm → C の GIL 包みは足場の `build-engine-bundle.sh`(`GIL=1` のとき `c-imports-gil.txt` を作り `--c-imports` に渡す)に既にある。rsp-delta の wrapper は fe-wasmgen の生成物で `--tls-ctx` の `R[]` を使うため追加の対応なし。ページ側は足場の `importGlueFromBlob`(pthread Worker に Blob URL を渡す)と診断用の `onExit` / `onAbort` のログが当たった。


## 2. ビルドと起動(実測)

- 冷間 CI 段階 `extract felift lift b2d objects link seed bundle`(`tools/mk-rt.sh`: `GIL=1`、F2xH7、`CI_JOBS=4`、`BROTLI_QUALITY=1`、nice 15。host / b2d / `engine/in` / cargo-target / emcache は opt1 の冷間ビルド x16 から hardlink): felift+lift 40 s(base の cargo-target の再利用)、b2d 28 s、**objects 406 s、link 777 s**、合計 約 20 分(load 10〜14)。bundle `fl-engine-ci-377215a9`、engine wasm `engine.45b66779f396.wasm` 142,504,057 B(E9: 142,587,278 B)。`GIL=1` は全オブジェクトを `-DFLE_GIL -DFLE_TLS_MORE` で作り直す(53 の lift 単位を含む)ので、E9 からの増分ビルドは使えない。
- `check_i1.py`(I1: GIL を持ったまま第 1 Worker のイベントループを待つ import が無い)rc=0(`logs/rt-check-i1.txt`)。`cargo test -p fe-wasmgen --test mx_hybrid_tls` は `wrapping_sub` の 1 行の後で通る(`logs/rt-cargo-test-mxtls.log`)。**vitest・C のテスト・test.sh full・a4 は走らせていない**(使い捨ての木)。
- 起動: マニフェストの guest 環境に `FLE_WORKERS=2`、`FLE_PROCESSORS=1`(`tools/patchglue-rt.py`)。**2 本の DedicatedWorker(UI = Worker 0、他の全スレッド = Worker 1)が走り、参照プロジェクト(serina1-serum2)が開いて再生でき、停止なし**(60 s の run 5 本 + 1 本の smoke、全部 'ok (no stop)')。
- 音声: 現在の木の音声ホスト側は `globalThis.fleAudioSab`(Worker 0 の JS の状態)で動くので、**ミキサーの pthread Worker(Worker 1)が ring を使えるように、post-link で glue を補修**した(`tools/patchglue-rt.py`。測定専用、製品形ではない。段階 C の代わり): (1) Worker 0 が pthread への 'load' メッセージに `fleAudioSab` と小さな共有ログ `fleUlog` を載せる、(2) 7 か所の `const A = globalThis.__fleAud;` を lazy init に、(3) Worker 1 の `fle_js_unlock` が Unlock(first, n, t)を共有ログに書き、Worker 0 の `fleAudioWorkerState` がそれを自分の配列に流す(page の 1 Hz レポートが従来どおり Unlock を受ける)、(4) Worker 1 が出す 2 Hz の audio-pulse を emscripten の `cmd 9`(`Module.fleRelayPulse`)で Worker 0 経由で page に送る。**この補修の影響**: 音声の `engineGapFrames`(Worker 0 の `A.gapFrames`)は Worker 1 の状態を見ないので**この版では意味が無い**(値 2.5 M は読まない)。underrun・非無音 quanta は AudioWorklet 側の数なので有効。

## 3. 再生 60 s(場面 b〔serina1-serum2〕、Space の後 60 s、V8 プロファイラ無し、`scn-rt60.json`)— 実スレッド対 E9 協調の交互 3 往復

E9 = 本番 bundle `fl-engine-ci-072efa6a`(opt1 の E9、`lift/out-opt2/prod-new/` の読み取り)。**実スレッド腕と E9 腕は同じ page(srcRT の `app`)・同じ driver**。順番は rt → e9 を 3 回。**各 run の load(始め〜終わり)と他の Chrome との重なりは表に併記。load > 12 を含む run は『参考』**。表は `tables/60s-alt3-windows-0-60.md`・`60s-alt3-windows-30-60.md`(10 s 窓ごとの全列)、生ログは `logs/runs60/`。

(以下、数字は 30〜60 s の窓の合計。書き込み率は『全 Unlock の frame ÷ 44100 ÷ 実 s』/『[first, first + n) の範囲の和集合 ÷ 44100 ÷ 実 s』。後者は前に書いた範囲の書き直しを 1 回と数える)

| run | load(始め→終わり) | 書き込み率(全 / 重ならない) | Unlock の first の差 4096 / 12288 / その他(%) | Worker の CPU % 〔UI / ミキサー〕 | desk presents/s | UI Worker の page への戻り/s(idle − chained) | 仮想 s / 実 s | underrun(60 s の窓) quanta / episodes / 最長 |
|---|---|---|---|---|---|---|---|---|
| 実スレッド 1 | 8.1 → 10.8 | 0.825 / 0.686 | 67 / 21 / 12 | 62 / 88 | 32.3 | 33 | 1.000(実時計) | 24 / 3 / 9 |
| E9 協調 1 | 13.0 → 14.3(>12: 参考) | 0.642 / 0.636 | 66 / 30 / 4 | 85(1 Worker) | 11.4 | 16 | 0.474 | 52 / 12 / 473 |
| 実スレッド 2 | 7.1 → 6.4 | 0.834 / 0.695 | 67 / 21 / 13 | 63 / 89 | 34.1 | 34 | 1.000 | 15 / 2 / 13 |
| E9 協調 2 | 4.9 → 3.9 | 0.698 / 0.686 | 76 / 22 / 2 | 88 | 12.7 | 17 | 0.542 | 20 / 6 / 498 |
| 実スレッド 3 | 10.2 → 12.4(参考) | 0.738 / 0.710 | 74 / 21 / 5 | 60 / 86 | 25.4 | 26 | 1.000 | 23 / 2 / 17 |
| E9 協調 3 | 11.1 → 9.1 | 0.729 / 0.714 | 78 / 19 / 3 | 88 | 12.6 | 17 | 0.544 | 9 / 3 / 539 |

(underrun は 0〜60 s の窓の値〔`rt-arms.py` の末尾行〕。非無音 quanta: 実スレッド 15.5〜16.1 k / 20.75 k = 77%、E9 協調 19.3 k = 93%。)

読み:
- 壁時計の 30〜60 s では『重ならない frame』の書き込み率が実スレッドと E9 協調で近い(0.69〜0.71 対 0.64〜0.71)。**ただしこの比較は音楽の位置が違う**(下の §3c: 協調は仮想時計が実時間の 0.53 倍なので、壁の 30〜60 s は音楽の約 16〜37 s、実スレッドは 30〜60 s)。**同じ音楽位置では実スレッドの方が書き込みが多い**(§3c)。『全 Unlock』の 0.74〜0.83 のうち 14〜20% は書き直し(§4)。
- **ミキサー Worker の CPU は 86〜89% で、1 コアに届かない**。残りの 11〜14% が何を待っているかは §5(ストップ統計)。
- UI Worker は 60〜63%。『その他』(GPU / SwiftShader / 他のスレッド)は実スレッド 171〜257%、E9 協調 77〜99%(この機械の人工物。実 GPU では無いはず、読み)。
- 音の連続性は**別の壊れ方**: 実スレッドは 2 ブロック飛び(12288)が約 21% で、約 23% の quanta が無音(非無音 77%)、1 回の無音は約 186 ms。E9 協調は 12288 が 19〜30% あるが非無音 93%(**読み・未確認**: 仮想時計が実時間の 0.47〜0.54 倍で動くので、DirectSound の位置の飛びは仮想時間での飛びであり、page の transport には『遅く連続した音』として届く。飛びの 12288 が協調で無音にならない機構そのものは今回確認していない)。underrun の最長は協調 473〜539 quanta(先頭の約 1.4 s)、実スレッド 9〜17 quanta。**どちらも P3(途切れない)を満たさない**。

### 3c. 同じ音楽位置での比較(**訂正**: info 4 の『税 +40〜70%』『書き込み率は同じ』は音楽の位置が違う窓の比較だった)

音楽は後半ほど重い: 実スレッドのミキサー側 CPU / Unlock は音楽 0〜10 s で 41〜52 ms、10〜30 s で 76〜97 ms、30〜50 s で 82〜107 ms、**50〜60 s で 118〜128 ms**(`tables/per-unlock-by-music-time.txt`)。協調の腕は仮想時計が遅い(0.53 倍)ので 60 s の壁時計で音楽の約 37 s までしか進まない。`tools/matched-music.py` で、音楽時間(Space からの仮想 s。実スレッドは実時計)の 10 s のビンごとに揃えた(`tables/matched-music-time.md`):

| 音楽の位置 | 実スレッド CPU / Unlock(3 run) | E9 協調の按分(3 run)/ w1 | 実スレッドの書き込み〔重ならない frame / 壁 s〕 | E9 協調 / w1 | 協調が要した壁時間 |
|---|---|---|---|---|---|
| 0〜10 s | 41〜52 ms | 48〜51 / 48 | 0.86〜0.91 | 0.83〜0.87 / 0.85 | 14.6〜15.0 s |
| 10〜20 s | 76〜88 ms | 63〜67 / 71 | 0.75〜0.90 | 0.74〜0.78 / 0.71 | 14.0〜15.0 s |
| 20〜30 s | 78〜97 ms | 67〜76 / 73 | 0.82〜0.85 | 0.65〜0.74 / 0.70 | 18.1〜20.5 s |
| 30〜40 s | 82〜104 ms | (協調は到達せず) | 0.71〜0.74 | — | — |
| 40〜50 s | 88〜107 ms | — | 0.71〜0.74 | — | — |
| 50〜60 s | 118〜128 ms | — | 0.60〜0.67(`tables/60s-alt3-windows-0-60.md`) | — | — |

**方法(音楽位置で揃えた窓の比較。ci1 / opt1 も使える)**: (1) 腕ごとに run の `audio-p3.json`(`pulses`: `epoch`, `virt`、`unlocks`: `first`, `n`, `t`)・`driver-s*.json`・`cpu-s*.json`(`sdriveg2.mjs` の `driverSnap` が 10 s ごとに書く)を残す。(2) 音楽時間 m(Space からの仮想 s)= `(virt(t) − virt(Space)) / 1e7`、実時計の腕は壁時計に等しい。(3) `python3 tools/matched-music.py LABEL=RUNDIR …` が、音楽の 10 s のビンごとに『その音楽に要した壁時間』『重ならない frame ÷ 壁 s』『Unlock/s』『ミキサー側の CPU』を出す。(4) 比較は**同じビンの同士だけ**。協調の腕は 60 s の壁時計で音楽の 約 37 s までしか進まないので、そのビンまでを比べる。(5) 壁時計の同じ窓(30〜60 s など)で腕を比べない(音楽の位置が違い、後半ほど重い)。
- **同じ音楽位置での実スレッドの税は +20〜30%**(10〜30 s: 76〜97 対 63〜76 ms)。0〜10 s では 0〜5%。= 0283b §4 の見込み(+0.10〜+0.25 コア)の範囲で、**+40〜70% は音楽の位置の交絡**(訂正)。w1 の腕(GIL・TLS のコード入りの協調)は E9 協調と同じ(48 / 71 / 73 ms)なので、『コードの費用ではない』は変わらない。
- **音楽 10〜40 s では、実スレッドの方が書き込みが多い**(0.75〜0.90 対 0.65〜0.78)。実スレッドは音楽を実時間で進め(Unlock 9.4〜10.9/s)、協調は同じ音楽を 1.4〜2 倍の壁時間で進める(仮想時計が待つ)。= **P3 の書き込み率・実時間性では、音楽の軽い区間で実スレッドが協調に勝つ**(先の『同じ』は誤り)。
- **P3 の壁は最も重い区間**: 実スレッドは音楽 50〜60 s で CPU / Unlock 118〜128 ms に達し、Unlock/s が 6.7〜7.3 に落ちる(r = 1 に必要なのは 10.77 × 118〜128 ms = **1.27〜1.38 コア**、1 thread の上限 1.0 の 1.3 倍)。**1 thread で r = 1 を保つには、最も重い区間で 1 Unlock あたり ≤ 92.9 ms が要り、E9 の実スレッドは約 −27% 足りない**(E11 のミキサー −15% でも残り約 −12%)。協調の腕は 60 s の壁時計ではこの区間に到達しない(**5 分の再生は未取得**なので、曲全体の最も重い区間は不明)。

### 3b. Unlock の first の差の度数の 10 s 窓ごとの推移(PM 12:58 の依頼 (2): 実スレッドでも周期が 3 ブロックに落ちるか)

12288(2 ブロック飛び)の割合(%、10 s 窓 6 個、再生開始後 0〜60 s): 実スレッド 1: 4 / 15 / 14 / 19 / 19 / **28**、2: 5 / 7 / 11 / 22 / 17 / **24**、3: 5 / 16 / 11 / 19 / 20 / **24**。E9 協調 1: 10 / 15 / 18 / 28 / 29 / **31**、2: 4 / 15 / 15 / 16 / 23 / **28**、3: 10 / 15 / 19 / 19 / 16 / **23**(`tables/60s-alt3-windows-0-60.md`)。連続(4096)の割合の最小は実スレッド 55〜72%、協調 63〜74%。
- **60 s の中では、どちらの腕も『全ての差が 12288』(純粋な 3 ブロック周期、0273 の 100%)にはならない**。実スレッドの『+1 ブロック』の間隔の中央値は 97〜112 ms(1.05〜1.2 ブロック)で、3 ブロック周期(279 ms)ではない。
- 12288 の割合は**両腕とも時間とともに上がる**(0〜10 s に 4〜10% → 50〜60 s に 23〜31%)。仮想時計が無く、実 CPU と同期の待ちだけの実スレッドでも同じ向きに上がる(= 曲の後半ほど重いか、FL のミキサーが遅れを積む。どちらかは区別していない)。**5 分の run が取れたら、この系列が 100% に向かうかを見る**(取れなければ『未取得』)。
- 実スレッドの 12288 は、直後に -1 を伴う散発の遅れ(§4)で、協調(仮想時計の課金。-1 が 2〜8%)の周期的な 3 ブロックとは形が違う。

## 4. なぜ 12288(2 ブロック飛び)が約 21% 出るか、『その他』12〜13%(書き直し)の正体 — Unlock の列の読み(実測 + 読み)

`tools/rt-unlock-seq.py`(再生開始後 30〜60 s)。**実スレッド**の Unlock の列の実例(run `rt-a60-rt-2` の最初の 4 s。first は 4096 frame = 1 ブロック単位、dt は前の Unlock からの page 時計での ms):

```
t+   0.0  first      0
t+ 118.5  first  12288  (+3 ブロック。dt 118)
t+ 195.5  first   8192  (-1 ブロック。dt 77)   ← 飛ばした 2 ブロックのうち 1 つを埋め戻す
t+ 285.6  first  12288  (+1 ブロック。dt 90)   ← さっき書いた 12288 をもう一度書く(書き直し)
t+ 372.0  first  16384
t+ 473.1  first  20480
t+ 576.4  first  32768  (+3。dt 103)  t+ 649.0 first 28672 (-1。dt 73)  t+ 739.1 first 32768 (+1。dt 90、書き直し)
```

- 実スレッドの 30〜60 s の Unlock の間隔(中央値): 『+1 ブロック』97〜112 ms(ブロック周期 92.9 ms より長い)、『+3 ブロック』138〜176 ms、『-1 ブロック』72〜75 ms。**+3 の直前の Unlock の間隔は 100 ms を超える(遅れた)**。
- 読み(**実スレッドの 12288 と『その他』は同じ現象の 3 点セット**): ミキサーが 1 ブロックを時間内(実時計の DirectSound の書き込み位置が先に進む前)に書けないと、FL は次の Unlock を書き込み位置の先(+3 ブロック)に置く。その後、飛ばした 2 ブロックのうち 1 つを書き戻し(-1)、直前に書いた 1 ブロックをもう一度書く(+1 の書き直し)。1 回の遅れで Unlock が 3 回、**新しい frame は 2 ブロック分、うち 1 ブロック(4096 frame、約 93 ms)は無音のまま**。『-1』の数: 実スレッド 10〜34 / 30 s、E9 協調 4〜6 / 30 s。『+3 の直後に -1』の割合: 実スレッド 8〜29%、協調 2〜8%。
- 書き直しは**無駄なミキシング**(同じ 1 ブロックを 2 回計算する)。**ミキサー側の CPU を OS の thread CPU で測り直した**(`tools/per-unlock-cpu.py`、`tables/per-unlock-cpu.txt`。30〜60 s の窓。実スレッド = Worker 1〔UI 以外の全ゲストスレッド〕の CPU ÷ Unlock/s。協調 E9 = 1 Worker の CPU に alpha1 の 0296 の配分〔E10 で『ミキサー + その他のゲスト thread』が稼働の 0.606、E11 で 0.648〕を掛けた**按分の推定**):

| | CPU / Unlock | CPU / 重ならない block | CPU / 音の 1 秒(重ならない frame 基準) | r = 1 に要る(10.77 Unlock/s × CPU / Unlock) |
|---|---|---|---|---|
| 実スレッド 1・2・3 | 100 / 99 / 109 ms | 120 / 118 / 113 ms | **1.29 / 1.27 / 1.22 コア** | 1.07〜1.17 コア(書き直しを除けば × 0.83 で 0.9〜1.0) |
| E9 協調 1・2・3(按分 0.606〜0.648) | 68〜80 ms | 69〜81 ms | **0.75〜0.87 コア** | 0.73〜0.86 コア(+ UI 0.31〜0.36 コアで 1 Worker は 1.04〜1.22) |

  - **(訂正)この表は壁時計の 30〜60 s の窓の値で、実スレッド(音楽 30〜60 s)と協調(音楽 約 16〜37 s)は音楽の位置が違う**。表の +40〜70% は交絡で、**同じ音楽位置では +20〜30%**(§3c)。0283b §4 の『実スレッドの上乗せ +0.10〜+0.25 コア』の範囲。
  - **税の内訳 (i)『GIL・TLS・アトミックのコードの費用』は除外できた**(実測 1 本、`rt-w1-rt`、15:22〜15:27、load 6.4 → 6.8): **同じ実スレッドの bundle を `FLE_WORKERS=1`(協調・仮想時計、GIL と TLS のコードは入ったまま)で動かすと、重ならない frame の書き込み率(30〜60 s)0.680、仮想 s / 実 s 0.529、Worker の CPU 87%、CPU / Unlock 72〜77 ms、音の 1 秒あたり 0.78〜0.83 コア = E9 協調と同じ(0.64〜0.71、68〜80 ms、0.75〜0.87)**。12288 の割合 24%、-1 は 0 件(協調の形)。= **+40〜70% の税は GIL・TLS・アトミックのコードではなく、実スレッドで動かしたこと自体(実時計、Worker 1 への同居、同期、書き直しの連鎖)か、この機械の環境**から来る。
  - 残る内訳(**未分離**): (ii) Worker 1 に同居する FL の他のスレッド(thread 7 が 530 回/秒起きる等)と、実時計での `Sleep(1)` のポーリングが本当に起床する費用(協調では仮想時間なので CPU を食わない)、(iii) 書き直し(14〜20%。遅れの結果で、税の原因ではなく増幅)、(iv) この機械で SwiftShader が +1.4 コア動く影響(SMT の兄弟 thread、キャッシュ)、(v) 協調側の按分の誤差(0.606〜0.648)。分ける安い腕: (iv) 実スレッドを page の `?hz=30` で動かし UI の描画を減らす(60 s 1 本)、(ii) `thread 7` などを別 Worker に出す(コード変更が要る)。
  - 以上は 1 本ずつ(w1 は 1 本、load 6.4〜6.8)なので、**『コードの費用は E9 協調と同じ』は傾向**で、差 10% 以内かは言い切れない。
  - (参考)pulse の `threads[mixer]` の壁時計(スライス内の時間 = CPU + スライス内の待ち)÷ Unlock は 実スレッド 105〜118 ms、協調 90〜99 ms だが、**待ちを含むので CPU の比較には使わない**(協調では OS の CPU 基準より約 1.3 倍大きい)。
- **r = 1 に必要なミキサー側の時間**: 上の表の最右列。**実スレッドは 1.07〜1.17 コアで 1 thread の上限 1.0 を超える**(専用の 1 コアを与えても足りない)。書き直し(遅れの結果)が無ければ 0.9〜1.0 で境界。協調 E9 のミキサーだけなら 0.73〜0.86 コアだが UI が同じ Worker に乗る(1.04〜1.22)。= P3 に必要なのは『別コア』より『ミキサー側の CPU を実スレッドの税込みで 1 コア未満にすること』— mt3 の 0234s の『再検討の条件: 1 スレッドの速度が 1.3 倍以上』と同じ向き。E10 の Elastique 4 関数は音の 1 秒あたり 114 → 49 ms(約 −6.5%)、E11 は alpha1 の 0296 でミキサー −15%(ブロックあたり、2 ms プロファイル)。

(DirectSound の再生位置と Unlock の順序についての PM の問い: 上の列は、実スレッドでは**書き込み位置が実時計で進み、遅れたブロックの後に飛ぶ**ことを示す。GetCurrentPosition の呼び出し頻度〔100〜140/s〕との関係は**この build では数えていない**: 呼び出しの数を数えるカウンタが無い。『そのときのミキサーの待ち』は §5 のストップ統計。)

## 5. ミキサー Worker の待ちの内訳と UI 側の CS(ストップ統計の build、実測 1 本 + 読み)

build: `tools/mk-rts.sh` = 同じ木に `-DFLE_GIL_STATS -DFLE_CS_HOLD_STATS` を**ランタイムと harness のオブジェクトだけ**に(53 の lift 単位は再利用)、link 以降を作り直した(13 分。engine wasm `engine.c30708783877.wasm` 142,578,143 B)。**測定の費用が重い build(CS の出入りごとに `performance.now()`)で、書き込み率などの絶対値は参考**。`FLE_DBG_REPORT_S=10` で 10 s ごとに Worker 0 がエンジンの内部表を stderr に出し、page の `engineLog()` から読む(`logs/stats-rts/eval-elog.json`)。run `rt-stats-rts`(60 s、load 11.6 → 15.0 = **参考**、他の担当の Chrome が 1 本重なった)。

**重要な制約**: 私が取った `engineLog().slice(-900)` は、**最後の 2 本のレポート(10 s 間隔)しか含まない**(1 レポートが約 450 行のウインドウ表を含むため)。以下は**その 10 s の窓(再生開始後およそ 50〜60 s、エンジンの実時間 10.003 s)の差**で、60 s の系列ではない。次の run では必要な行だけを抜いて全レポートを残す(`tools/scn-rt60s.json` を直す)。

| 項目(10.0 s の窓、毎秒の値) | 値 |
|---|---|
| DirectSound: Lock / Unlock | 7.5 / 7.5 回(= 4096 frame × 7.5 = 書き込み 0.70 s/s) |
| **GetCurrentPosition の呼び出し** | **107.5 回/秒**(ミキサー thread 17 が約 100 回/秒 `Sleep(1)`〔1 回 約 1.2 ms〕と組で呼ぶ) |
| ミキサー(thread 17)の Sleep(1) | **120 ms/s(12%)**、100 回/秒 |
| 同じ Worker 1 の他のスレッド | thread 7: `Sleep` で 992 ms/s、**530 回/秒起きる**(1 回 約 0.09 ms 走る)、thread 5: 57 回/秒(約 0.1 ms) — pulse の統計でも thread 7 は壁時計の 5.4%、ミキサーは 94%(= Worker 1 はほぼ idle しない) |
| **GIL(giant lock)** Worker 1(ミキサー側) | 保持 58 ms/s(40,448 回/秒、平均 1.4 µs)、**待ち 36 ms/s**(slow acquire 684 回/秒) |
| GIL Worker 0(UI 側) | 保持 34 ms/s(22,329 回/秒)、待ち 27 ms/s(691 回/秒) |
| **FL の CS 0x16b6aa8**(DirectSound バッファ) | **ミキサーが 876 ms/s 保持(88%)、107.5 回/秒**。UI の待ちは 5 ms/s 未満 |
| FL の CS 0x16b6b20 | ミキサー側が 111 ms/s 保持、2,352 回/秒 |
| **UI(thread 0)の待ち** | `WaitMessage` 727 ms/s(**UI は 73% 待ち**、24 回/秒起きる)。`EnterCriticalSection` の待ちは窓内 5 ms/s 未満 |
| Worker の CPU(`/proc` の thread、run 全体の 30〜60 s) | UI 54〜59%(run-queue 待ち 1〜6%)、ミキサー 81〜86%(run-queue 待ち 3〜7%) |

累積(run 全体、エンジン起動から約 270 s のレポート): UI が CS を待った回数と時間は 0x1344e938 で 172 回・932 ms(最長 39.4 ms)、0x16b6b20 で 140 回・842 ms(最長 112.7 ms)。ミキサー(thread 17)が UI の保持する 0x16b6aa8 を待った 188 s は**プロジェクトを開く間の 1 回**(UI が 188 s 保持)で再生中ではない。

**CS 0x16b6aa8 の保持者の時系列**(`longest hold` の行、Space を 0 とした秒): **UI(thread 0)の保持は再生前だけ** — −216 s に 591 ms、−212 s に 440 ms、**−13 s までの 188 s(プロジェクトを開く間)**。**再生中の保持は全てミキサー(thread 17)**: +5.2 s に 380 ms、+9.8〜+16.9 s に 155〜217 ms、+49.5〜+52.4 s に 191〜207 ms。= ミキサーは 1 回のミックス(`EnterCriticalSection` → ミキサー本体 → Lock/Unlock → `LeaveCriticalSection`、0273 §1)を **CS の中で 100〜380 ms** かけている。保持の分布(再生を含む run 全体、188 s の外れ値を除く): 25〜100 ms の保持 215 回(合計 18.1 s、平均 84 ms)。窓の 10 s では 107.5 回/秒の短い保持(`GetCurrentPosition` のたび)と、約 7.5 回/秒のミックスの保持(1 回約 100 ms)の合計が 876 ms/s。

読み:
1. **ミキサー Worker の『CPU でない時間』約 12〜14% の正体は FL 自身の `Sleep(1)` の周期(120 ms/s)**。GIL の待ち(36 ms/s = 3.6%)は CPU のスピンと重なる。**FL の CS の待ち・UI との競合による待ちは、この窓では無い**(UI の CS 待ちは累積で 1.8 s/約 120 s 再生 = 1.5%)。= ミキサーが 1 ブロックに間に合わない原因は『待ち』ではなく**ミキサー自身の計算量**(1 Unlock あたり 99〜109 ms の CPU〔Worker 1 の OS の CPU 0.86〜0.89 コア ÷ 8.0〜9.0 Unlock/s、§4〕。協調 E9 は按分で 68〜80 ms)。PM の問い (1) への答え: 『CPU でなく待ちなら本命の梃子』— **待ちではなかった**。
2. UI は 73% 待っている(CS ではなく `WaitMessage` = FL が仕事を持たない)。UI の tick は FL の自分のタイマー等で 24〜37 回/秒で、**UI Worker の CPU を増やしても tick は増えない**(UI が飽和していない)。UI の描画(desk presents)は 25〜36/s。
3. GetCurrentPosition は 107 回/秒(ミキサーの `Sleep(1)` の周期と同数)。『書き直しの Unlock との関係』: §4 の列のとおり、遅れたブロックの後で FL が書き込み位置の先へ飛ぶ。**GetCurrentPosition の頻度と書き直しを結びつける測定はしていない**(この統計に無い)。
4. 注意: Worker 1 は**全ての非 UI スレッドを協調的に走らせる 1 本**(`FLE_WORKERS=2`)。thread 7 が 530 回/秒起き、ミキサーは 560〜617 回/秒 scheduler を通る(JSPI の切り替え)ので、ミキサー専用の Worker ではない。実スレッド版の『別コアのミキサー』は、FL のミキサー以外のスレッドも同じ Worker に乗る形。FL の他のスレッドを別 Worker に出す変更(FLE_WORKERS > 2)は**試していない**。

## 6. go / no-go の基準(0283b §5)に対する現在の数(入力の真の T は §7)

| 基準(0283b §5) | 実スレッド(E9 の木 + 足場、FLE_WORKERS=2) | 判定 |
|---|---|---|
| ① 書き込み率(30〜60 s)≥ 0.9(Go)、< 0.7(No-go) | 壁時計の 30〜60 s: 全 Unlock 0.74〜0.83、重ならない frame 0.69〜0.71(E9 協調 0.64〜0.71)。**同じ音楽位置(§3c)では 10〜30 s で 0.75〜0.90(協調 0.65〜0.78)、30〜50 s で 0.71〜0.74、50〜60 s で 0.60〜0.67** | 軽い区間は Go に近いが、**重い区間(音楽 30 s 以降)で No-go の線(0.7)の上下** |
| ② UI の tick(実時間あたり)≥ 100/s | desk presents 25〜36/s、UI thread の起床 24〜37/s(E9 協調 11〜14/s、スライス 31〜36/s)。UI Worker は 54〜63% CPU・73% が `WaitMessage` | **✗**(ただし CPU でも CS でもなく、FL が仕事を持たない待ち。原因は未特定) |
| ③ Worker 2 本の CPU 合計 ≤ 1.8 コア | 0.62 + 0.88 ≈ 1.46〜1.51(run-queue 待ち 1〜7%) | ✓(+ この機械の SwiftShader 1.7〜2.5 コア) |
| ④ UI の CS の待ち ≤ 0.15 s/s、最長 ≤ 40 ms | 累積 1.77 s / 約 120 s の再生(1.5%)、最長 112.7 ms(0x16b6b20)・39.4 ms(0x1344e938)。窓の 10 s では 5 ms/s 未満 | 率は ✓、最長は 0x16b6b20 で ✗ |
| ⑤ 入力の真の T(File クリック、再生中)≤ 協調の E9 | 実スレッド 37.2 / 49.7 / 63.2 ms(load 1.7〜7.2)対 E9 協調 21.5 / 33.2 / 47.0 ms(load 9.0〜3.2)、同じ page(交互ではなく 1 本ずつ) | **✗**(中央値で約 1.7 倍悪い) |
| (go/no-go 2 の予告)5 分で r ≥ 0.95 | **§7 に追記** | — |

読み: ① が最重要。**書き込み率は音楽の位置で 0.9 → 0.6 に落ちる**(音楽が後半ほど重い)ので、30〜60 s の壁時計の 1 つの数字で go/no-go は決まらない。③ は満たす(= 2 コアで足りる)が、CPU 合計が足りても 1 本のミキサー thread の速度が最も重い区間で足りない(§3c・§4)。

## 6b. SwiftShader の CPU を除いた 2 コア予算の読み(PM 14:25 の (3)。**読み、未測定**)

**(訂正: 以下の『1.07〜1.17 コア』『+40〜70%』は壁時計の 30〜60 s の値で、音楽の位置が違う窓の比較。同じ音楽位置の税は +20〜30%、最も重い区間〔音楽 50〜60 s〕の r = 1 の必要時間は 1.27〜1.38 コア、§3c)**

- この機械の『その他』(GPU / SwiftShader / 他)は実スレッド 171〜257%(UI の描画が 25〜36 frame/s で、1 frame あたり約 6〜7 ms の CPU)。実 GPU(依頼者の GTX 980)では rasterization の CPU 負担は無く、残るのは UI Worker の GL 命令の発行と GPU プロセスの 1 thread(0.1〜0.3 コア程度、読み)。
- 実スレッドのミキサー Worker の run-queue 待ち(CPU を待って走れない時間)は 3〜7%、UI Worker は 1〜6%(`/proc/<tid>/schedstat`、ストップ統計の run)。**SwiftShader の 1.7〜2.5 コアを取り除いても、ミキサー thread が戻せる時間は run-queue 待ちの 3〜7% が上限**(SMT の兄弟 thread の影響は分けられない)。
- 予算(実 GPU を仮定): UI Worker 0.54〜0.63 + ミキサー Worker 0.86〜0.89(OS の CPU)= **約 1.4〜1.5 コア + 他(AudioWorklet・compositor・GPU プロセス)0.2〜0.4 = 3 コア弱**。**2 コア(UI + ミキサー)に収まるのは UI 0.6 + ミキサー 0.9 = 1.5 で、CPU の合計の意味では成り立つ**。しかし **P3(r = 1)に必要なミキサー側の CPU は 10.77 Unlock/s × 99〜109 ms = 1.07〜1.17 コア**(§4)で、1 thread の上限 1.0 を超える。= **2 コア予算で P3 が成り立つかは『合計』ではなく『ミキサー 1 thread の CPU が現状から 7〜15% 以上(書き直し込み)、または税の分だけ減るか』で決まる**。E10 の Elastique(約 −6.5%)だけでは足りず、E11(alpha1: ミキサー −15%)と合わせると 85〜93 ms / Unlock = 0.92〜1.0 コアで境界。

## 7. 入力の真の T(再生中の File クリック・Esc。`scn-latb2.json`、L1c の製品の答えの規則、V8 プロファイラ無し)

**同じ page・同じ driver・同じ scenario の 1 本ずつ(別の時刻、交互ではない)**: 実スレッド `lb-rt-1`(14:35〜14:40、load 1.7 → 7.2)、**E9 協調 `lb-e9-1`(本番 E9 bundle 072efa6a、15:17〜15:22、load 9.0 → 3.2)**。File クリック 35 回・Esc 35 回。`logs/lat/`。

| 入力 | 腕 | 答え | T = 答えの present が返るまで(中央値 / p90 / p99 ms) | B(page → エンジン) | C(エンジンの処理) |
|---|---|---|---|---|---|
| File クリック(button down) | **実スレッド** | 35/35 | **37.2 / 49.7 / 63.2** | 8.2 / 21.2 / 30.2 | 26.1 / 33.2 / 37.1 |
| File クリック | **E9 協調** | 35/35 | **21.5 / 33.2 / 47.0** | 0.4 / 14.7 / 30.6 | 17.8 / 24.6 / 33.1 |
| Esc(key down) | **実スレッド** | 34/35 | **18.0 / 30.3 / 51.5** | 5.8 / 16.0 / 35.1 | 9.8 / 15.1 / 21.1 |
| Esc | **E9 協調** | 35/35 | **4.3 / 10.3 / 32.2** | 0.4 / 6.7 / 29.4 | 3.7 / 4.9 / 7.1 |

- **実スレッドの方が真の T が悪い**: File クリック 37.2 対 21.5 ms(+15.7 ms)、Esc 18.0 対 4.3 ms(+13.7 ms)。差は B(page → エンジン、実スレッド +8 ms 前後)と C(エンジンの処理、File クリックで +8 ms)の両方。(0283 §3.2 の E9 協調〔別 page、2 往復〕の 22 / 35 / 51、6 / 18 / 112 とも整合。)
- **読み**: B: 実スレッドの UI Worker は idle のとき `Atomics.waitAsync`(wake word)と page の IdleDriver の両方で待つので、page が ring に入力を置いた後の起床が協調の scheduler 点の poll(エンジンが常に走っているのでほぼ即時)より遅い。C: UI スレッドが GIL を取り直し(`fle_g3_*` の export と import ごと)、他の Worker と共有メモリの上で動く費用 — 区別していない。
- 音声側の副産物(各 run の累積、再生 112〜119 s): **遅れた書き込み `engineLateWrites`: 実スレッド 651 回、E9 協調 645 回(ほぼ同数)**。上書きできた frame(`Applied`): 1,038,165 対 1,172,030。失った frame(`Lost`): 120,014 対 121,068。= **『遅れた書き込み』の頻度(約 5.5〜5.8 回/秒)は協調と実スレッドで同じ**(§9 の行 8 の実数)。underrun: 実スレッド 10 quanta / 3 episodes(最長 6)、E9 協調 543 quanta / 21 episodes(最長 484 quanta = 再生開始の約 1.4 s)。

## 8. 失敗の形(設計の記述のみ、測定の対象外)と、『唯一の構成』にするための変更点

**失敗の形**(0283b §2 を今回の実測に合わせて更新):
- SAB が無い環境: 入力 ring がすでに SAB 前提で、COOP same-origin / COEP require-corp は全応答に付いている。実スレッド版で**新しく失う物は無い**(『SAB が無いと動かない』が増えない)。
- 論理 CPU が少ない(≤ 2): 実スレッド版は UI Worker 0.6 + ミキサー Worker 0.9 + ブラウザの他のスレッド(この機械では SwiftShader を除いても compositor・GPU・AudioWorklet)で**3 コア以上**を要る。2 コアでは協調 1 Worker より悪い(GIL のスピンと待ちが付く)。→ 旗なしの『唯一』では『起動時に `navigator.hardwareConcurrency` を見て Worker の数を決める』**実行時の判断**は未決(`FLE_WORKERS=1` の協調を残す = 2 経路のテストが残る。no-options 方針に反するかは PM の未決)。
- Firefox 159: 入れ子 Worker + JSPI は 158 までの手元の確認だけ(0234g)。**今回の足場は `Atomics.waitAsync`(`fle_idle_lib.js` の UI の wake word の待ち)を使う。Firefox での対応は未確認**(未対応なら UI の待ちが『ページの deadline だけ』に落ちる設計の確認が要る)。pthread Worker の最初の起動は『親 Worker の最初の idle まで遅れる』(Chrome の実測、0234 系)ので P1(起動 10 s)への影響も未測定。
- Worker の生成失敗 / 共有メモリの確保失敗: `pthread_create` が返らない。名前つきの停止(I1 の検査の延長)にする。

**『唯一の構成』にするための変更点**(今回のビルドと測定で分かったことを反映。行数は 0234m の見積りのまま、実績ではない):

| 項 | 現状(足場 + 今回の手当) | 唯一の構成にするための変更 | 根拠・注意 |
|---|---|---|---|
| ビルド | `GIL=1` の環境変数で全オブジェクトを `-DFLE_GIL -DFLE_TLS_MORE`(53 の lift 単位を含む)。ストップ統計は `SPIKE_CFLAGS` | `GIL=1` を既定に(旗の除去)。`SPIKE_CFLAGS`・`FLE_GIL_STATS`・`FLE_CS_HOLD_STATS`・`FLE_DBG_REPORT_S` を製品から外す | 冷間 20 分(objects 406 s + link 777 s)。E9 からの**増分ビルドは使えない**(全オブジェクト作り直し)。CI の cache key(`ci/plan.sh`)に GIL を入れる |
| 直 wasm → C | `c-imports-gil.txt`(`__wrap_fle_imp_*`)を `GIL=1` で作る(足場にある) | 既定にする。`--hybrid-mx-tls`・`--tls-ctx` を既定に | `mx_hybrid_tls` のテストは debug で u32 の引き算が panic(`wrapping_sub` 1 行、今回の木で直した) |
| scheduler | `MYW`・`RT_ON`・`wake_seq_`・`schedule_ex` の `wseq`・`fle_idle_wake_word` | 協調の分岐(`!RT_ON`)を残すか除くか。**除くと** 0279 の課金 k=4・IdleDriver の pacing は不要 | 協調を除くと 84/55/87 行のテストが全部作り直し(0234m: 8〜9 日) |
| L1c(0280/0281) | `fle_input_ring_poll()` を `MYW == 0` のときだけ | 『Worker 0 のとき』の条件だけにする(`#ifdef FLE_GIL` を外す) | §1 の表 |
| export | `fle_g3_*`・taskbar・`fle_host_post_input` を `fle_gil_enter/leave` で包む(今回、alpha10 の 3 つを追加) | 同じ。新しい export が増えるたびに `check_i1.py` と包みの一覧を直す | I1 の検査は rc=0 |
| 音声 | **post-link の glue 補修 4 点**(`patchglue-rt.py`) | **段階 C**(0234l): 音声 ring を wasm 共有メモリへ。Unlock の記録・2 Hz のパルス・`engineGapFrames` の状態を Worker 間で共有メモリのカウンタにする。pthread への `fleAudioSab` の引き渡しを glue の正規の経路に | 約 200 行(0234m)。今の補修は measurement 専用で、`engineGapFrames` が壊れる |
| ページ | `importGlueFromBlob` が pthread Worker に Blob URL を渡す(足場) | 同じ。CSP の `worker-src blob:` を確認 | UI の `onExit`/`onAbort` の診断ログは外す |
| テスト | 実行していない(vitest・C・test.sh full・a4) | 8〜9 日(0234m §2) | — |
| 総計 | | **S0 の 1 日(今回の rebase)は終わり。残り S2〜S6 で約 17〜19 人日**(0283b の 20〜22 から rebase の分を引いた) | 今回の S0〜S1 の実績: rebase + ビルド + 起動 約 1.5 時間(半日未満) |

## 9. 書き込みカーソルとプレイカーソルの戻し方は契約どおりか(PM 14:12 の依頼) — Learn の契約と runtime の戻り値の対応表

出典の分け方: **Learn の契約**は mt3w の 0273 §2 が保存した Learn 53 ページ(`lift/out-git/audio1-results/learn/`、ページ ID は 0273 の表)から引いた物で、私は Learn を読み直していない(今回の外部取得は別のページに当たって失敗した)。**runtime の戻り値**は今回の木(`38efb398` + 足場)の `fle_dsound.c`(`cursors()` 902〜929、`audio_sync()` 415〜470、`m_b_getcurpos` 938〜948、`m_b_unlock` 1111〜1141)・`fle_audio_ring.c`(`fle_audio_lead_frames`)を**読んだ**。**足場は `fle_dsound.c`・`fle_audio_ring.c`・`fle_audio_transport.c` に触れていない**(`rt-on-e9.diff` に無い)ので、協調と実スレッドで戻り値の式は同じ。**実機 Windows の write − play の列は 0 件**(Learn は『typically about 15 ms』しか書かない)。

**分担(PM の指示で mt3 の 0295 が native N1 での再現と契約の対応表の本表を担当)**: 本節は**実機ブラウザ(実スレッド・協調 E9)での観測**の側で、**契約の対応表の正本は mt3 の 0295 の表**。下の表は 0273 の行番号を借りた暫定の対応で、0295 と食い違えば 0295 を採る。DirectSound の『疑似時計』: 実スレッドでは最初の `CreateThread` から実時計に切り替わる(`fle_sys_clock_real_enable`)ので、`read_tick()` の 1 µs の加算と 0279 の課金 k = 4 は時間に効かない(協調の E9 では効く。**読み**: `fle_sys.c` の実時計の分岐を読んだだけで、動作の確認はしていない)。

| # | 項目 | Learn の契約(0273 の行) | runtime の戻り値(今回の木) | 実スレッド(実時計)での観測(今回の測定) | 判定 |
|---|---|---|---|---|---|
| 1 | play カーソル | 『次に再生するバイト』(Play and Write Cursors)。0273 行 1 | `abs_pos_at(fle_audio_consumed())` を size で割った余り × nBlockAlign。ブラウザでは `consumed` = AudioWorklet の RD(`Atomics.load`) | 実時計で単調に進む。更新は RD の更新単位 | 一致 |
| 2 | play の更新の粒度 | Learn に値が無い(`DSBCAPS_GETCURRENTPOSITION2` で『できるだけ正確に』)。行 2 | RD は 128 frame(2.9 ms)ごと | ミキサーは 107 回/秒(約 9.3 ms ごと)読む。粒度は検知に十分(0273: poll 16 ms でも崖は動かない) | 一致(値は Learn に無い) |
| 3 | write カーソル | play の先、『typically about 15 ms』(GetCurrentPosition)。行 3 | `abs_pos_at(max(pulled, consumed))`。呼ぶたびに `audio_sync()` が出力へ `consumed + lead` まで渡すので write − play = **lead(60 ms = 2646 frame)で常に一定** | 10 s の窓(50〜60 s)の `late_frames` の増加 **0**(累積 49,429 frame は起動時)= runtime の側は**一度も遅れていない**(`write_frames < consumed` にならない) | **選択**(S-audio-lead = 60 ms。『typically』なので違反ではない) |
| 4 | write は play の後ろに来ない | 行 4 | `max(pulled, consumed)` | — | 一致 |
| 5 | 出力ポインタの NULL 許容 | 行 5 | `if (R[2])`、`if (R[8])` | — | 一致 |
| 6 | 呼ぶたびの副作用 | Learn に無い | `cursors()` が `audio_sync()` を呼び、**出力へ frame を渡す(`mix_chunk` + `js_publish`)**。= カーソルの読み取りが出力の供給を兼ねる | 107 回/秒の読み取り = 107 回/秒の供給(1 回あたり約 410 frame ≈ 9.3 ms 分)。GIL の下で C + JS を呼ぶ費用がミキサー thread に載る(CPU の見積りは今回未測定) | 契約外の選択(行 22 の構造) |
| 7 | 止まったバッファ / プライマリ | 行 8・9 | 0/0、止まった位置 | — | 選択 |
| 8 | commit 済み(play〜write)への遅れた書き込み | 『触るべきでない』=定義されない(行 15) | `rewrite_unread()`: 出力が読む前なら guard(256 frame)より先を現在の内容で上書きして渡し直す。読み済みは `lost` に数える | 同じ scenario の 1 本ずつ(§7): **遅れた書き込み 651 回(実スレッド)対 645 回(協調)、Applied 1.04 M 対 1.17 M frame、Lost 120 k 対 121 k frame** = 頻度は同じ | 選択(実機より寛容) |
| 9 | Unlock が出力に渡る時点 | Lock → Unlock の区切りで『commit する量を決める』(行 14) | デバイスは `audio_sync()` のたびに**その時のバッファ全体**を読む。区切りで絞らない | 実スレッドでは、Lock と Unlock の間に別 thread の `GetCurrentPosition`〔= UI 側ではなく Worker 1 内の別 thread〕が `audio_sync()` を呼びうる(協調では譲らないので起きなかった) | **逸脱(小)**。実スレッドで効くかは未検証 |
| 10 | Unlock の記録(page の測定) | — | `first` = **`consumed + d`**、`d = (書いた領域の先頭 − play) mod size`(`m_b_unlock` 1136)= 『書いた領域が出力に読まれる絶対 frame』 | **+3 ブロックの `first` は『play が既に入っている半分』を書いたときに出る**(`d` が次の周回になる) | 計測の定義(下) |

**§4 の Unlock の列との関係(読み)**:
- `first` は『その領域が出力に読まれる絶対 frame』なので、ゲストが `half`(play の入っていない側)を時間内に書けば +1 ブロック、**play が既に入っている半分を書く(遅れた)と次の周回の位置になり +3 ブロック**になる(2 半分のリングで 1 周 = 2 ブロック、play は +1 進んでいる)。続く -1・+1 は、ゲストが次の判定で逆の半分を書き、さらに戻る列。= **runtime の戻り値は契約どおりで、『遅れて書いた』ことを正しく写している**。
- 遅れの大きさ: +3 の Unlock の間隔は 138〜176 ms(中央値、3 run)、その前の間隔は 100〜138 ms、全体の中央値は 99〜116 ms。**ブロック周期 92.9 ms を超える間隔が続いた後で +3 が出る**。= 遅れは『ミキサー自身の 1 Unlock あたり 99〜109 ms の CPU』(§4・§5)から来る。0273 の仮想時間のシナリオ(C = 検知から次のポーリングまで)は『C ≥ 約 186 ms で 3 ブロック周期』と予測し、実時計・実スレッドでは**周期が 3 ブロックに固定されず、遅れた回にだけ +3 が出る**(+3 は Unlock 全体の 21%)。仮想時間(協調)では C が長いまま続くので 3 ブロック周期になり、実時間では C が揺れるので遅れた回だけ飛ぶ — 読みで、同じ現象の連続版と散発版。
- 『契約の細部が合っていないから FL が先へ飛ぶ』という仮説への答え: **戻り値の側には、その仮説を支える逸脱が見つからない**(行 1〜5 は一致または Learn に値の無い選択)。0273 は lead を 15/60 ms、poll の間隔を 1〜16 ms に変えても崖が動かないことを仮想時間で示しており、今回の実スレッドの観測(`late_frames` 0、write − play 一定)もそれと整合する。**mt3 の 0295(`lift/out-git/mt3w-results/0295/docs/0295-directsound-cursors-and-fl-late.md`、最終 16:15 に訂正版)の結論(要点、私は再測定していない)**: FL のミキサーの規則は『Unlock の後 `Sleep(1)` + `GetCurrentPosition` を 1 ms ごと、最後に読んだ play が次に書く半分の外になったらミックスして Lock/Unlock、ミックスの後に play を読み直さない(662/662)、Lock の offset は厳密な交互(662/662)』。**+3 は『遅れた書き込みの領域が次の周回の位置になる』記録上の結果**(本節の `first` の定義の読みと同じ)。ミックスが 92.9 ms を超えると待つ周期(+3)と待たない周期(+1)が交互になる。write cursor のリード 15・60・120 ms、カーソルの粒度 10・50 ms、write cursor を無効値にしても FL の列・+3 の数・書き込み率は同一。**訂正(mt3 15:27、PM 15:25 も受理)**: FL は Unlock の直後に `GetCurrentPosition(NULL, &write)` で **write cursor だけを読み**、write が『今書いた半分の中』なら underrun として UI のカウンタ + 通知を出す検知を持つ(『FL は write cursor を使わない』は誤り)。ただしその検知の結果は Lock の位置・ペース・+3 の数を変えない(検知を止めた A/B で同一)。検知の頻度は write のリード(runtime 60 ms)で決まり、C > 約 33 ms で検知(リード 15 ms なら C > 約 78 ms)。差分案なし。= 本節の問いの答えは『戻り値に逸脱が見つからず、runtime の差分案は無い』。

**未検証**: (i) 実機 Windows の `write − play` の列と更新の粒度(0 件)、(ii) ゲストの `lead`(`[obj+0x3c0]` 由来)の実行値、(iii) 行 9(Lock〜Unlock の間の別 thread の `audio_sync()`)が実スレッドで書き込みの順序に効くか、(iv) 行 8 の `lost`/`applied` の数。(iv) は次の run で `window.__app.audio.detail()` の `engineLateWrites*` を読めば取れる(約 3 行の scenario の変更、Chrome 1 本分)。

## 10. S-gpu7-N(出典なしの選択)と自己申告

- **S-gpu7-24**: 実スレッドの腕は `FLE_WORKERS=2`・`FLE_PROCESSORS=1`(mt3 の『n = 1』)。UI = Worker 0、他の全ゲストスレッド = Worker 1。n > 1(FL 自身の並列ミキシング)・Worker 3 本以上は試していない。
- **S-gpu7-25**: 音声は post-link の glue 補修(`patchglue-rt.py`)。測定専用、製品形ではない。`engineGapFrames` が無意味になる(§2)。
- **S-gpu7-26**: 書き込み率の 2 通りの数え方(全 Unlock の frame / [first, first + n) の範囲の和集合)。前者は 0283 までと同じ定義、後者は書き直しを 1 回と数える。
- **S-gpu7-27**: UI の tick の代わりに、desk presents/s と『UI thread の page への戻り(idle − chained)/s』、ストップ統計の `WaitMessage` の park 回数/s を使う。実スレッドの pulse には thread 0 の統計が出ない(Worker 0 が thread を切り替えない)。**『FL の UI の tick/実 s』そのものは実スレッドで未測定**。
- **S-gpu7-28**: 『1 Unlock あたりの時間』= pulse の mixer thread の `tot`(スライス内の壁時計)÷ Unlock の数(30〜60 s の窓、29.6 s)。CPU ではなく、スライス内の待ちを含む。
- **S-gpu7-29**: `fe-wasmgen/src/wasm.rs` の `reg_base` を `wrapping_sub`/`wrapping_add` に(使い捨ての木のみ。release は元から wrap)。
- **S-gpu7-30**: `fle_thread.c` の足場の hunk のうち `DBG1` の 3 行は写さなかった。`FLE_CS_HOLD_STATS` の `schedule_ex` の包みは最初写さず(統計 build の最初の link が『`fle_thread_off_ms` が未定義』で失敗)、後から写した(plain の bundle `377215a9` には影響しない: `#define schedule_ex_inner schedule_ex`)。
- **S-gpu7-31**: `rt-on-e9.diff` は最終の srcRT の木(上の後から足した `fle_thread.c` の統計用の包みを含む)。plain の bundle `377215a9` はその前の木から作った(統計の包みは plain には入らない)。

- **S-gpu7-32**: 協調 E9 の『ミキサー側の CPU』は 1 Worker の OS の CPU に alpha1 0296 の配分(E10 の『ミキサー + その他のゲスト thread』÷ 稼働 = 0.606、E11 = 0.648。2 ms の V8 プロファイル付きの比率)を掛けた**按分の推定**。協調の OS の thread CPU は 1 Worker 分しか無く、分けられない。
- **S-gpu7-33**: w1 の腕 = 同じ実スレッド bundle を `FLE_WORKERS=1`・`FLE_PROCESSORS=1` で(仮想時計、GIL・TLS のコード入り)。1 本、load 6.4〜6.8。
- **S-gpu7-34**: E11 の dry-run は `rt-on-e9.diff`(38efb398 → 使い捨ての木の全差分)をそのまま `git archive 7373cd55` に当てた。E11 の木で `cargo test` やビルドはしていない。
- **S-gpu7-36**: 音楽位置で揃えた比較(`matched-music.py`)は、協調の腕の音楽時間を engine の pulse の `virt`(100 ns 単位)、実スレッドの腕を壁時計とし、10 s のビンの境界の壁時刻は pulse を 250 ms 刻みで補間、Worker の CPU は 10 s の snapshot の間を線形補間、協調の按分は 0.627(0.606 と 0.648 の中間)。
- **S-gpu7-35**: §9 の『Learn の契約』は 0273 §2 の引用の再掲で、Learn の再取得はしていない(外部取得は別のページに当たって失敗した)。契約の対応表の正本は mt3 の 0295。

**自己申告**:
1. **`timeout` コマンドを 1 回使った**(ci1 の session ID を調べる前置きに `timeout 20 true`。権限で拒否された。0283 の 1 回と合わせて 2 回)。
2. 実スレッド版の最初の入力の真の T の run(`lb-rt-1`)を始めた直後に、**他の Chrome が 2 本(ci1 と別の担当)走っているのに気づき**、**自分の run だけを PID で止めて**(親の bash・run-rt.sh・node・chrome の主 PID と、その renderer の子)、その run のディレクトリを消した。他の担当の run には触っていない。
3. 最初の smoke の run(13:07〜13:11)は別の担当の Chrome(port 4832)と重なった。ストップ統計の run(13:58〜14:03)は opt1 の p3 の Chrome(port 4835)と重なった(ci1 の待ち中に『1 行で知らせて入れる』合意で入れた)。この 2 本は『参考』。60 s の交互 3 往復は、他の担当の Chrome が走っていたかを run ごとには確認していない(load は表に併記)。
4. ストップ統計の最初の build は、私が写し漏らした `fle_thread_off_ms` で link が失敗した(13 分の無駄)。
5. `rm -rf` の前の `readlink -f` で `cd ../..` の誤りに気づいた(存在しないパスへの `rm -rf` で実害なし)。その後、正しいパスを `readlink -f` で確認して削除した。
6. 最初に取った `engineLog().slice(-900)` が最後の 2 レポートしか含まなかった(§5)。全レポートを残す scenario に直した。

## 11. E11(main 7373cd55)の上での go/no-go #2 に備える(PM 14:47 の追加。コードの作業は 0289 の後)

### 11.1 `rt-on-e9.diff` を 7373cd55 に当てる dry-run(実測、`patch --batch -p1 --dry-run`、`logs/rt-dry-on-e11.log`)

`rt-on-e9.diff`(38efb398 → 使い捨ての木、59 ファイル、240 KB)を `git archive 7373cd55` の木に当てた: **55 ファイルは offset も fuzz も無しで当たり、`lower.rs` は offset のみ(12 hunk が offset 付き)、3 ファイル 5 hunk が不成立、1 hunk が fuzz 2**。

| ファイル | 結果 | 原因(読み) | 手当て |
|---|---|---|---|
| `fe-wasmgen/src/lib.rs` | **3/5 hunk 不成立**(#2 at 77、#4 at 639、#5 at 935)、#3 は offset 16 で成立 | E11(0285)が `Config.hybrid_dispatch` と `Ctx.ti_helper` を足し、`Config` の Default と `Ctx { … }` のリテラル 2 か所の行が変わった | 今回と同じ機械的な手当て: Default に `hybrid_mx_tls: false`、2 つの `Ctx` に `mx_tls: conf.hybrid_mx_tls`。**半日未満** |
| `fe-wasmgen/src/main.rs` | **1/2 hunk 不成立**(#1 at 1)、#2 は fuzz 2 で成立 | E11 が `--hybrid-dispatch LIMIT LANDING HOOK` と usage 行を足した | usage 行に `--hybrid-mx-tls` を足し、引数の解析を重ねる。機械的 |
| `app/scripts/build-engine-bundle.sh` | **1/6 hunk 不成立**(#5 at 367) | 0285b: `hyb_guard cpart` が **7 番地**(`$T $MXA $BUFA $PRG $LIM $LAND $HOOK`)を検査するようになった | `MXTLS` の枝は MXA・BUFA を `0x0` にしていた(TLS 内の位置で、link map の番地ではない)。**LAND も同様に扱う必要がある(下)** |
| 上以外の 55 ファイル + `lower.rs`(offset) | 成立 | — | `tests/mx_hybrid_tls.rs` は新規ファイルなので衝突しない |

**意味の衝突(patch には出ない。E11 の rebase の本命)**:
1. **`fle_landing_top` が TLS_MORE の build ではデータの symbol でなくなる**。`fle_ctx_more.h:14` が `#define fle_landing_top (*(fle_landing **)(fle_ctx + FLE_CTX_LANDTOP))`(thread-local の `fle_ctx` の欄)にするため、link map に `fle_landing_top` の行が無い。E11 の `build-engine-bundle.sh` は `LAND=$(g "…fle_landing_top$")` で link map から番地を取り、`--hybrid-dispatch $LIM $LAND $HOOK` で型付き間接呼びの共有ヘルパー(`lib.rs` 978〜986: landing / hook の検査)に**番地を埋め込む**。→ 実スレッド版では **LAND を `RB + FLE_CTX_LANDTOP`(register page 空間の番地)にして、`reg_base` 経由の TLS 相対の読みにする**変更が要る(`--hybrid-mx-tls` と同じ形の `--hybrid-landing-tls` か、`mx_tls` を landing にも使う)。`typed_indirect.rs` のテストを TLS 版で 1 本足す。
2. `fle_progress_limit`(`fle_dispatch.h:37`、共有の 64 bit)と `fle_dispatch_hook` は TLS_MORE でも大域のまま(足場の `--hybrid-progress` が `fle_progress_count` を大域のまま使うのと同じ)。**読み・書きの競合は heuristic の範囲**(実スレッド版では RT_ON のとき slice の preemption を使わない〔読み〕が、型付き間接呼びが読む `limit` は E9 までは直 wasm が読まなかった値)。確認が要る。
3. E11 のアダプタ(`lib.rs` 1255〜1336)は `c.reg_base(…)` で register page に触るので、**TLS 相対の形に既になっている**(良い知らせ)。
4. 0285 の追加 C(`fle_hyb_dispatch` の変更)があれば GIL の包みの対象外か確認する(`gen_wrap.py` が `fle_imp_*` を拾うので、新しい import だけ確認)。

**見積り**: E11 への rebase = 機械的な 5 hunk(半日未満)+ 意味の衝突 1 と 2 の手当てとテスト(0.5〜1 日)+ ビルド(冷間 約 20 分)+ 起動確認 = **1〜1.5 人日**。

### 11.2 E11 のミキサー速度での 2 コア予算の更新(alpha1 の 0296 との突き合わせ)

| 項 | alpha1 0296(E11、協調、2 ms プロファイル、窓 30〜50 s) | 本書の実測(E9 の実スレッド / 協調) | E11 の実スレッド(推定) |
|---|---|---|---|
| 協調のミキサー側 CPU / ブロック | **ミキサー thread 45.1 ms + その他のゲスト thread 12.7 ms = 57.8 ms**(E10 は 53.2 + 15.1 = 68.3) | E9 協調: 68〜80 ms / Unlock(按分、§4) | — |
| 実スレッドの税 | alpha1 の仮定 +15〜+58% | **同じ音楽位置で +20〜30%**(§3c。壁時計の 30〜60 s の +40〜70% は交絡で訂正) | — |
| 実スレッドのミキサー側 CPU / Unlock | — | 音楽 10〜30 s: 76〜97 ms、音楽 50〜60 s: 118〜128 ms | alpha1 の窓(協調の音楽 約 15〜30 s)で **57.8 × (1.2〜1.3) = 69〜75 ms**。最も重い区間は E9 の 118〜128 ms × 0.85 = **100〜109 ms** |
| r = 1 に要るミキサー Worker(× 10.77) | **0.71〜0.98 コア** | 音楽 10〜30 s: 0.82〜1.04、音楽 50〜60 s: 1.27〜1.38 コア | alpha1 の窓: **0.74〜0.81 コア**、最も重い区間: **1.08〜1.17 コア**(なお 1 を超える) |
| UI Worker | **0.39 コア**(PM の要約による。内訳は 0296 を参照。私は検算していない) | 実スレッド 0.54〜0.63(desk presents 25〜36/s の描画 + FL の UI) | 0.55〜0.63(UI の仕事は E11 で −18%/ブロック、ただし実スレッドの UI は時間あたりの仕事) |
| 2 コアの合計 | 1.1〜1.4 | 1.4〜1.5(+ 書き直し分) | **1.4〜1.7** |

読み: (i) alpha1 の窓(協調の音楽 約 15〜30 s)の範囲では、E11 の実スレッドは 0.74〜0.81 コアで 1 コアに収まる(alpha1 の 0.71〜0.98 と整合)。(ii) **しかし音楽の後半は重く(E9 の実スレッドで 50〜60 s が 30 s 前後の約 1.4 倍)、最も重い区間では E11 でも 1.08〜1.17 コアで 1 コアを超える**。alpha1 の 0296 の窓は軽い側なので、go/no-go #2 は 5 分の再生が必須。(iii) 実スレッドの UI Worker は alpha1 の協調の UI 0.39 より重い(0.55〜0.63)— UI が 2.7 倍描く(SwiftShader 込み、この機械)ため。(iv) 税の『コードの費用』は w1 で除外済み(§4)。残る (ii) 同居スレッドと実時計のポーリング、(iii) 書き直し、(iv) この機械の環境が実 GPU / 専用 Worker で消えるかが go/no-go #2 の本題。

### 11.3 go/no-go #2(E11 の上の実スレッド版)の測定計画

前提: §11.1 の rebase(1〜1.5 人日)+ 冷間ビルド(約 20 分)+ 起動確認。**製品の木には入れない**(使い捨ての木、今回と同じ)。

| # | 何を | 何分 | どの比較で | 合否の基準(案) |
|---|---|---|---|---|
| A | **税の残りの分解**: (iv) 実スレッドを page の `?hz=30` で 60 s × 3(UI の描画を減らして SwiftShader の CPU を下げる)、(ii) 同居スレッドを別 Worker に出す(`FLE_WORKERS=3` 相当。コード変更 約 0.5 日)。**(i) のコードの費用は今回の `w1` で除外済み** | 約 5 分 × 3 + 約 5 分 × 3 | 同じ実スレッド bundle の標準 `hz` との交互 | CPU / Unlock(Worker 1)が標準より 20% 以上下がれば『環境・同居が主因』、変わらなければ『実時計・同期の構造が主因』 |
| B | E11-実スレッド対 E11 協調(本番 E11 bundle)の 60 s | 約 5 分 × 6(3 往復) | 同じ page・同じ driver で交互、load を記録。**§3c の方法で音楽位置を揃えて比べる**(壁時計の同じ窓で比べない) | 音楽 10〜40 s で重ならない frame の書き込み ≥ 0.9、ミキサー側 CPU / Unlock が音楽位置ごとに E9 の実スレッドより 15% 以上下がる、12288 の割合 ≤ 10% |
| C | **5 分の再生(必須。重い区間に到達する)** | 約 8 分 × 2 | E11 実スレッド対 E11 協調(交互 1 往復)。音楽位置で揃える | 最も重い区間(今回は音楽 50〜60 s)で **CPU / Unlock ≤ 85 ms**、5 分の書き込み ≥ 0.9、12288 の割合が時間とともに上がらない(今回は 4〜10% → 23〜31%) |
| D | 入力の真の T(File クリック・Esc、再生中) | 約 5 分 × 4(2 往復) | E11 実スレッド対 E11 協調 | T の中央値が協調以下(今回は実スレッドが約 1.7 倍悪い。B = page → エンジンの起床を直すか) |
| E | ストップ統計(FLE_GIL_STATS + FLE_CS_HOLD_STATS)の 60 s の**全レポート付き** | 約 5 分 + link 13 分 | E11 実スレッド | Sleep(1)・GIL・CS の待ちが今回(12% / 3.6% / ≈ 0)から増えない |
| F | UI の tick(実スレッド): thread 0 の統計が pulse に出るよう、Worker 0 の scheduler 点でスライスを数える計器(使い捨ての木に約 10 行) | A〜D と同じ run | — | UI の tick/実 s を協調と同じ定義で比べられる(今回は desk presents/s と WaitMessage の park 回数で代用) |

**成功条件(数)**: 1 thread で r = 1 を保つ条件は 10.77 Unlock/s × (CPU / Unlock) ≤ 1.0 コア、すなわち **最も重い区間で CPU / Unlock ≤ 92.9 ms**(余裕を見て **≤ 85 ms = 0.92 コア**)。今回の E9 の実スレッドは最も重い区間で 118〜128 ms(約 −27% 足りない)、E11 の見込みは 100〜109 ms(約 −12% 足りない)。**UI Worker は ≤ 0.6 コア**(今回 0.54〜0.63。2 コア予算 UI + ミキサー ≤ 1.5 に収まる条件)、**2 Worker の合計 ≤ 1.5 コア**(今回 1.4〜1.5)。UI の入力の真の T は協調以下(今回は +8 ms 前後 × 2 悪い)。**失敗(No-go)**: 最も重い区間で CPU / Unlock > 100 ms、または 5 分の書き込み < 0.8。

合計の Chrome 時間 約 100 分(ci1 / opt1 / alpha1 の窓と調整)。**実機(GTX 980)の値は 0 件**なので、go の場合も依頼者の機械の beacon(`webglRenderer`)での再確認を最後に入れる。
7. **info 3(14:10)の『1 Unlock あたり スライス内 105〜118 ms、協調 90〜99 ms』は pulse の壁時計(待ちを含む)で、CPU の比較に使うのは誤りだった**。OS の thread CPU で測り直した値(実スレッド 99〜109 ms、協調の按分 68〜80 ms、§4)に訂正した。info 4(15:30)で PM に訂正済み。
8. 私の preview server(port 4990、4991)が、いつの間にか落ちていた(原因は確認していない。他人のプロセスは触っていない)。14:17 に起動し直した。
9. 締切の時刻を最初は PM の時計だと誤読していた。PM に確認し(15:00 前)、この機械の `date` と確定した。
10. 3 本の Chrome が同時に走っているのに気づく前に、私の 1 本(`lb-rt-1`)を始めてしまった(PID で止めて消した、自己申告 2)。その後は、ci1 / opt1 / alpha1 と窓を調整して入れた(13:59〜14:03、14:35〜14:40、15:17〜15:27)。
11. `date` と PM の時計の差を過大に見積もって一時、締切を 14:50 前後と誤認した(実作業の優先順位に影響した:5 分再生を最後に回した)。
12. **info 4(15:30)で PM に出した『税 +40〜70%』『書き込み率は実スレッドと協調で同じ』は、音楽の位置が違う窓の比較による私の誤り**だった(協調の仮想時計が 0.53 倍なので、壁時計の同じ 30〜60 s が別の音楽の位置)。15:37 に `per-unlock-by-music.py` の表で気づき、info 5(15:42 相当)で PM に訂正した。**窓を揃える確認を、最初の交互 3 往復の解析(14:00)の時点でやるべきだった**(0283 の E7b 対 E9 の比較でも同型の交絡があり得る: 0283 §3.1 の UI スライスの 67 対 34/s は仮想時計の遅れの結果で、音楽の位置は同じ窓ではない)。
13. PM 15:33 の追加(`?hz=30` の実スレッド 60 s、他の Chrome が居ない窓の 1 本)は、ci1 の lat chrome が 16:02 ごろまで続くため **16:00 までに取れなかった(未取得)**。`tools/run-after.sh hz30` / `rt60` を用意した(ビルド・assets・preview は残してある間は実行できる)。

## PM の決定と取り込み(2026-11-06 15:40、docs のみ。製品の差分なし)

取り込み: gpu1(gpu7)の最終の文書・表・道具と、使い捨ての足場の差分 `rt-on-e9.diff`(238,688 B、59 ファイル、出荷しない)を `docs/investigations/gpu7/` に置いた。生のログ(約 10 MB)は入れない(原本は `lift/out-git/gpu7-results/0289/logs/`)。測定は実 Chrome 156 headless で実エンジン(実スレッド版の使い捨てビルドと E9 の本番 bundle)。この機械は SwiftShader で load 1.4〜15。**依頼者の機械・実 Windows は 0 件**。vitest・C のテスト・test.sh・a4 は走らせていない(測定専用の木)。

判断:
1. **所見を受理する**。同じ音楽位置で比べると、実スレッドの税は +20〜30%(音楽 0〜10 s は 0〜5%)で、`FLE_WORKERS=1` の腕は協調と同じ(GIL/TLS のコードの費用ではない)。ミキサーが 1 ブロックに間に合わないのは待ちでなく自身の計算量(Sleep(1) 12%、GIL 待ち 3.6%、FL の CS 待ちほぼ 0)。+3 → −1 の書き直しは mt3 の 0295 のとおり記録上の結果。
2. **P3 の壁は曲の最も重い区間**(r = 1 には 1 Unlock あたり 92.9 ms 以下)。実スレッド E9 は音楽 50〜60 s で 118〜128 ms(1.27〜1.38 コア)で約 27% 足りず、E11(ミキサー −15%)の推定でも 100〜109 ms で約 12% 足りない。協調版は 60 s で音楽 37 s までしか進まないので重い区間に達していない。**5 分再生は未取得**で、曲全体の最も重い区間は不明。
3. **実スレッドの S2 以降(約 17〜19 人日)には今は進まない**。E11 の木での go/no-go #2 に備える: mt3 に決定 0301(`rt-on-e9.diff` を E11 〔main 7373cd55〕に載せ直した使い捨て `rt-on-e11.diff`。gpu1 の見立てでは 55/59 ファイルが当たり、3 ファイル 5 hunk〔fe-wasmgen lib.rs 3、main.rs 1、build-engine-bundle.sh 1〕が不成立、`fle_landing_top` の TLS 化と E11 の型付き間接呼びの LAND の意味の衝突は `reg_base` の TLS 相対で解く)を割り当て済み。
4. **go/no-go #2 の成功条件(gpu1 §11 を採る)**: 最も重い区間の CPU/Unlock ≤ 92.9 ms(余裕込みで ≤ 85 ms = 0.92 コア)、UI Worker ≤ 0.6 コア、2 Worker の合計 ≤ 1.5 コア、5 分の書き込み率 ≥ 0.9、12288 の割合が時間とともに増えない、入力の真の T が協調以下。失敗: 最も重い区間 > 100 ms、または 5 分の書き込み率 < 0.8。**5 分再生は必須、音楽位置を揃えた比較で**。
5. **音楽位置を揃えた比較(`tools/matched-music.py`)は、以後すべての P3 の比較で必須**。E8〜E11 のこれまでの壁時計の書き込み率(E8 0.401、E9 0.763、E10 0.819、E11 0.866)は、仮想時計の速さが違うために音楽の区間が違っており、交絡している(performance-targets §4e の 15:35 の段落)。ci1・opt1 に音楽位置で揃えた列を足すよう指示済み。
6. 入力の真の T は実スレッドが協調より悪かった(File クリック 37.2 対 21.5 ms、Esc 18.0 対 4.3 ms。単発・非交互のため参考値)。SwiftShader の描画 CPU(+1.4 コア)は実 GPU で小さいはずだが未測定。
7. 未確認: 5 分再生、`?hz=30` の実スレッド 60 s、他の Chrome が無い窓での 1 本、実機の値、Firefox、交互の入力 T。自己申告(§10)の訂正点(info 4 の「税 +40〜70%・同じ書き込み率」は音楽位置の交絡)は文書に反映済み。
