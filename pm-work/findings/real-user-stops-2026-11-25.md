# 実機の停止の集計と、件数の多い理由の原因の推定(Issue #1404、2026-11-25)

## 0. 範囲

- 記録: `lift/out-app/host-extra/perf/`(ビーコンの保存先から取った直近 3 日の抽出。commit はしない)。126 件、17 セッション。期間(UTC)は 2026-11-23 09:37Z から 2026-11-25 00:50Z まで。道具の窓(実行時刻の前 72 時間。2026-11-22 01:17Z から 2026-11-25 01:17Z まで)は、この記録の全部を含む。
- 版(bundle.version): e29a730b932d、d7a6e6f392f2、2578ea1e23f6、b440e13a623a。b440e13a623a(32 記録)には、停止したセッションは無い。
- sid・IP・パス・ユーザー名・ユーザーエージェントは書いていない。セッションは、版・時刻・件数で呼ぶ。
- 道具: `node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf --stops`(この Issue で足した)。

## 1. 表(道具の出力)

停止したセッションは 10 件(記録 54 件)。内訳は、停止 7 件、エラー 1 件、届かず離脱 2 件(重なりあり)。件数(記録)は、その指紋のセッションの全記録の数。

- 期間(UTC): 2026-11-22 01:17Z から 2026-11-25 01:17Z まで(直近 72 時間)。記録: 126 件、セッション: 17
- 停止したセッション: 10 件(記録 54 件)。内訳: 停止 7 件、エラー 1 件、届かず離脱 2 件(1 つのセッションが複数に入ることがある)
- 版ごと(停止したセッション数): d7a6e6f392f2 1 件, e29a730b932d 4 件, 2578ea1e23f6 5 件
- 停止したセッションは、engine.exit.reason が stopped か error の記録が在るもの、error の記録が在るもの、最初の idle に届かなかったもの(--hangs と同じ定義)。
- 指紋は、engine.exit.stop の lastFle(先頭 80 字)・stopLine・abortKind と、error の what・where を ` | ` でつないだもの(先頭 240 字)。数字と 16 進の番地は #、X:\ で始まるパスは <path>。件数(記録)は、その指紋のセッションの記録の数(全部)。並びは件数の多い順。

### 版 d7a6e6f392f2(停止したセッション 1 件)

| 指紋 (lastFle・stopLine・abortKind・error.what・error.where の順) | 件数 (記録) | セッション | 終わり方 | 最初 (UTC) | 最後 (UTC) | DPR の区分 | 開く窓の答え: Demo の下 (files.engine) | 開く窓の答え: .flp (files.engine) | 起動からの時間 (pageMs) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| - \| - \| - \| - \| - | 7 | 1 | 停止 ×1 | 2026-11-24 06:53Z | 2026-11-24 06:56Z | 1 超 2 未満 ×1 | 記録無し ×1 | 記録無し ×1 | 60 秒以上 ×1 |

### 版 e29a730b932d(停止したセッション 4 件)

| 指紋 (lastFle・stopLine・abortKind・error.what・error.where の順) | 件数 (記録) | セッション | 終わり方 | 最初 (UTC) | 最後 (UTC) | DPR の区分 | 開く窓の答え: Demo の下 (files.engine) | 開く窓の答え: .flp (files.engine) | 起動からの時間 (pageMs) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| - \| - \| - \| - \| - | 21 | 4 | 停止 ×3, 届かず離脱 ×1 | 2026-11-24 08:51Z | 2026-11-24 09:10Z | 1 超 2 未満 ×4 | 記録無し ×4 | 記録無し ×4 | 10 秒以上 60 秒未満 ×2, 60 秒以上 ×2 |

### 版 2578ea1e23f6(停止したセッション 5 件)

| 指紋 (lastFle・stopLine・abortKind・error.what・error.where の順) | 件数 (記録) | セッション | 終わり方 | 最初 (UTC) | 最後 (UTC) | DPR の区分 | 開く窓の答え: Demo の下 (files.engine) | 開く窓の答え: .flp (files.engine) | 起動からの時間 (pageMs) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) \| == stop: SIGABRT (runtime abort, see runtime message) \| (not a runtime message) \| - \| - | 17 | 2 | 停止 ×2 | 2026-11-25 00:41Z | 2026-11-25 00:50Z | 1 超 2 未満 ×2 | 記録無し ×2 | 記録無し ×2 | 60 秒以上 ×2 |
| - \| - \| - \| @<url> line # > WebAssembly.compile:wasm-function[#]:# (other) \| engine worker, running (the engine trapped before it could stop by itself) | 5 | 1 | エラー ×1 | 2026-11-25 00:46Z | 2026-11-25 00:47Z | 1 超 2 未満 ×1 | 記録無し ×1 | 記録無し ×1 | 60 秒以上 ×1 |
| fle: RaiseException: exception code #, flags #, # argument(s) raised; \| == stop: SIGABRT (runtime abort, see runtime message) \| (not a runtime message) \| - \| - | 3 | 1 | 停止 ×1 | 2026-11-25 00:32Z | 2026-11-25 00:32Z | 1 超 2 未満 ×1 | 記録無し ×1 | 記録無し ×1 | 10 秒以上 60 秒未満 ×1 |
| - \| - \| - \| - \| - | 1 | 1 | 届かず離脱 ×1 | 2026-11-25 00:48Z | 2026-11-25 00:48Z | 1 超 2 未満 ×1 | 記録無し ×1 | 記録無し ×1 | 10 秒未満 ×1 |

## 2. 件数の上位 5 件

件数は記録の数。DPR の区分は、どれも 1 超 2 未満(実値 1.7647。169 相当)。開く窓の答え(answeredUnderDemo・answeredFlp)は、この記録のどれにも無い(下の 4 を見る)。

| 順 | 版 | 指紋 | 件数 (記録) | セッション | 終わり方 | 起動からの時間 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | e29a730b932d | `- \| - \| - \| - \| -`(停止の項目が無い) | 21 | 4 | 停止 3、届かず離脱 1 | 10〜60 秒 2、60 秒以上 2 |
| 2 | 2578ea1e23f6 | `fle: RegOpenKeyExW: an empty component …` | 17 | 2 | 停止 2 | 60 秒以上 2 |
| 3 | d7a6e6f392f2 | `- \| - \| - \| - \| -`(停止の項目が無い) | 7 | 1 | 停止 1 | 60 秒以上 1 |
| 4 | 2578ea1e23f6 | `@<url> line # > WebAssembly.compile:wasm-function[#]:# (other) …`(trap) | 5 | 1 | エラー 1 | 60 秒以上 1 |
| 5 | 2578ea1e23f6 | `fle: RaiseException: exception code #, flags # …` | 3 | 1 | 停止 1 | 10〜60 秒 1 |

## 3. 上位 5 件の原因の推定

「実測」は記録の値。「推定」と「不明」は、コードの読みや他の記録からの推測。確からしさは、各項の末尾に書く。

### 3.1 指紋 1(版 e29a730b932d、「- | - | - | - | -」、21 記録)

- 実測: 4 セッションのどれも、停止の項目(engine.exit.stop)が無い。停止の 3 件の message は、証拠の行(`first non-background pixel …`)と戻り番地だけ。戻り番地で分けると、次の 3 つになる。
  - 戻り番地 0x411e4d: 2 セッション、13 記録(起動から 16 秒と 293 秒で停止)。
  - 戻り番地 0x43ba94: 1 セッション、7 記録(起動から 158 秒で停止)。
  - 届かず離脱: 1 セッション、1 記録(起動から 42 秒。最後の段は lazy_done の 3.5 秒)。
- 推定: 0x411e4d は、版 2578ea1e23f6 の指紋 5(RaiseException)と同じ戻り番地。`lift/crates/fe-lift/tests/seh_rt.rs` は、0x411e4d を、関数 0x411d20 の中の実機の戻り番地として使う(`CALL(0x411e4d, raise_stub)`)。同じ発生元の例外と見られる。
- 不明: 0x43ba94 の発生元(この作業の場所に、この番地の記録も表も無い)。届かず離脱の理由(記録から区別できない)。
- 確からしさ: 0x411e4d は推定(fle: の行がこの版の記録に無いので確定できない)。0x43ba94 と届かず離脱は不明。

### 3.2 指紋 2(版 2578ea1e23f6、RegOpenKeyExW、17 記録)

- 実測: lastFle は `fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) in a key path, reached after the keys before it exist`。stopLine は `== stop: SIGABRT`。2 セッション(308 秒 12 記録、117 秒 5 記録)。どちらも戻り番地 0x5af85c。
- コードの読み: `lift/runtime/fle_reg.c:341` の fle_stop。RegOpenKeyExW で、鍵の経路を先頭から辿り、空の要素(先頭・末尾・連続の区切り)に着いた時に止める。文言は記録と一致する。fle_reg.c:338 の注記は、この扱いを決定 0099 の暫定とし、Windows がここで何を返すかは「どのページにも無い」としている。
- 確からしさ: 停止の直接の理由は確定(記録の行とソースの文言が一致し、この文言を出す箇所は 1 つ)。guest のどの鍵の経路か、呼び出し元 0x5af85c の関数は不明(記録に鍵の名前が無く、対応する表も無い)。

### 3.3 指紋 3(版 d7a6e6f392f2、「- | - | - | - | -」、7 記録)

- 実測: 1 セッション(182 秒で停止、終了コード 134)。停止の項目は無い。message は証拠の行と、戻り番地 0x680e3d2。
- 推定: 0x680e3d2 は、`pm-work/issue-drafts/wave1p/128-exoplanet-balance-exit.md` が ExitProcess(0) の戻り先として挙げた番地(台帳では #1372。`pm-work/created-issues.tsv` の 128 の行)。その草稿は、Balance の入口と続きが別の側で lift されるため、longjmp の続きが登録なしで呼ばれ、出口の枠が 0 なら ExitProcess(0) になると読んでいる。`lift/runtime/fle_proc.c:279` の ExitProcess は fle_stop を通るので、134 で止まる点も合う。
- 確からしさ: 推定。戻り番地の一致と草稿の読みによる。fle: の行は記録に無いので、ExitProcess の行は確かめられない。

### 3.4 指紋 4(版 2578ea1e23f6、trap、5 記録)

- 実測: engine.exit.reason は trap(終了コード -1)。error の what は `@<url> line 2 > WebAssembly.compile:wasm-function[136291]:0x93b89be (other)`、where は `engine worker, running (the engine trapped before it could stop by itself)`。== stop の行は無い。起動から 85 秒。
- 推定: wasm の関数番号 136291 の、オフセット 0x93b89be で trap した。trap の種類(other)と、関数名は、記録からは分からない。
- 不明: 関数名(`lift/out-app/gen/wasm-func-names.tsv` が、この作業の場所に無い。在る時だけ使うと Issue にある)。trap の理由(message の先頭の 1 行が、関数の位置だけで、理由の語が無い)。
- 確からしさ: 不明。

### 3.5 指紋 5(版 2578ea1e23f6、RaiseException、3 記録)

- 実測: lastFle は `fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no frame-based handler took it (11 frames searched)`。戻り番地 0x411e4d。起動から 47 秒で停止。
- コードの読み: `lift/runtime/fle_seh.c:459` が、枠の探索で、どのハンドラも取らなかった時の文言を作る。`lift/runtime/fle_proc.c:372` の注記は、枠が無い RaiseException は MSDN のとおり ExitProcess と同じに扱うとし、decision 0132 で fle_seh.c がリンクされると枠を探す。Windows の定義では flags 0x1 は継続不可(EXCEPTION_NONCONTINUABLE)。
- 確定: 停止の直接の理由(fle_seh.c:459 の文言と一致)。
- 推定: 発生元は戻り番地 0x411e4d(指紋 1 と同じ。seh_rt.rs の読み)。
- 不明: 0xEEDFADE が何の例外か(リポジトリに定義が無く、テストでも値だけ使う)。投げたのが、どの guest のコードか(戻り番地の関数の対応表が無い)。
- 確からしさ: 直接の理由は確定。発生元は推定。

## 4. 読みの注意(前の記録と違った点)

- 停止の項目(engine.exit.stop)と証拠の項目(engine.exit.evidence)は、この 126 件のうち 3 件(版 2578ea1e23f6 の停止)にしか無い。版 e29a730b932d と d7a6e6f392f2 の停止 4 件には無い。Issue の背景は「版 e29a 以降には入る」としていたが、記録の上では合わない。app の版は bundle.version に出ないので、どの版のアプリかは、この記録からは決められない(推定)。
- engine.exit.message は、停止の 7 件すべてで証拠の行だった。停止の理由(fle: の行)ではない。コードの読み: `lift/runtime/fle_raster.c:803` の fle_raster_report が、停止の報告の最後に証拠の行を書き直し、`lift/harness/wasm/trace_fruity.c:53` の __wrap_fputs が、それを last_msg(harness の runtime message)にする。その結果 message は証拠の行になり、abortKind も「(not a runtime message)」になる(abort_kind() は last_msg を見る)。停止の理由は、lastFle と戻り番地でしか分からない。
- 前の記録(`pm-work/findings/real-user-exits-2026-11-24.md` の 3.2 と 4.2 の H1)は、message が証拠の行だったことから「fle: の行は無い」と推定していた。版 2578ea1e23f6 では、message が証拠の行でも fle: の行が在る。この点で、前の推定は確かめ直しが要る。
- 指紋の 80 字(lastFle)で、理由の後ろが落ちる。指紋 2 は「reached after the keys before it exist」、指紋 5 は「no frame-based handler took it」の部分が指紋に入らない。同じ型の別の理由を、指紋が区別できない場合がある。
- 件数(記録)は、長く続いたセッションほど多い(定期の記録は 30 秒ごと)。理由どうしの比べ方には、セッション数の方が合う。
- 答えの項目(answeredUnderDemo・answeredFlp)は、126 件のどれにも無い。files.engine は 13 件に在るが、件数の項目は無い。この Issue の表の「記録無し」は、そのためである。

## 5. 直す Issue の案(1 行ずつ。起票は PM)

- A(指紋 5 と指紋 1 の 0x411e4d): RaiseException(コード 0xEEDFADE、枠なし)の発生元(戻り番地 0x411e4d)を特定し、枠で扱うか、起きない経路を直す。
- B(指紋 1 の 0x43ba94): 戻り番地 0x43ba94 の停止の発生元を、停止の行(lastFle)の記録で特定する。
- C(指紋 2): RegOpenKeyExW の、途中に空の要素がある鍵の経路(決定 0099 の暫定。呼び出し元 0x5af85c)について、Windows の挙動を確かめ、扱いを決める。
- D(指紋 3): 戻り番地 0x680e3d2 の ExitProcess(0) が #1372 の続きと同じ経路かを、版 d7a6e6f392f2 の停止の行の記録で確かめる。
- E(指紋 4): wasm の trap(関数番号 136291、オフセット 0x93b89be)の関数名を表で引き、trap の記録に関数名と理由を出す。
- F(計測): 停止の記録に、停止の行・lastFle・戻り番地を、版によらず必ず入れる(版 e29a・d7a6 の停止に停止の項目が無い理由を直す)。message が証拠の行で塞がらないよう、理由の行を別に記録する。

## 6. 記録に無いもの・限り

- 鍵の名前、guest の戻り番地の関数名(symbol の表が、この作業の場所に無い)、wasm の関数名、0xEEDFADE の意味。
- 件数が少ない(10 セッション、54 記録)。停止の理由ごとの比べ方は、件数が増えてからにする。
- 版 b440e13a623a の停止は、この記録には無い。
- 版の時刻(deploy の時刻)は、この作業では見ていない。

## 7. 道具の変更

- `tools/perf-beacons/summary.mjs` に `--stops` を足した(既存の `--exits`・`--hangs` と既定の出力は変えていない)。
- `spec/perf-stops.test.mjs` を足し、`spec/workflow.test.sh` の試験の一覧に加えた。
- 製品(`app/`・`lift/`・`ci/`・`.github/`)は変えていない(読むだけ)。
