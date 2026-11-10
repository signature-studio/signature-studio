# 決定 0311(仮): present を別 Worker `P` に出す — 使い捨ての木での実装と、協調版での測定(第 1 報の下書き)

状態: **仮の決定・実装(使い捨ての木、出荷しない)**。作業者 `gpu1`(成果物名は gpu8)、2026-11-06 JST 19:40〜。依頼: PM(0311、第 1 報 21:00、最終 22:30)。前提: 0306 §9(設計)、§10(idle の内訳)、§11(T の B・C)。基準: main d0447902 の `app/`(E13 の push 後の木。page は E12 と同じ)を `lift/out-gpu2/srcP/app` に取り出し、差分は `diffs/` に出す。**エンジン(wasm)は E12 本番のまま**(`fl-engine-ci-7a9b0800`、ci1 の bundle を読み取りで使用)。**runtime の C は変えていない**。区分: **実測**=このセッションのコマンドの出力、**読み**=解釈、**未取得**。出典なしの選択=`S-gpu8-N`。この機械(16 vCPU の Zen 5 VM、実 GPU なし = SwiftShader、headless Chrome 156)の値で、依頼者の機械は 0 件。

## 冒頭の三点

1. **何を実行したか(実測)**: 実 Chrome 156 headless と実 Firefox 158 で**実エンジン**(E12 本番 `fl-engine-ci-7a9b0800`、wasm 無変更、協調 1 Worker。および mt3 の B1 = 実スレッド n=2・段階 C)に参照プロジェクトを開き、**main の page そのまま(p0 系)と、present を別 Worker `P` に出した page(p 系)、B1 の page、B1 に P を足した page(b1p)**を、ABBA を含む計 約 45 本動かした(純再生 20 s → 再生中の File メニューのクリックと Esc を 35 回、B1 は 5 分再生と PCM 捕捉も、Firefox は 30 s 再生)。**P は実エンジンで動いた**(画面は FL 全体が正しい)。代役・再生・モックなし。依頼者の機械は 0 件、この機械は SwiftShader/llvmpipe。各 1〜4 本。
2. **結論(実測 + 読み)**:
   - **協調 + P**: 仮想 s/実 s 0.82〜0.94 → 1.00、present 20 → 32〜43/s。T は present の**状態**に左右される。低い状態(32〜35/s)は File +0.6〜1 ms・Esc +1.8〜3 ms で p0 並み、高い状態(43/s)は Esc が +10 ms(中央値 15.7 対 4)。V4(idle で処理した入力を既に出た scene で直ちに答える)で答えの出ない Esc は 6/70 → 2/70(p0 1/70)。**E14 の 5 基準(V4 の ABBA、§3q): (1) ○、(2) File ○・Esc ×(+4.9 ms)、(3) ×(2 対 1)、(4) File ○・Esc ×(+10.3、ぎりぎり)、(5) ○(模擬あり)。PM の判断: V4 は採らない、E14 は push しない。**
   - **2 状態(§3r)と V7(§3s・3t・3u)**: 低い状態 = scene/rAF 0.6〜0.7・wake > 8 ms が 0〜2%・Esc の T が p0 並み、高い状態 = scene/rAF 1.00・wake > 8 ms が 21〜74%・Esc +6〜10 ms(B1+P と協調 P の両方)。GPU プロセスは飽和していない(SwiftShader 16 スレッドは 1 本 0.12〜0.19 コア、GPU main 最大 0.14)。**H\***(読み): 高い状態は、engine が WANT に毎回すぐ応えて publish した scene が P の frame step の直後に当たり、P が rAF の間 JS を受けられない。**V7**(P の WANT を frame step の後の task で立てる、P 層の順序の補正、PM が本命に決定)で、計器つきの pv7 2 本・製品の形の pv7s 4 本は全て低い状態、V7 の無い pmd2 は 3/3 本が高い状態。
   - **E14 の判定(P + V7、製品の形、ABBA 通算 pv7s 4 本対 p0mt 3 本、§3u)**: File T 21.0 対 19.3(+1.7)、Esc 5.9 対 4.9(+1.0)、p90 +0.6 / +6.0 ○、仮想 s/実 s 0.87〜1.00 対 0.63〜0.84 ○、present 約 30/s 対 14〜20/s。**Esc の late だけ基準と同数でない**: pv7s 8/140 対 p0mt 1/105(再生中 35 件ずつ。全 run 通算で p0mt 2/175、Fisher 両側 p = 0.026〜0.08、弱い証拠)。late の読みは §3w(idle 経路の入力の答えの seq が credit されない形。V4 が直す方向、未検証)。
   - **P + V7 + V4(製品の形、§3x)**: pv74s 3 本対 p0mt 3 本で、File T +0.4 ms、Esc T +1.9 ms(縁)、p90 +0.5 / +5.1、仮想 s/実 s 0.90〜1.00 対 0.65〜0.74、再生中の Esc の late 2/105 対 1/105(P + V7 だけは 8/140)= E14 の基準を満たす(Esc の T は縁)。**PM の判断(05:47): E14 の最終の組は P + V7(desk-worker.ts のみ +4/−1)。V4 は worker.ts の追加の差分で、P + V7 でも late の差が統計的に有意でない(両側 0.08)うえ、Esc の中央値の差が大きい(+1.9 対 +1.0 ms)ので採らない(記録に残す)。**
   - **B1(実スレッド、製品の形)**: 5 分再生で数の基準を全て満たす(PCM の零 0.036%、CPU/Unlock 最大 74 ms、UI Worker 0.58、合計 ≤ 1.43、書き込み ≥ 0.978)。**入力 T は協調より +10 ms(B +4.8・C +5.5)で未達**。**B1 + P**: B は消え(6.6 → 0.4 ms)、UI Worker 0.58 → 0.40、File の T 中央値は協調より速い(10.0 対 17.6 ms、p90 45.8 対 25.1)が、Esc の T が 23.7 ms。V5(P が port で engine の event loop を起こす)は効かず不採用(§3o)。
   - **機能(§3p)**: Firefox 158 で P は動いた(画面が正しく、再生 30 s)。P の能力判定が偽になる 2 経路(WebGL2 無し、`Atomics.waitAsync` 無し〔Worker の探針で模擬〕)は旧経路で動いた。
3. **未取得**: B1 + P + V7(実スレッド版の製品の形、page `app-dist-b1p7s` を用意済み、計時は未)、Esc の late の原因の確認(scene 単位の記録つきの V7)、再生前の Esc が P 側で late になる run が 2 回(原因未調査)、Firefox で p3 と pv7s の速さの比較(pv7s は p0・p3 より約 11% 遅い 1 本、§3v)、実際の Firefox < 145、first idle、Hz 120、実機(GTX 980)、e2e の desk 系テスト、再現。

## 1. 設計の確定(現在の page の present 経路を実コードで読んだ)

| 何が | どこで(main の木) | 今 |
|---|---|---|
| canvas の所在 | `client.ts:86` `transferControlToOffscreen()` → `init` メッセージで**エンジン Worker** | エンジン Worker が WebGL2 context を持つ |
| 描画 | `desk-gl.ts:200` `GlDeskPresenter.present()`: `api.scene()`(wasm の `fle_desk_scene`、`:203`)→ `readScene`(DataView)→ `texSubImage2D`(`:241`、エンジンの線形メモリから直接)→ quad の draw | JS 2.0 + `(program)` 6.0 + GL 直後の idle 3.6〜5.8 ms /present がエンジン Worker の時間(0297 §4b) |
| 起点 | `worker.ts:338` `startDeskLoop`(Worker の rAF)、`:220` `onEngineIdle` の fallback(`DESK_FALLBACK_MS = 20`、`:39`)、`:201` `answerInline`(`INLINE_TRY_MS = 6`、`:141`)、`:112` `processed` の直後 | rAF は Worker が飽和すると 3.6/s しか配られない(0297 §4)。present の大半は idle の fallback(17.7/s) |
| sleep | `worker.ts:41` `DESK_SLEEP_AFTER_MS = 100`、`desk-sleep.ts` `QuietClock` | 100 ms 変化が無ければ rAF を止め、idle が次の変化を探す |
| 答え(L1c) | `worker.ts:192` `completeInline` ← `deskTick`(`:381`)が『settled の後に始めた present が画素を変えた』とき | page の latency の `answer` |

**P への移し方**(実装済み。`desk-link.ts`・`desk-worker.ts`):
- page(`client.ts`)が P を作る(`new Worker(new URL('./desk-worker.ts', import.meta.url), { type: 'module', name: 'fl-desk' })`、Vite が別 chunk にする)。共有 SAB の link(6.4 KB)と MessageChannel を作り、canvas・input ring・link・port1 をエンジン Worker に、link・port2 を P に渡す。
- エンジン Worker は**最初の idle**(`ensureDesk`)で、engine が scene の export を持ち、`webgl2Usable()`(今と同じ『platform が決める』条件)が真なら、`OffscreenCanvas`(まだ context なし)とエンジンの `WebAssembly.Memory`(共有)を port で P に**移す**(canvas は transfer、memory は共有のまま clone)。偽なら今までどおり(2D の `DeskPresenter`、または link の無い構成)。
- P は `GlDeskPresenter`(**既存のコードを再利用**。変更は `sceneMemory`〔description の読み先を link の slot にする〕と `deferred`〔`resync()` が要求になる〕の 2 点だけ)を作り、context ができてから `sceneMode(1)` を**要求**する(エンジン Worker が次の scheduler 点で `fle_desk_scene_mode(1)` を呼ぶ = 『GL context ができてから runtime を scene に切り替える』順序を保つ)。
- 起点は P: **P の rAF**(P の event loop は空なので毎 frame 配られる)が `WANT` を立てる。エンジン Worker は**すでに ring を見ている点**(`inputPoll` = scheduler 点、約 2,000 回/s、と `onEngineIdle`)で `WANT` を見て、`fle_desk_scene` を呼び、返った description(6,464 B)を link の slot に**コピー**して `PUB` を進め `Atomics.notify` する。P は `Atomics.waitAsync(PUB)` で起き、`present()` し、`DONE`・結果(画素を変えた矩形数)・時刻を書く。
- **in-flight 1**: エンジンは `PUB === DONE` のときだけ公開する(`fle_desk_scene` が返す『書かれた矩形』は呼び出しで消費されるので、P が描いていない scene を次で上書きすると矩形が失われる。P が忙しい間、runtime は矩形を溜め続け、次の呼び出しでまとめて返す)。slot は 1 枚で足りる(0306 §9 の『2 枚』は不要だった)。
- 答え(L1c): エンジン Worker が公開時に `inlineSettled`(gate)を覚え、`DONE` が追いついたとき `result > 0` かつ gate なら `completeInline(true)`。page には `input-done`(今と同じ)と新しい `desk-answers`(seq の一覧と、P が書いた present の時刻)を送る。P は自分の `desk` 統計(`at` = present の時刻)を page に**直接**送る(エンジン Worker を経由しない)。
- sleep: `QuietClock` は P に移った。P の frame loop が眠ると `ASLEEP` を立て、エンジン Worker の idle の fallback(`DESK_FALLBACK_MS` 20 ms)が**眠っている間だけ**次の変化を探す。画素が変わった present が P を起こす。
- test aid(`readback`・`desk-gl-read`・`desk-peek`)はエンジン Worker が P に port で転送し、P が page に直接答える(`desk-peek` の窓の一覧はエンジン側の `describe()` を転送時に添える)。

### 無くなる物 / 増える物

| 無くなる(エンジン Worker から) | 増える |
|---|---|
| present の GL 呼び出し(`texSubImage2D`・draw・uniform…)と、その直後の GPU 待ちの idle(1 present 約 12〜14 ms、Worker の 14〜21%、0306 §9) | P Worker 1 本(生成、module 読み込み、GL context と shader の作成が P に移る。CPU は present の分 ≒ 0.10〜0.20 コア) |
| Worker の rAF の loop(`startDeskLoop`)と `QuietClock`(P へ)、rAF が配られない問題 | link の SAB 6.4 KB、公開ごとのコピー 6,464 B(≒ 1 μs 台)と `Atomics.notify` 1 回、scheduler 点ごとの `Atomics.load` 数回 |
| idle の fallback による present の**常時**の試行(17.7/s):awake の間は P の WANT が起点になる(rAF 数 = 最大で表示の Hz) | `fle_desk_scene` は rAF ごと(≤ Hz)に呼ばれる(今は idle ごと ≤ 50/s) |
| **残るもの**: 20 ms の gate(`DESK_FALLBACK_MS`)は P が眠っている間の『最初の変化を見つける』ためにだけ残る(静かな間は Worker が空いているので費用は無視できる) | P の frame loop が起きる/眠る信号(link の 2 語) |

### 自己点検(`docs/policies/lift-fidelity.md` と PM の指示)

- **ピクセルを postMessage しない**(オーナーの 2026-11-04 の指示): P はエンジンの共有メモリから `texSubImage2D` する(今と同じ)。postMessage は初期化(canvas・memory の handover)と test aid の読み戻しだけ。link のコピーは 6.5 KB の description で、ピクセルではない。
- **FL の class/窓の分岐なし**: `desk-gl.ts` は窓を知らないまま(description のとおりに描く)。追加した物は link・P・handover だけで、窓の種類・位置・名前を見る所はない。
- **旗・環境変数・URL パラメータの追加なし(No-options)**: P を使うか否かは**platform の能力**(`webgl2Usable()` と engine の scene export)が決める(今の GL/2D の選択と同じ規則)。
- **FL の挙動の代行なし**: P は description を描くだけ。『settled』の判定・応答の規則(L1c)は変えていない(エンジン Worker が gate を覚える)。
- 新しい処理の分類(方針の 3 つ): (a) 除去できる余計な規則: Worker の rAF の loop と、present のための idle の常時試行(awake の間は不要になる)。(b) 外部 API の契約の実装: 該当なし。(c) 通信/変換の削減: ピクセルの経路は不変、**待ち(エンジン Worker の GL 待ち)の削減**であって、通信/変換の削減ではない。PM/オーナーの判断が要る(0306 §9 と同じ)。
- **能力の判定(PM 20:03 の指示で変更)**: 判定は『platform の能力の有無』で、FL や窓の分岐ではない。**引き渡しの前に P が自分で確かめる**: P は起動直後に `Atomics.waitAsync` があるか、`webgl2Usable()`(WebGL2 が `failIfMajorPerformanceCaveat` なしで得られ、最大 texture ≥ 8192)が真かを調べ、link の `READY` に 3(できる)か 2(できない)を書く。エンジン Worker は最初の idle で `READY` を読み、**3 のときだけ canvas を渡す**。2 のとき(Firefox ≤ 144 の `waitAsync` なし、GL なし)は**旧経路**(エンジン Worker が自分で WebGL2 または 2D の `DeskPresenter` で present)に戻る。0(P がまだ言っていない)なら、その idle では決めず、次の idle で読む。canvas は渡した後に戻せないので、引き渡し前の確認が要る(PM の限界 (1) への答え)。Firefox は 145(2025-11-11)から `waitAsync` を持つ(PM の調査)ので、依頼者の Firefox 159 と ci1 の Firefox 158 は P の経路に入る。
- 既知の限界(未対応): (1) 引き渡した後に P が GL の program を作れなかった場合(`webgl2Usable()` が真でも shader の失敗)は、log を出して desktop が空になる(旧経路への戻りは無い。珍しい)。(2) `desk-dump`(非 peek、`api.compose`)は P を使わない既存の経路のまま。(3) Firefox でのこの経路は未確認(実装は browser 共通の API だけを使う)。

## 2. 実装(差分: `diffs/0311-present-worker-src.diff`・`-test.diff`・`-tsconfig.diff`)

| ファイル | 追加/削除 | 内容 |
|---|---|---|
| `app/src/engine/desk-link.ts`(新) | +99 | link の SAB(6,592 B: ヘッダー 128 B + description の slot 6,464 B)、`DeskLinkEngine`/`DeskLinkP` |
| `app/src/engine/desk-worker.ts`(新) | +137 | P: 能力の確認、handover の受け取り、`GlDeskPresenter`、frame loop(rAF)、`Atomics.waitAsync` で scene を待つ present の loop、test aid の読み戻し |
| `app/src/engine/worker.ts` | +95 −18 | `ensureDesk`(引き渡しの判断)、`pService`/`reapPresented`/`publishScene`(P との契約)、答え(L1c)の経路、test aid の転送 |
| `app/src/engine/desk-gl.ts` | +8 −3 | `sceneMemory`(description の読み先)と `deferred`(resync が要求になる)の 2 点だけ |
| `app/src/engine/client.ts`・`protocol.ts`・`main.ts` | +24 −4 | P を作る、port を渡す、`desk-answers` |
| テスト | +76(desk-link)+ 約 90(desk-gl-link)+ 1(worker-diag の fake) | link の契約(1 scene in flight、コピー、wake、要求)、GlDeskPresenter が link の slot から読み engine memory から upload する・size 変更で再帰しない |

単体テスト 1320 passed / 8 skipped / 0 failed(追加は desk-link 5 件と desk-gl-link 3 件、worker-diag の fake を 1 行直した)、`tsc` の 3 構成 clean。**runtime の C は変えていない**。新規の option・環境変数・URL パラメータなし。

## 3. 測定(実測。`tables/pt-presents-cpu.md`・`tables/pt-true-T.md`・`logs/`)

### 3a. 純再生 20 s と、再生 + File メニュー(35 回)の present と CPU

| run | 区間 | present/s | 仮想 s/実 s | FL UI の刻み/実 s | 提示率(present/刻み) | エンジン Worker / P (コア) | SwiftShader 等 (コア) | 開始 load |
|---|---|---|---|---|---|---|---|---|
| p0-1(main そのまま) | 純再生 | 20.8 | 0.889 | 57.5 | 0.36 | 0.84 / – | 1.24 | 0.83 |
| p-2 | 純再生 | **43.2** | **1.000** | 64.8 | 0.67 | 0.92 / 0.35 | 2.59 | 0.91 |
| p-3 | 純再生 | **43.4** | **0.998** | 64.7 | 0.67 | 0.93 / 0.35 | 2.57 | 0.57 |
| p2-4(修正版) | 純再生 | 42.7 | 1.000 | 65.7 | 0.65 | 0.93 / 0.35 | 2.55 | 1.12 |
| p0-1 | 再生 + File メニュー | 18.2 | 0.985 | – | – | 0.84 / – | 1.20 | |
| p-2 / p-3 / p2-4 | 再生 + File メニュー | 38.6 / 38.3 / 38.7 | 0.997 / 0.996 / 0.997 | – | – | 0.90 / 0.40 | 2.44 / 2.41 / 2.44 | |

(p0 の idle 由来の present: 18.8/s。P では 0 = awake の間は P の frame loop が起点。P の rAF は 57/s 配られた〔p0 のエンジン Worker の rAF は 6.8/s〕。)

読み: エンジン Worker から present(GL の呼び出し、GPU 待ち)が消え、**エンジンが実時間で走れるようになった**(0306 §9 の見込み『0.80〜0.87 → 1.0』より良い、0.889 → 1.000)。present が倍になったのは、P の rAF が毎 frame 配られ(p0 は 6.8/s)、エンジンが速い分 FL の刻み(57.5 → 64.8/s)も増えたため。**実画素の変化の上限(0297: 純再生 19.4/s、刻みの 40%)は p0 の Worker 飢餓の下での値で、P では刻みの 67%** — FL が描く量は p0 の測定では取りこぼされていた(読み)。SwiftShader 等の CPU が 2.1 倍になったのは present が倍になった分(この機械は GPU がソフト実装)。

### 3b. 入力の真の T(File クリックと Esc、再生中、`tables/pt-true-T.md`)

| 入力 | p0(1 本、n=35) | p(2 本、n=70/60) | p2(修正版、1 本、n=35/29) | 参考: E11 協調 2 本(0306、n=70) |
|---|---|---|---|---|
| File クリック T 中央値 / p90 | **18.6 / 30.8** ms | **19.3 / 43.3**(run 別 20.9, 18.9) | 19.4 / 29.1 | 18.2 / 23.1 |
| Esc T 中央値 / p90 | **4.4 / 15.3** ms | **9.9 / 27.0**(run 別 12.7, 7.5) | 12.4 / 28.8 | 4.6 / 18.8 |
| B(page → エンジン) | 1.8 / 1.1 | 0.6 / 0.4 | 0.4 / 0.6 | 0.7 / 0.8 |
| 答えが出た数(Esc) | 35/35 | 29/35・31/35 | 29/35 | 34/35 |

- **B は改善**(1.8 → 0.6 ms、0306 §11 の予想どおり: UI スレッドが present で占有されない)が、**Esc の T は悪化**し、答えの出ない Esc も増えた。File クリックは中央値同等、p90 は p で悪化(43.3)・p2 で p0 並み(29.1)。
- Esc の個別の記録(`p-3`・`p2-4`)は**二峰**: 多くは T ≒ 3〜4 ms(p0 と同じ)だが、一部は C が 19〜33 ms(p0 には無い)。p0 の悪い Esc は B が大きい(エンジン Worker が present で止まっていた)のに対し、P の悪い Esc は B が小さく C が大きい = **エンジンは受け取ったが、答えの present が遅れた**。
- 読み(**未検証、2 つ**): **H1(矩形の消費)**: WANT 起点の scene(57/s)が、ゲストが Esc の結果を描き終えた後・settled を読む前に走り、答えの scene が使う矩形を取る(§3d)。二峰の T と答えの出ない Esc の両方を説明する。**H2(コンポジタの待ち)**: P の present は JS では約 0.35 ms だが、GL 呼び出しが SwiftShader のコンポジタの『1 frame in flight』で 16〜30 ms 待たされることがあり(0297 §4b の `(program)` の塊)、present が倍の P では答えがその後ろに並ぶ。H1 は変種 V1 と `ansEarlyShown`、H2 は present の所要時間の計器(§3e)で分ける。
- 修正版 `p2`(`answerInline` が P の描画中に試行を使い切らない/settled を読んだ後に publish)は **T を直さなかった**。= 『6 ms の INLINE_TRY_MS で答えが遅れる』という最初の読みは外れた(p2 は File クリックの p90 を 43.3 → 29.1 にしたが 1 本の揺れの範囲かもしれない)。
- 次の検証(計器): `GlDeskPresenter.present()` の所要時間の累積計数(> 8 ms・> 16 ms の回数と合計)を足し、p0 と p を比べる。長い present が Esc の悪い記録と同じ時刻にあるかを、`lat` の記録の時刻と突き合わせる(次の窓、約 12 分)。**直す道の候補**: (a) P の present の間隔を表示の Hz 以下にする(今は publish ごとに即描画。frame に揃えて 1 frame 1 回)、(b) 入力を待っている間(`inlineSince ≠ 0`)は P の WANT 起点の publish を止め、答えの scene だけを出す、(c) `OffscreenCanvas` の frame を `commit()` で明示し、前の frame の確認を待たない。

### 3c. 未取得
first idle(P の生成が起動を遅らせないか: ci1 の方法で交互 10 組を依頼する)、Hz 120、`P` の再現(p 2 本・p2 1 本)、実機(GTX 980: SwiftShader の『1 frame in flight』の待ちは実 GPU で小さいはず〔読み〕)、Firefox(`waitAsync` のある 145 以降は P の経路に入る、未実行)、e2e の desk 系テスト(Chrome を要する。未実行)。

### 3d. 『答えの出ない入力』の定義(PM 20:58 (a))

- **page 側(`ui/latency.ts`)**: 入力が `input-done`(engine が処理した、`tIdle` が決まる)を受けた後、**`tIdle + 1 ms(IN_WINDOW_MARGIN_MS)` までに 1 つも frame が credit されず、さらに `LATE_MS = 150 ms` を過ぎても何も来なければ outcome `'none'`**(`expire()`、`latency.ts:266`)。窓の後 150 ms 以内に frame が来れば `'late'`。数えている『答えが出た』は outcome `'window'`/`'stream'`。
- **engine 側(`worker.ts`)**: `inputPoll` が『settled』(ゲストが取り終え、メッセージループに戻った)を読んだ後に**始めた present が画素を変えたとき**だけ `completeInline(true)` が答える(`deskTick` の `gate`、decision 0280 L1c)。settled を読んだのに答える present が現れなければ `INLINE_GIVEUP_MS = 150 ms` で `completeInline(false)`(答えなしで `input-done`)。
- **どの経路で出ないか(コードの読み。計器で確かめる)**: `fle_desk_scene` が渡す『書かれた矩形』は呼び出しで**消費される**。P の rAF(57/s)による `WANT` 起点の publish は、ゲストが Esc の結果を描き終えた後・settled を読む前にも走る。その scene が結果の矩形を**持ち去る**(gate は偽 = 答えにならない)と、settled の後の答えの scene は**差分なし**で、`present()` が 0 を返し、答えにならない。次に画素が変わる present(再生ヘッドなど、20〜30 ms 後)が gate を満たして答えるか、150 ms まで答えが出ない。= **二峰の T(3 ms か 19〜33 ms)と、答えの出ない Esc の両方を説明する**。p0 でも同じことは起こり得るが、present が少ない(idle の fallback 20 ms と rAF 6.8/s)ので稀。『P の frame に合流して答えが消える』のでも『タイムアウト』のでもなく、**答えの scene の矩形が先の scene に取られる**(読み、`ansEarlyShown` で確かめる)。
- **この読みなら変種 V1 が直す**(下)。最初に作った修正版 `p2`(答えの試行を使い切らない)が効かなかったのは、原因が試行の間隔でなく矩形の消費だったため。

### 3e. 計器(使い捨ての木、製品に入れない): `diffs/0311-measure-only-timing-{P-arm,p0-arm}.diff`
- `GlDeskPresenter.present()` を `presentInner()` で包み、**画素を変えた present の所要時間**(呼び出し側から見た、JS と GL 呼び出し内の待ち)を数える: 合計、> 8 ms・> 16 ms の回数と合計、最大、**> 8 ms の present の開始時刻と長さ(最後の 512 件、`slowLog`)**。p0(エンジン Worker の present)と P の両方に同じ計器を入れた(`srcP0m`・`srcPm`)。
- P の arm には答えの経路の計数を足した(link の Int32 を 5 語): 答えた scene の数、**settled を読んでから publish まで**・**publish から engine が DONE を見るまで**の平均、P が描画中で答えの試行を見送った数、**settled の前に publish されて矩形を取った scene の数(`ansEarlyShown`)**。
- 解析: `tools/pt2-analyze.py`(各 File クリック/Esc の T を、その入力から答えまでの間に遅い present(> 8 ms)が走っていたか否かで分ける)。

### 3f. 変種(各 1 差分、計器と同じ木 `srcPm` を土台に `tools/mkvariants.py` で作る。すべて `app-dist-pvN`)
| 変種 | 差分 | 内容 | 自己点検(class/窓の分岐なし、旗/環境変数/URL なし、FL の挙動の代行なし) |
|---|---|---|---|
| **V1**(`diffs/0311-variant-V1-no-frame-publish-while-awaiting.diff`、worker.ts +3) | 入力が答えを待ち、かつゲストが settled でない間は、frame 要求(WANT)起点の scene を publish しない | settled の後の最初の scene が結果の矩形を持つ | 判定は engine Worker 自身の L1c の状態(`inlineSince`・`inlineSettled`)だけ。窓・class を見ない。FL の挙動は変えない(present を数 ms 遅らせるだけ) |
| **V2**(`...V2-one-present-per-frame.diff`、desk-worker.ts +9) | P は 1 つの animation frame に present を 1 回まで。同じ frame で 2 回目は次の frame を待つ | コンポジタの『1 frame in flight』と揃える | P 内の時刻だけ。窓を見ない。答えの scene も次の frame まで待つので T は悪化し得る |
| **V3**(`...V3-commit.diff`、desk-worker.ts +1) | present の後に `OffscreenCanvas.commit()`(その platform に無ければ何もしない) | frame の提出を明示 | platform の能力の有無(`commit` の有無)だけ。窓を見ない。transferControlToOffscreen の canvas では `commit()` が使えない可能性が高く、効果は無い見込み(読み) |

**V4・V5(③④ の後の追加。V5 は §3o で効かず不採用、V4 は §3q で late を減らしたが Esc の T は直らず不採用)**:

| 変種 | 差分 | 内容 | 自己点検 |
|---|---|---|---|
| **V4**(`diffs/0311-variant-V4-answer-by-the-scene-already-shown.diff`、worker.ts +3、`app-dist-pv4`) | IdleDriver が処理した入力(idle 経路)は、injection と settled の後に publish されて**画素を変えた scene が既にあれば**、それで直ちに答える(無ければ次の scene で答える) | 答えの scene の矩形を、`processed` の前に走った別の scene が持ち去ると、次の scene は差分なしで答えが出ない(P の arm の未応答 Esc の候補。経過の記録の読み: 残りの late な Esc 6 件の 5 件は『1.2〜1.3 s 開いたままの経過』= `completeInline` でなく `processed` が awaiting を取って終わった経過に属し、答えが次の scene に回って差分なしになった可能性) | 判定は engine Worker 自身の L1c の状態(`tSettledAt`・`tInject`)と scene の結果だけ。窓・class を見ない。答えを早めに出すだけで FL の挙動は変えない |
| **V5**(`diffs/0311-variant-V5-wake-the-engine-event-loop.*.diff`、desk-worker.ts +2、worker.ts +1、`app-dist-pv5`〔協調 + P の計器つき〕・`app-dist-b1pv5`〔B1 + P の計器つき〕) | P が frame 要求(rAF)と scene の DONE のたびに port で engine Worker に空のメッセージを 1 通送り、engine Worker の event loop を起こして `servicePresent` を走らせる | 実スレッドの Worker 0 は UI スレッドが待つと JSPI の idle に入り、次の deadline(FL の刻み 約 16 ms)まで scheduler 点が無い。P の WANT・DONE は共有メモリのフラグなので見えず、`processed` で publish できなかった答えの scene が約 16 ms 遅れる(B1 + P の Esc +約 20 ms の説明、読み) | ピクセルは送らない(空のメッセージ)。窓・class の分岐なし。旗・option なし。FL の挙動は変えない。メッセージは 100/s 程度 |

### 3g. E14 の受け入れ条件(PM 20:58。以下 3h〜3j は時系列の記録で、最新の判定は 3m・3q)
(1) 純再生の仮想 s/実 s ≥ p0。(2) File クリックと Esc の T の中央値が p0 + 2 ms 以内。(3) 答えの出ない入力が p0 と同数(35/35)。(4) p90 が p0 + 10 ms 以内。(5) Firefox ≥ 145 の fallback が動く(P の能力の判定が偽のときに旧経路へ戻る = 実装済み、Firefox での実行は未確認)。**現状(p): (1) 満たす、(2) Esc で不合格(+3〜8 ms)、(3) 不合格(29〜31/35)、(4) File クリックで不合格(p90 43.3 対 30.8)**。V1 でこの 3 つが直るかを次の窓で測る。

### 3h. 計器つきの 6 本(21:20〜21:53、静かな窓、開始 load 0.5〜2.6。`tables/pt6-table.md`・`tables/ep-analyze.md`)

| run | 開始 load | 純再生 present/s | 仮想 s/実 s | Workers (コア) | File クリック T 中央値 / p90 | Esc T 中央値 / p90 (答えが出た数/35) | present() > 8 ms |
|---|---|---|---|---|---|---|---|
| p0m-1(main + 計器) | 2.25 | 20.1 | 0.845 | [0.85] | 17.6 / 24.3 | 4.3 / 16.9 (35) | 0 |
| p0mt(main + 計器 + 経過の記録) | 0.68 | 20.7 | 0.871 | [0.84] | 17.8 / 25.7 | 4.6 / 13.6 (35) | 0 |
| pm-2(P + 計器) | 0.53 | 32.0 | 1.003 | [0.88, 0.26] | 18.5 / 26.2 | 3.9 / 16.2 (27) | 0 |
| pv1-3(P + V1) | 2.55 | 31.8 | 0.999 | [0.90, 0.26] | 20.1 / 29.9 | 4.9 / 11.7 (27) | 0 |
| pv1-4(P + V1) | 0.80 | 43.3 | 1.002 | [0.93, 0.35] | 18.8 / 30.5 | 9.6 / 28.0 (31) | 0 |
| pmt(P + 計器 + 経過の記録) | 0.86 | 43.7 | 0.996 | [0.93, 0.35] | 16.5 / 24.8 | 11.0 / 28.8 (29) | 0 |

実測の事実:
1. **(1) 仮想 s/実 s ≥ p0: 満たす**(P 0.996〜1.003 対 p0 0.845〜0.871)。
2. **どの run でも `present()` の所要時間は 8 ms を超えなかった**(最大 7.4 ms)= **H2(present() の中で GL が待たされる)は否定**。
3. **P の arm に 2 つの状態が出る(同じ build でも)**: present/s 32 の run(pm-2、pv1-3)は Esc の T 中央値が p0 並み(3.9・4.9 ms)、43 の run(pv1-4、pmt、および前の p-2・p-3・p2-4)は 7.5〜12.7 ms。エンジン Worker・FL の刻み(65/s)・P の frame(55/s)は同じ。原因は未特定。
4. **経過の記録(pmt / p0mt、各入力の inline の答えの経過)**: 経過は pmt 112 件・p0mt 130 件。**settled の前に publish された scene は pmt で 0 件(全 112 経過)** = **H1(矩形の消費)は否定**。変種 V1(settled の前に WANT 起点の publish を止める)も答えの出ない Esc を直さなかった(27・31/35)。
5. **経過の長さ**: 答えが出た経過の『開始から答えまで』は p0mt 中央値 **3.4** ms(p90 16.3)、pmt 中央値 **17.1** ms(p90 27.9)。settled は両方 2.3 ms。settled の後の最初の publish/present は p0mt 3.4、pmt 5.8(p90 17.0)ms。**engine が DONE を見たのは pmt で 17.1 ms = publish の後 約 11 ms**(p0 は同期なので 0)。= **P の arm では publish から P が描いて DONE を書くまでが中央値 約 11 ms 遅い**(`present()` の中ではない)。
6. **答えの出ない入力の原因を 1 つ見つけた(私の実装の穴)**: 答えの出ない Esc(pmt の seq 145・149・169)は、記録に **inline の経過が無い** = IdleDriver が idle で処理した入力(scheduler 点で取られなかった)。その答えは元は `processed` の直後の `flushDeskStats(true, answers)` で出ていたが、P の arm では `flushDeskStats` が先頭で return するため**答えが一切出ない**。修正を実装した(`idleAnswerSeqs`: 処理された入力の seq を、次に publish される scene の完了で、その scene が画素を変えたとき `desk-answers` として送る)。**未測定**(`app-dist-p3`・`app-dist-pmw`、次の窓)。
7. 経過の記録で『答えなしで終わった経過』は pmt 2 件・p0mt 3 件(どちらも約 3.3 s 続いた = 入力の途中に長い停止、別の現象)。

読み(未検証): 5 の約 11 ms は、P の event loop が `Atomics.waitAsync` の resolve を処理するまでの待ち(wake latency)。`OffscreenCanvas` の frame の commit/rAF と同じ Worker thread を取り合う(読み)。次の窓で `app-dist-pmw`(publish の時刻を link に書き、P が起きた時刻との差の分布を数える計器つき)で測る。**6 の修正で答えの出ない Esc が 0 になるか**も同時に見る(受け入れ条件 (3))。

### 3i. 受け入れ条件(PM 20:58)の現状(P のみ、修正 6 の前)
(1) 満たす。(2) File クリックは満たす(16.5〜20.1 対 17.6〜17.8、最大 +2.5)、Esc は 32/s の run で満たし 43/s の run で満たさない。(3) 満たさない(27〜31/35。6 の修正を測る)。(4) File クリックの p90 は満たす(24.8〜30.5 対 24.3〜25.7)、Esc の p90 は 43/s の run で満たさない(28 対 13.6〜16.9)。(5) Firefox の fallback は実装済み・未実行。

### 3j. PM の問い (a)(b) への答え(コードとログの読み。22:00。時系列の記録、最新は 3r 以降)

**(b) 『エンジンの経過は 110/112 が応答なのに、page の Esc は 29/35』**: 経過の記録(`ep-trace`)は **scheduler 点で inline に取られた入力**の経過だけを記録する。page の Esc 35 件を経過に突き合わせると(`logs/rt-ptt-pmt`)、**答えの出なかった Esc down 6 件(late 5・none 1)は全て経過の無い入力**で、経過のある Esc down 26 件は全て答えが出ている。p0mt では経過の無い Esc down 2 件は答えが出ている。= 答えの出ない Esc は **IdleDriver が idle で処理した入力(`processed` の callback)**で、P の arm では**答えを出す経路が無かった**(元の実装は `processed` の直後の `flushDeskStats(true, answers)` で答えるが、P の arm では `flushDeskStats` が先頭で return する)。page の受け取り(`latency.ts`)ではなく、**P の arm の実装の穴**。`desk-answers` の順序は『`input-done`、次に `desk-answers`』で、`answer()` は `tIdle` が決まった記録だけを credit するので競合しない。**修正済み**(`idleAnswerSeqs`)、測定は次の窓(`app-dist-pmw`・`app-dist-p3`)。

**(a) 『32/s の run と 43/s の run』**: 事実だけ(原因は未特定):
- 差は **1 present あたりの画素**に出る: 32/s の run は 0.67 Mpx/present(p0 の 0.68 と同じ)、43/s の run は 0.59〜0.60 Mpx/present。rects/present は両方 1(union の矩形)。FL の UI の刻み(65/s)・P の frame(55〜58/s)・engine の idle/bounce の統計は同じ。SwiftShader 等のスレッド数は 16 で同じ(CPU が 1.9 対 2.6 コア = present が多い分)。
- publish → engine が DONE を見る時間: 32/s の run 2.7〜3.0 ms、43/s の run 9.5 ms。開始 load(0.5〜2.6)、build(同じ build の pv1-3 と pv1-4 で二分)、実行の順序(32/s は 21:26〜21:36 の 2 本、43/s は 20:34〜20:55 と 21:37〜21:47)のいずれにも決定的な対応はない。
- 読み(未検証): present が画素を変える刻みの割合が 58%(32/s)と 76%(43/s)の 2 つの状態に分かれている。因果の向き(P の応答が遅いから present が増えるのか、その逆か)も分からない。次の窓で、P の arm に『公開した scene の数、結果が 0 の scene の数、公開の間隔の分布』を足して分ける(`pmw` は P が起きるまでの遅れ〔wake latency〕の分布を既に数える)。受け入れ条件の判断には、**どちらの状態でも満たすか**を見る必要がある。

### 3k. (PM 22:01 の依頼)mt3 の B1(0312 S1 = 段階 C)の受け入れ測定 — 実スレッド(n=2、`FLE_WORKERS=2`。`FLE_PROCESSORS` は B1 では無視される〔mt3 22:2x の訂正〕)。表: `tables/b1-matched-music300.md`・`b1-rt-arms-10s.md`・`b1-silence.md`、`logs/rt-b1-300/`

B1 = bundle `fl-engine-ci-8301687b`(E13 + 足場 + 16 件の修正 + 段階 C: 音声リングを wasm の共有メモリへ、`patchglue-rt.py` を通さない)+ mt3 の page(`~/work/mt3w-s1/app-dist`)。**製品の形(測定専用の glue 補修なし)での実スレッドの最初の実 Chrome 実行**。`scn-b1-300.json`(300 s の再生 + 最初の 50 s の PCM 捕捉)、22:04:28〜22:13:13、開始 load 0.6、全期間 0.6〜6.1(他の重い job なし)。

| 基準(0289 §11.3 / 0306) | B1 の結果(1 本) | 判定 |
|---|---|---|
| PCM の零の塊が協調並み(patch 有り 3 区間・0.036%、なし 212 区間・19.63%) | **3 区間・790 フレーム(0.036%)**、`[399+111]`・`[45579+381]` は協調・rt11wr と位置も長さも同じ(音楽に固有)、残り 1 本は capture の先頭(`valid=false` の underrun) | **○** |
| 最も重い区間で CPU/Unlock ≤ 85 ms | 30 s 区間の最大 66 ms、10 s 窓の最大 **74 ms** | **○** |
| UI Worker ≤ 0.6 コア | 30 s 区間 0.49〜0.64(平均 約 0.58)、10 s 窓 0.40〜0.65 | **○(際)** |
| 2 Worker の合計 ≤ 1.5 コア | 0.62〜1.43(10 s 窓)、30 s 区間 0.89〜1.34 | **○** |
| 5 分の書き込み ≥ 0.9、12288 の割合が上がらない | distinct **0.978〜1.000**(全 bin)、12288 は **0〜2% で平坦**(rt11 の 1〜7%、E11 協調 e11c の 2〜13% より低い) | **○** |
| 非無音・underrun | 非無音 96.0%(30 s ごと 87〜100%、協調 e11cb と同じ位置で 87%)、underrun 3 quanta・1 回 | **○** |

**入力の真の T(ABBA: p0 / B1 / B1 / p0、22:13:40〜、開始 load 0.73〜1.01。`tables/b1-true-T.md`・`b1-pt-table.md`)**:

| 入力 | E12 本番協調(p0、2 本、n=70) | B1(2 本、n=70) | 差 | B(page → エンジン) p0 / B1 | C(エンジンの処理) p0 / B1 |
|---|---|---|---|---|---|
| File クリック T 中央値 / p90 | 17.9 / 29.5 ms | **28.0 / 35.5** ms | **+10.1** | 1.8 / 6.6 | 15.7 / 21.2 |
| Esc T 中央値 / p90 | 4.9 / 17.3 ms | **13.5 / 21.9** ms | **+8.6** | 1.4 / 6.1 | 3.2 / 7.3 |
| 答えが出た数(Esc) | 70/70 | 68/70 | | | |

= **E11 の実スレッド(0306 §5: +9.3/+8.1、B +4.8・C +5)と同じ T の退行がそのまま**(B +4.8〜+4.7、C +5.5〜+4.1)。**製品の形でも T の受け入れ条件(協調 + 2 ms 以内)は未達**。B は UI Worker の present での占有が主因という読み(0306 §11)に対し、P の arm では B が 1.8 → 0.4〜0.6 ms になった(§3b)ので、**B1 + P(実スレッド版の UI Worker から present を外す)は B の退行を消す**見込み(読み、未測定)。`srcB1P`(mt3 の B1 の木に P の差分を当て、tsc clean、`app-dist-b1p`、port 4987)を用意した。

読み: 実スレッド(n=2)は**製品の形でも数の基準を全て満たす**。測定専用 glue の artefact(零)は段階 C で構造的に消えた(Worker ごとの JS 状態が無い)。12288 の割合が下がったのは、音声の中継の費用(Worker ごとの gap 零埋め・状態の読み)が消えたため(読み)。**入力の真の T は ② で測る**(下)。

### 3l. 穴の修正後の P の ABBA(③、22:35〜22:57、開始 load 0.65〜0.72。`tables/pt3-fixed-table.md`・`pt3-fixed-T.md`)

p0mt-1 → pmw-2 → pmw-3 → p0mt-4(pmw = P + 計器 + 穴の修正 `idleAnswerSeqs` + wake latency)。

| run | 純再生 present/s | 仮想 s/実 s | File T 中央値 / p90 | Esc T 中央値 / p90 (答え/35) |
|---|---|---|---|---|
| p0mt-1 | 21.3 | 0.935 | 17.8 / 22.7 | 4.5 / 15.5 (34) |
| pmw-2 | 34.6 | 1.002 | 18.4 / 29.8 | 4.6 / 17.3 (31) |
| pmw-3 | 34.2 | 1.001 | 18.3 / 27.5 | 6.8 / 22.0 (33) |
| p0mt-4 | 21.1 | 0.883 | 17.6 / 23.0 | 4.1 / 23.2 (35) |
| **p0 2 本の合算(n=70)** | | | **17.8 / 22.7** | **4.2 / 19.0**(69/70) |
| **P 2 本の合算(n=70)** | | | **18.4 / 29.2** | **6.0 / 17.5**(64/70) |

- **答えの出ない Esc**: 修正前 27〜31/35 → 31・33/35。経過のない(idle で処理された)Esc の答えは 8/9(修正前 0/6)に回復。残りは『経過のある episode が page では late』(3 件 + 経過なし 1 件)。p0 も 1 件。
- **wake latency**(publish → P の loop が走り出すまで、n ≈ 2530/72 s): 平均 1.6〜1.9 ms、> 2 ms が 11〜12%、**> 8 ms が 9〜11%、> 16 ms が 3〜4%、最大 25〜43 ms**。= P の event loop が約 1 割の publish で 8 ms 以上待たされる。`present()` の中ではない(全 run で ≤ 8 ms)。**読み(未検証)**: OffscreenCanvas の frame の提出/rAF と同じ Worker thread を取り合う。
- **状態**: 2 本とも present 34.2・34.6/s(32/s 側に近い、1 present 0.65 Mpx、43/s の 0.59 ではない)。

### 3m. E14 の 5 基準(PM 20:58)に対する判定(P のみ、修正後。p0 2 本 対 P 2 本)

| 基準 | 結果 | 判定 |
|---|---|---|
| (1) 純再生の仮想 s/実 s ≥ p0 | 1.001〜1.002 対 0.883〜0.935 | **○** |
| (2) File クリックと Esc の T の中央値が p0 + 2 ms 以内 | File 18.4 対 17.8(+0.6)、Esc 6.0 対 4.2(+1.8。run 別は 4.6・6.8 対 4.5・4.1) | **○(Esc はぎりぎり)** |
| (3) 答えの出ない入力が p0 と同数 | P の Esc 6/70 対 p0 1/70(File は 0 対 0) | **×**(修正前の 8〜6/35 から改善) |
| (4) p90 が p0 + 10 ms 以内 | File 29.2 対 22.7(+6.5)、Esc 17.5 対 19.0 | **○** |
| (5) Firefox ≥ 145 の fallback が動く | 実装済み。**01:13 追記(§3p)**: Firefox 158 で P 動作 ○、能力判定が偽の 2 経路(WebGL2 無し = 実、`Atomics.waitAsync` 無し = Worker の探針で模擬)で旧経路 ○。実際の Firefox < 145 は未実行(模擬のみ) | **○(模擬あり)** |

= 修正後の P は (3)(答えの出ない 5 件)以外の基準に届く。(3) の残りの原因は未特定(in-episode の late)。43/s の状態(Esc の T が +5〜6 ms)は ③ では出なかったので、**2 状態が出る確率**を知るには ABBA をさらに重ねる必要がある(各状態で基準を満たすかが E14 の判断)。

### 3n. ④ B1 + P(mt3 の B1 の木に P の差分を当てた `srcB1P`〔tsc clean、単体テスト 1322 passed、`app-dist-b1p`〕)の ABBA(22:57〜23:18、開始 load 0.9〜1.2。`tables/b1p-table.md`・`b1p-true-T.md`)

| run | 開始 load | 純再生 present/s | Workers (UI / P / mixer, コア) | File T 中央値 / p90 | Esc T 中央値 / p90 (答え/35) |
|---|---|---|---|---|---|
| p0-1 | 1.14 | 21.5 | [0.84] | 17.6 / 22.9 | 4.6 / 17.8 (35) |
| b1p-2 | 1.20 | 51.3 | [0.42, 0.39, 0.61] | 10.1 / 43.3 | 24.8 / 38.6 (34) |
| b1p-3 | 0.91 | 49.7 | [0.39, 0.39, 0.59] | 9.9 / 51.3 | 22.5 / 32.9 (33) |
| p0-4 | 1.19 | 21.2 | [0.84] | 17.2 / 27.3 | 4.0 / 15.9 (35) |
| p0 2 本(n=70) | | | | 17.6 / 25.1(B 1.2、C 15.9) | 4.4 / 15.9(B 0.9、C 3.3)70/70 |
| b1p 2 本(n=70) | | | | **10.0 / 45.8**(B 0.4、C 22.2) | **23.7 / 35.8**(B 0.2、C 3.8)67/70 |

- **B は消えた**(B1 単独 6.6/6.1 ms → 0.4/0.2 ms)。**UI Worker は 0.58 → 0.39〜0.42 コア**(≤ 0.6 を余裕で満たす)。合計 1.40〜1.41 コア(≤ 1.5)。present 50/s。
- **File クリックの T 中央値は協調より速い**(10.0 対 17.6 ms)が p90 が悪い(45.8 対 25.1)。**Esc の T は 23.7 ms**(B+C は 4 ms なのに)= 答えの present が C の終わりから約 20 ms 遅い。**P 単独(協調)の Esc +1.8 ms と違う**ので、実スレッド(Worker 0 の idle)に固有。読み = V5(上)。b1p の T の記録は負の値(−7.8、−4.0 ms)を含み、**credit された frame が入力より前の present の可能性がある**(記録の帰属の不確かさ。計器つきの b1pm で `desk-answers` の経路を確かめる)。
- 計器つきの b1p の run(wake latency)と V5(`b1pv5`)は §3o(23:53〜00:15)で測った。協調の V5(`pv5`)は PM が不採用としたので測っていない。

### 3o. 変種 V5(B1 + P、P が port で engine Worker の event loop を起こす)の ABBA(23:53〜00:15、b1pm → b1pv5 → b1pv5 → b1pm、開始 load 0.46〜1.77。`tables/v5-b1p-*.md`、`logs/rt-ptv-b1pm-*`・`rt-ptv-b1pv5-*`)

| run | 開始 load | 純再生 present/s | Workers (UI / P / mixer, コア) | File T 中央値 / p90 | Esc T 中央値 / p90 (答え/35) | wake latency > 8 ms(P が起きるまで) |
|---|---|---|---|---|---|---|
| b1pm-1(計器のみ) | 1.18 | 50.1 | [0.42, 0.40, 0.64] | 5.8 / 54.2 | 20.0 / 35.7 (31) | 40% |
| b1pv5-2(V5) | 1.77 | 50.0 | [0.44, 0.40, 0.62] | 10.5 / 53.5 | 24.4 / 34.5 (34) | 64% |
| b1pv5-3(V5) | 0.46 | **33.7** | [0.37, 0.28, 0.58] | 22.2 / 47.5 | 11.4 / 22.8 (28) | 10% |
| b1pm-4(計器のみ) | 0.64 | 50.8 | [0.42, 0.41, 0.62] | 10.9 / 54.7 | 11.8 / 39.4 (34) | 40% |

- 2 本ずつ合算(n=70): b1pm = File 7.8 / 54.2(B 0.9、C 23.1)・Esc 14.4 / 39.1(n=65)、b1pv5 = File 21.0 / 48.9・Esc 17.8 / 34.0(n=62)。**V5 は T を良くしていない**(run 間の揺れが差より大きい)。**PM の判断: V5 は採らない。**
- **wake latency(P の loop が走るまで)は V5 で縮まらない**(50/s の状態の 3 本で > 8 ms は b1pm 40%・40%、b1pv5-2 64%)。『publish → engine が DONE を見る』の平均は b1pm 13.2・13.4 ms、b1pv5-2 10.5 ms、b1pv5-3 1.9 ms(33.7/s の状態)。50/s の状態同士の比較で約 3 ms の短縮。**T は縮まらなかった**ので、P の遅れの主因は engine の event loop が寝ている事ではなく、P 側(読み)。
- **2 状態がここにも出た**: b1pv5-3 だけ present 33.7/s・wake > 8 ms が 10%(他の 3 本は 50/s・40〜64%)。開始 load は決め手でない(0.46 の b1pv5-3 は低い状態だが、0.64 の b1pm-4 は 50/s)。協調 + P の 32〜34/s の状態(§3h・3l)と同じ型。
- 未取得: 2 状態を分ける要因。計器(`rafDt` = P の animation frame の間隔の分布、wake latency)を入れた `app-dist-pmd`(協調 + P)・`app-dist-b1pmd`(B1 + P)を作った(port 4981)が、まだ走らせていない。

### 3p. 機能の確認(PM の依頼): Firefox 158 と、P の能力判定が偽になる 2 経路(00:36〜01:12、他の job が走る窓。時間は参考値にしない)

全て `scn-ffsmoke.json`(Firefox)・`scn-psmoke.json`(Chrome): FL を起動 → プロジェクトを開く → 「Problems loading the project」の Ok → Space で 30 s 再生 → 画面の保存。page は `app-dist-p3`(協調 + P、計器なし、port 4997)。run のパスは `logs/`。

| # | 何を動かしたか | 画面 | 結果(実測) |
|---|---|---|---|
| 1 | **Firefox 158.0**(snap、geckodriver、headless、llvmpipe)+ P(`ff-ffsmoke4-p3`、00:50:35〜00:54:50、開始 load 1.12) | FL 全体が出た(メニュー、チャンネルラック、ミキサー、プレイリスト)。再生中の画面で位置 `26:08:01`、ミキサーのメーターが動く | エラーなし(opErrors 空、エンジン `engine-idle`)。first idle 3.8 s、first pixel 約 4.1 s、127 入力が全て処理。page のログ『display method D (present worker)』。再生 30 s の窓: **present 742(24.7/s)、仮想 s/実 s 0.815(busy 0.915)**。**Firefox の p0 との比較(`ffsmoke4-p0`、01:15〜01:19、開始 load 1.75)**: 同じ 30 s 再生の最後の位置は P 26:08:01 対 p0 26:10:11(差 2 step = 約 0.15 s)で、**仮想の速さは同等**。perf の 30 s 窓の仮想 s/実 s は P 0.815(busy 0.915、present 24.7/s)対 p0 1.084(busy 0.803、present 13.6/s)だが、窓の取り方の揺れが大きい(読み)。Worker の CPU(再生 20 s の窓): P は engine 0.98 + P 0.15 + CanvasRenderer 0.27 + llvmpipe 約 0.8、合計 2.26 コア、p0 は engine 0.98 + CanvasRenderer 0.13、合計 1.61 コア(P の方が present が多い分だけ重い)。各 1 本 |
| 2 | Firefox 158 + p0(main の page、`ffsmoke-p0`)・P(`ffsmoke2-p3`、`ffsmoke3-p3`) | ダイアログが出たまま | **私の click の座標の誤り**(1・2 本目はスクリーンショット上の座標、3 本目は見積もりの誤り)で Ok を押せず、再生まで行かなかった。p0 でも P でも同じ状態 = **P のせいではない**。4 本目で `clickIn`(ダイアログの +369,+315)に直して通った |
| 3 | Chrome 156 + `--disable-3d-apis` + P(`rt-psmoke-d3`) | 出た(正しい) | **後退経路を通っていない**: このフラグは main thread の WebGL だけを止め(page の `webglRenderer` が null)、Worker の `OffscreenCanvas` の WebGL2 は使えた。P が動き(Workers 0.97 + 0.32 コア)、仮想 s/実 s 1.003 |
| 4 | Chrome 156 + `--disable-software-rasterizer`(Worker でも WebGL2 が作れない)+ P(`rt-psmoke-nsr`、01:01:47〜01:06:02、開始 load 5.3) | 出た(正しい。位置 `26:09:16`、メーター動く) | エラーなし。**desk.mode が `2d`**(旧経路 = engine Worker の 2D)、Worker は 1 本だけ忙しい(0.90 コア)、P は動いていない。再生の窓: present 474(15.8/s)、仮想 s/実 s 1.002。= **『WebGL2 が無い platform は旧経路』は動く** |
| 5 | Chrome 156 + Worker の探針 `delete Atomics.waitAsync`(`--worker-init tools/worker-init-no-waitasync.js`、製品の木は変えない)+ P(`rt-psmoke-nowa`、01:07:35〜01:11:49、開始 load 3.3) | 出た(正しい。位置 `26:10:20`、メーター動く) | エラーなし。**旧経路で動いた**(DedicatedWorker が 1 本だけ 0.85 コアで、P の分の 0.3 コアが無い。desk.mode は `gl`)。読み: P が `READY` 2(不可)を書き、engine Worker が旧経路へ戻った。再生の窓: present 445(14.8/s)、仮想 s/実 s 0.815。= **Firefox < 145 のように waitAsync が無い platform を模して、旧経路で動く**。実際の Firefox < 145 は**この機械に無く未実行(模擬のみ)** |

- 5 の『P が READY=2 を書いた』は P のログではなく Worker の CPU の形(P の 0.3 コアが無い)と画面の正しさからの読み。確かめるには P に 1 行のログを足す必要があり、足していない。
- 3〜5 の present/s と仮想 s/実 s は、他の job が走る窓で、開始 load が 3.3〜5.9。**性能の比較には使わない**(動いたかどうかだけ)。
- 後退経路が動かない場合の挙動(page の『display method』の表示が P のまま)は、旧版(main)でも WebGL2 が無いとき同じ表示(読み)。

### 3q. V4(製品の形 `pv4s` = V4 を `srcP` に当てた物、計器なし)の ABBA(01:20:50〜01:42:42、p0mt → pv4s → pv4s → p0mt。`tables/v4-pt-table.md`・`v4-true-T.md`、`logs/rt-ptv-{p0mt-1,pv4s-2,pv4s-3,p0mt-4}`)

実 Chrome 156 headless、実エンジン E12(7a9b0800)、協調、この機械(SwiftShader)、依頼者の機械は 0 件、各 2 本。chain 開始時の loadavg は 0.62・3.42・4.44・3.49(2〜4 本目が ≤ 3 でなかった。他の重い job は ps に無し)、cpu snapshot の load は 1.13・0.64・0.91・1.13。

| run | present/s | 仮想 s/実 s | Workers (コア) | File T 中央値 / p90 | Esc T 中央値 / p90 (答え/35) |
|---|---|---|---|---|---|
| p0mt-1 | 19.7 | 0.815 | [0.85] | 17.2 / 29.3 | 5.9 / 20.2 (35) |
| pv4s-2 | 32.0 | 0.999 | [0.89, 0.26] | 18.6 / 29.4 | 7.0 / 17.1 (34) |
| pv4s-3 | 43.2 | 1.000 | [0.94, 0.35] | 17.8 / 26.2 | 15.7 / 33.9 (34) |
| p0mt-4 | 20.6 | 0.843 | [0.84] | 17.7 / 31.3 | 3.8 / 12.8 (34) |
| **p0mt 2 本(n=70)** | | | | 17.3 / 29.9(B 0.8、C 16.0) | 4.4 / 19.5(B 0.9、C 2.8)69/70 |
| **pv4s 2 本(n=70)** | | | | 18.4 / 27.5(B 0.6、C 18.3) | 9.3 / 29.8(B 0.3、C 7.1)68/70 |

| E14 の基準 | 結果 | 判定 |
|---|---|---|
| (1) 仮想 s/実 s ≥ p0 | 0.999・1.000 対 0.815・0.843 | ○ |
| (2) T の中央値が p0 + 2 ms 以内 | File 18.4 対 17.3(+1.1)、Esc 9.3 対 4.4(+4.9) | File ○、**Esc ×** |
| (3) 答えの出ない入力が p0 と同数 | Esc pv4s 2/70 対 p0mt 1/70(V4 の前は 6/70)、File 0 対 0 | **×(差 1 件)** |
| (4) p90 が p0 + 10 ms 以内 | File 27.5 対 29.9、Esc 29.8 対 19.5(+10.3) | File ○、**Esc ×(ぎりぎり)** |
| (5) Firefox の fallback | §3p | ○(模擬あり) |

- **V4 は答えの出ない Esc を 6/70 → 2/70 に減らした**(p0mt 1/70)。残りの原因は未特定。
- **2 状態が V4 でも出た**: pv4s-2(present 32/s)は Esc 7.0 / p90 17.1、pv4s-3(43/s)は Esc 15.7 / p90 33.9。File の T は両状態で p0mt 並み。= **Esc の T は present の状態に左右され、V4 では直らない**。T の判定は 43/s の状態が出る確率に依存する(2 本中 1 本)。状態を分ける要因は未特定(§3h・3l・3o の B1 + P の V5 の ABBA でも同じ型)。
- 次の測定案: `app-dist-pmd`(協調 + P、rafDt = P の animation frame の間隔と wake latency の計器つき)を、2 状態の両方が出るまで 4 本以上。

### 3r. 『状態を決める物』(PM 01:5x の依頼): 既存ログの再解析で仮説を 1 つに絞る(01:48〜01:55、Chrome・Firefox は動かしていない。表: `tables/state-cpu.md`・`state-summary-b.md`・`state-summary-c.md`、ツール `tools/state-cpu2.py`・`state-summary.py`)

**事実 1(実測、cpu-b0/b1 の純再生 20 s): GPU プロセスのどのスレッドも飽和していない**(『43/s で GPU プロセスが 1 コア近く埋まる』は否定)。

| run(状態) | present/s | SwiftShader 16 スレッド合計(コア) | GPU main thread | Viz | renderer main | Workers 合計 |
|---|---|---|---|---|---|---|
| p0mt-1(旧経路) | 19.7 | 1.16 | 0.048 | 0.019 | 0.037 | 0.85 |
| pv4s-2(P、32/s) | 32.0 | 1.89 | 0.091 | 0.027 | 0.037 | 1.14 |
| pmw-2(P、35/s) | 34.6 | 2.05 | 0.094 | 0.028 | 0.025 | 1.16 |
| pv4s-3(P、43/s) | 43.2 | 2.56 | 0.123 | 0.033 | 0.028 | 1.29 |
| b1pv5-3(B1+P、34/s) | 33.7 | 2.01 | 0.094 | 0.027 | 0.024 | 1.22 |
| b1pm-1(B1+P、50/s) | 50.1 | 2.98 | 0.144 | 0.036 | 0.028 | 1.46 |

SwiftShader は 16 スレッドに散り 1 本あたり 0.12〜0.19 コア、GPU main thread は最大 0.14、Viz は 0.04。CPU は present の Mpx に比例(0.088〜0.105 コア/Mpx/s)。機械全体 4〜5 コア/16。P の rAF は全状態で 55〜58/s。

**事実 2(実測、wake 計器のある run の純再生 20 s): 状態は『rAF 1 回あたりに engine が scene を出す割合』と『P が起きるまでの遅れ』で分かれる**。

| run | present/s | scene(publish)/s | P の rAF/s | scene/rAF | wake > 8 ms |
|---|---|---|---|---|---|
| pmw-2 / pmw-3(協調、低い状態) | 34.6 / 34.2 | 38.4 / 39.0 | 58.3 / 58.0 | 0.66 / 0.67 | 1% / 1% |
| b1pv5-3(B1+P、低い状態) | 33.7 | 40.5 | 56.4 | 0.72 | 2% |
| b1pm-1 / b1pm-4(B1+P、高い状態) | 50.1 / 50.8 | 55.8 / 55.5 | 55.8 / 55.7 | **1.00 / 1.00** | 69% / 21% |
| b1pv5-2(B1+P、高い状態) | 50.0 | 56.1 | 56.1 | **1.00** | 74% |

協調の 43/s の run は wake 計器が無く scene/s は未取得(present/rAF は 0.75 で低い状態の 0.58 より高い)。present のうち画素が変わった割合は両状態とも約 0.9 で同じ。c 相(再生 + File メニュー + Esc、約 70 s)でも同じ型(scene/rAF 0.82〜0.84 で wake > 8 ms が 9〜11%、対 0.95〜0.99 で 40〜64%)。

**仮説 H\*(1 つ、読み・未検証)**: 高い状態 = engine が WANT(P の rAF が立てる)に毎回すぐ応えて scene を出し、その publish が P の Worker の『frame step の直後(frame の commit の区間)』に当たる → P の loop が JS を受け付けず、起きるまでの遅れ(wake)が長くなり、入力の答えの scene も同じ遅れを被る。低い状態 = engine の応答が遅れ/まとまり(WANT の約 3 割が次の rAF にまとまる)、publish が P の idle の区間に当たって wake が速い。状態は run ごとに決まる位相(engine の scheduler 点・FL の刻み 65/s と rAF 57/s の関係)で、負荷では決まらない(開始 load 0.46〜1.8)。反証できる予言: (a) 遅い wake(> 8 ms)は『直前の rAF から 0〜5 ms 以内の publish』に集中する、(b) 低い状態では publish が rAF の間に散る。

**測定項目(次の窓、`app-dist-pmd2` = 協調 + P の計器つき、`diffs/0311-measure-only-pmd2-publish-after-frame.diff`)**: publish が P の直前の rAF から何 ms 後か(≤2 / ≤5 / ≤9 / ≤14 / >14 ms)の 5 階級ごとの件数と、その中の wake > 8 ms の件数(予言 (a)(b) を直接見る)。加えて rafDt・wake・scene/rAF・present/s・CPU。2 状態の両方が出るまで 4 本以上(毎回 present/s で分類)。

**確かめる実験 1 本(V7、`tools/mkvariants.py 7`、未 build・未測定)**: P の WANT を rAF callback の中で立てず、callback から MessageChannel で投げた task で立てる(frame step の後)。予言が当たれば高い状態でも wake が速くなり Esc の T が p0 並みに戻る。旗なし・窓の分岐なし・ピクセルなし。ただし V7 は lift-fidelity の 3 分類(契約の実装・余計の除去・通信/変換の削減)のどれでもなく、採用するなら `S-gpu8-N` と PM の判断が要る。

**実 GPU(依頼者の GTX 980)**: 本機の SwiftShader では GPU プロセスの飽和は無かったので『SwiftShader の飽和が原因で実 GPU では出にくい』は成り立たない。frame の commit の区間が実 GPU で短いかは**読み・未確認**(本機で測れない)。

### 3s. 仮説 H\* の検証: pmd2(計器つきの P)と pv7(計器つきの P + V7)(02:16〜02:45、窓の順に pmd2-1・pmd2-2・pv7-3・pv7-4・pmd2-5。`tables/pmd2v7-pt-table.md`・`pmd2v7-true-T.md`・`pmd2v7-state-summary.md`、`logs/rt-ptv-{pmd2-1,pmd2-2,pv7-3,pv7-4,pmd2-5}`)

実 Chrome 156 headless、実エンジン E12(7a9b0800)、協調、この機械(SwiftShader)、依頼者の機械は 0 件。pmd2 = §3r の測定項目を入れた P の page(`app-dist-pmd2`)、pv7 = pmd2 に V7(WANT を rAF callback の中でなく、callback から MessageChannel で投げた task で立てる)を足した物(`app-dist-pv7`、`diffs/0311-variant-V7-want-after-the-frame-step.diff`)。pmd2-1 は opt1 の Wine(02:16:08〜21、nice 19、約 14 秒)が起動直後と重なった**参考**(PM の注記)。run 6(pv7)は省いた(PM の指示)。

| run | 開始 load(chain 開始時) | present/s | 仮想 s/実 s | scene/rAF | wake > 8 ms | File T 中央値 / p90 | Esc T 中央値 / p90(答え/35) | 状態 |
|---|---|---|---|---|---|---|---|---|
| pmd2-1(参考) | 0.58 | 42.4 | 1.000 | 1.00 | 67% | 17.9 / 27.9 | 11.2 / 28.6 (35) | 高い |
| pmd2-2 | 1.99 | 43.7 | 0.997 | 1.00 | 68% | 18.9 / 31.8 | 10.8 / 28.0 (35) | 高い |
| pmd2-5 | 1.95 | 42.9 | 1.001 | 1.00 | 67% | 18.2 / 31.4 | 11.0 / 27.1 (35) | 高い |
| pv7-3 | 2.19 | 31.8 | 1.000 | 0.63 | 0% | 18.6 / 26.4 | 4.8 / 14.8 (35) | 低い |
| pv7-4 | 1.56 | 31.4 | 1.000 | 0.58 | 0% | 18.6 / 26.0 | 4.9 / 19.5 (34) | 低い |

プール: pmd2(3 本、n=105)= File 18.4 / 31.1・Esc 11.0 / 28.0。**pv7(2 本、n=70/69)= File 18.6 / 26.3・Esc 4.8 / 19.5**。V4 の窓の基準 p0mt(2 本、§3q)= File 17.3 / 29.9・Esc 4.4 / 19.5。= pv7 は p0mt に対し File +1.3 ms・Esc +0.4 ms(p90 は File −3.6、Esc ±0)、答えの出ない Esc は 1/70 で p0mt と同数、仮想 s/実 s 1.000。

**予言の判定(読み)**:
- (a) 遅い wake は『直前の rAF から短い時間内の publish』に集まる: pmd2 では publish の 88〜90% が rAF から 5 ms 以内で、その遅い wake(> 8 ms)は ≤2 ms の階級で 63〜64%、≤5 ms で 74〜76%、**≤9 ms の階級でも 56〜67%**、> 9 ms の階級では 0〜24%。= 遅れは rAF の直後 0〜9 ms に落ちた publish に集中し、wake は 8 ms 以上待たされる(9 ms の publish が 8 ms 以上待つなら、P は rAF の約 17 ms 後 = 次の rAF の頃まで JS を受けていない)。**P の frame step が rAF の間ほとんど JS を受け付けない**と読める。
- V7 では同じ位相(publish の 86〜96% が rAF の 5 ms 以内)でも遅い wake が 0%(pv7-3・pv7-4)。= WANT を frame step の後に立てると、engine の publish は P が JS を受けられる区間に当たる。**H\* と合う。**
- (b) 『低い状態では publish が rAF の間に散る』は当たらなかった: 低い状態(pv7)でも publish の多くは rAF の 5 ms 以内。違いは publish の位相でなく、P がその間 JS を受けられるか否か。
- scene/rAF は V7 で 1.00 → 0.6 に下がった(present/s も 43 → 32)が、Esc の T は p0 並みに戻った。scene/rAF が 0.6 になる理由は未解明(読み: V7 で WANT の立つ時刻が遅れ、1 slot・1 in flight の往復と重なる rAF が出る)。pv7-3 だけ rAF の間隔が二峰(≤14 ms が 40%、25〜40 ms が 49%)で、pv7-4 は規則的(≤19 ms が 98%)。どちらも遅い wake は 0%。
- **pmd2 は 3/3 本が高い状態、pv7 は 2/2 本が低い状態**(各 n が小さい。過去の別の日の pmw 等は pmd2 系でも低い状態が出ていたので、pmd2 が高い状態に偏った 3 本だった可能性は排除できない)。

**区分(PM の判断)**: V7 は P の中の WANT を立てる task の順序だけで、FL の挙動も契約も変えない。lift-fidelity の 3 分類のうち (a)『我々自身が足した層〔P の present の経路〕の副作用の除去』。旗・環境変数・窓/class の分岐・ピクセルなし。決定 0311 には『P 層の順序の補正』と書く。

### 3t. 製品の形 P + V7(`pv7s` = 最終の `srcP` に `diffs/0311-present-worker-V7-src.diff` を当てた物、計器なし、V4 なし)の ABBA(02:48〜03:12、p0mt → pv7s → pv7s → p0mt。`tables/pv7s-pt-table.md`・`pv7s-true-T.md`、`logs/rt-ptv-{p0mt-11,pv7s-12,pv7s-13,p0mt-14}`)

実 Chrome 156 headless、実エンジン E12(7a9b0800)、協調、この機械(SwiftShader)、依頼者の機械は 0 件、各 2 本。chain 開始時の loadavg は 1.27・1.23・2.41・2.06(≤ 3)、cpu snapshot の load は 0.67・0.68・1.74・0.88。他の重い job は ps に無し。

| run | present/s | 仮想 s/実 s | Workers (コア) | File T 中央値 / p90 | Esc T 中央値 / p90 (答え/35) | 状態 |
|---|---|---|---|---|---|---|
| p0mt-11 | 20.4 | 0.844 | [0.85] | 17.5 / 24.2 | 4.6 / 20.1 (34) | 旧経路 |
| pv7s-12 | 31.2 | 0.998 | [0.89, 0.26] | 20.1 / 29.2 | 6.9 / 17.8 (32) | 低い |
| pv7s-13 | 30.9 | 0.999 | [0.94, 0.27] | 20.1 / 27.6 | 3.8 / 16.3 (35) | 低い |
| p0mt-14 | 13.8 | 0.629 | [0.89] | 19.2 / 30.1 | 4.4 / 6.5 (35) | 旧経路 |
| **p0mt 2 本(n=70/69)** | | | | **18.9 / 28.4**(B 0.8、C 17.4) | **4.5 / 17.3**(B 0.7、C 3.5)69/70 |
| **pv7s 2 本(n=70/67)** | | | | **20.1 / 28.8**(B 0.5、C 19.2) | **4.8 / 17.8**(B 0.3、C 3.9)67/70 |

| E14 の基準 | 結果 | 判定 |
|---|---|---|
| (1) 仮想 s/実 s ≥ p0 | 0.998・0.999 対 0.844・0.629 | ○ |
| (2) T の中央値が p0 + 2 ms 以内 | File 20.1 対 18.9(+1.2)、Esc 4.8 対 4.5(+0.3) | ○ |
| (3) 答えの出ない入力が p0 と同数 | Esc pv7s 3/70 対 p0mt 1/70、File 0 対 0。全て outcome `late`(150 ms を過ぎてから frame が来た)。pv7s の 3 件は pv7s-12 の 1 run、p0mt の 1 件は p0mt-11 | **×(差 2 件)** |
| (4) p90 が p0 + 10 ms 以内 | File 28.8 対 28.4(+0.4)、Esc 17.8 対 17.3(+0.5) | ○ |
| (5) Firefox の fallback | §3p(模擬あり)。pv7s の Firefox 158 の確認は下 | ○ |

- **状態**: pv7s の 2 本は 2 本とも低い状態(30.9〜31.2/s)。V7 を持つ run は計器つきの pv7(2 本)と合わせて 4/4 本が低い状態、V7 の無い pmd2 は 3/3 本が高い状態(§3s)。
- **答えの出ない(late)Esc の全 run 通算**: P + V7(pv7-3・pv7-4・pv7s-12・pv7s-13)= 4/140、p0mt(p0mt-1・p0mt-4〔§3q〕・p0mt-11・p0mt-14)= 2/140、File は全て 0。差 2 件は雑音の範囲かもしれない(n が小さい。読み)。V4 を付ければ idle 経路の入力の答えは減る(§3q: 6/70 → 2/70)が、PM の判断で V4 は付けない。
- p0mt-14 は present 13.8/s・仮想 s/実 s 0.629 と、p0mt の他の run(19.7〜20.6/s、0.815〜0.844)より遅い。原因は未調査(開始 load 0.88、ps に他の job なし)。(1) の判定に影響しない方向(P の方が速い)。

### 3u. 製品の形 P + V7 の ABBA の延長(PM 03:13〜: pv7s-15 → p0mt-16 → pv7s-17、03:17〜03:36。通算 pv7s 4 本・p0mt 3 本。`tables/pv7s-pt-table.md`・`pv7s-true-T.md`・`late-table.md`、`logs/rt-ptv-{pv7s-15,p0mt-16,pv7s-17}`)

chain 開始時の loadavg は 1.35・2.09・2.22(≤ 3)。他の重い job は ps に無し(PM の tsc・vitest は 03:15 までに終了)。

| run | present/s | 仮想 s/実 s | File T 中央値 / p90 | Esc T 中央値 / p90 | Esc: 答え / late / none (35) |
|---|---|---|---|---|---|
| p0mt-11 | 20.4 | 0.844 | 17.5 / 24.2 | 4.6 / 20.1 | 34 / 1 / 0 |
| pv7s-12 | 31.2 | 0.998 | 20.1 / 29.2 | 6.9 / 17.8 | 32 / 3 / 0 |
| pv7s-13 | 30.9 | 0.999 | 20.1 / 27.6 | 3.8 / 16.3 | 35 / 0 / 0 |
| p0mt-14 | 13.8 | 0.629 | 19.2 / 30.1 | 4.4 / 6.5 | 35 / 0 / 0 |
| pv7s-15 | 29.8 | 0.987 | 22.5 / 29.1 | 11.9 / 21.6 | 33 / 1 / 1 |
| p0mt-16 | 14.8 | 0.661 | 21.2 / 29.9 | 5.1 / 10.6 | 35 / 0 / 0 |
| pv7s-17 | 29.5 | 0.868 | 20.6 / 31.4 | 5.9 / 29.9 | 32 / 3 / 0 |
| **p0mt 3 本(n=105/104)** | | | **19.3 / 28.6** | **4.9 / 13.7** | 104 / 1 / 0 |
| **pv7s 4 本(n=140/132)** | | | **21.0 / 29.2** | **5.9 / 19.7** | 132 / 7 / 1 |

| E14 の基準 | 結果 | 判定 |
|---|---|---|
| (1) 仮想 s/実 s ≥ p0 | pv7s 0.868〜0.999 対 p0mt 0.629〜0.844 | ○(pv7s-17 は 0.868 で他より低い) |
| (2) T の中央値が p0 + 2 ms 以内 | File 21.0 対 19.3(+1.7)、Esc 5.9 対 4.9(+1.0) | ○(File はぎりぎり) |
| (3) 答えの出ない入力が p0 と同数 | Esc pv7s 8/140(late 7・none 1)対 p0mt 1/105(再生中の 35 件ずつ)、File は 0 対 0 | **×** |
| (4) p90 が p0 + 10 ms 以内 | File 29.2 対 28.6(+0.6)、Esc 19.7 対 13.7(+6.0) | ○ |
| (5) Firefox の fallback | §3p・3v | ○(模擬あり) |

- **late の通算**(再生中の Esc 35 件ずつ): P 単独 6/74 → V4 2/72 → 計器つき V7 1/70(pv7-3・pv7-4)→ **製品の V7 8/140**、p0mt = 2/175(p0mt-1・4・11・14・16)。Fisher(両側)は pv7s 8/140 対 p0mt 全 5 本 2/175 で p = 0.026、対 p0mt 3 本 1/105 で p = 0.082、P + V7 全 9/210 対 p0mt 全 2/175 で p = 0.074。**弱い証拠で、V7 で late が約 1% から 4〜6% に増える方向**(多重比較・run 間の揺れがあり、確定ではない。読み)。
- late は run に固まる(pv7s-12 に 3、pv7s-17 に 3、pv7s-13 に 0、pv7s-15 に 2)。late の Esc は全て直前の入力が button up(File メニューを開いた直後の Esc)、エンジンの処理 C は 1.9〜8.8 ms で、エンジンは答えている。その frame が page に 150 ms 以内に credit されなかった理由は未特定(計器つきの V7 の run が 2 本しかなく、ep-trace・wake は pv7s に無い)。V4(idle で処理した入力を既に出た scene で直ちに答える)を付ければ減る可能性があるが、PM の判断で付けていない。
- **Esc T の分布は二峰**: pv7s も p0mt も 2〜6 ms の群と 10〜25 ms の群に分かれ、中央値は後者の割合で動く(pv7s-15 は 11.9 ms、pv7s-13 は 3.8 ms)。V7 は『P が高い状態に入り Esc が +10 ms になる』を 6 本中 0 本に減らしたが(pv7 2 + pv7s 4。pmd2 は 3 本中 3 本が高い状態)、pv7s-15 は低い状態の present/s(29.8)でも Esc の中央値が 11.9 ms。
- p0mt-14・p0mt-16 は present 13.8・14.8/s、仮想 s/実 s 0.629・0.661 と、02:16 以前の p0mt(19.7〜20.6/s、0.815〜0.844)より遅い。pv7s-17 も 0.868 と、pv7s-12・13 の 0.998・0.999 より低い。03:00 以降に機械の状態が変わった可能性(原因未調査、ps に他の job なし)。

### 3v. Firefox 158 + 製品の形 P + V7(`pv7s`、`logs/ff-ffsmoke-pv7s`、03:12:31〜03:17:06、開始 load 2.17)

FL 全体の画面が出た、プロジェクト読み込み・Ok・Space で 30 s 再生、opErrors 空、engine-idle、first idle 4.3 s、page ログ『display method D (present worker)』、再生中の位置 23:16:19 でメーターが動く。再生 20 s の窓: present 34.7/s、Worker は engine 1.00 + P 0.15 + CanvasRenderer 0.25 コア。**ただし速さ**: perf の 30 s 窓の仮想 s/実 s 0.765(busy 0.90)で、同じ時刻の再生位置は 23:16:19。前の Firefox の P 単独 p3 は 26:08:01、p0 は 26:10:11(§3p)= この 1 本は約 11% 遅い。各 n=1、Firefox + llvmpipe で、V7 のせいか run の揺れか(03:00 以降は Chrome でも機械が遅い run が出ている)は決められない。必要なら Firefox で p3 と pv7s を各 2 本。

### 3w. Esc の late の読み(PM 03:37 の依頼、既存ログのみ・Chrome は動かしていない。`logs/rt-ptv-{pv7-4,p0mt-4,p0mt-11,pv7s-12,pv7s-15,pv7s-17}`、`tables/late-table.md`)

**事実(実測)**:
- `outcome = late` は『この入力の答えとして credit された frame が無く、後から別の frame が 150 ms 以内に来た』状態(`latency.ts` の `lateFrame`)。display method D では、答えは P が `desk-answers` で seq を送って credit される(`answer()`)ので、**late/none の Esc = 答えの seq が credit されなかった入力**で、『答えが遅い』ではない。
- ep-trace(inline で取られた入力の経過)を持つ run で突き合わせると、late の Esc は全て **inline の episode が無い**入力(= IdleDriver が idle で処理した入力): 計器つき V7 の pv7-4 の seq 237、p0mt-4 の seq 157、p0mt-11 の seq 143(§3j の pmt の 6 件も同じ)。製品の形 pv7s(計器なし)は ep-trace が無く、突き合わせられない。
- late の Esc のエンジンの処理時間 C(tIdle − tInject): p0mt の 2 件は 0.1・0.2 ms、pv7-4 の 1 件は 0.8 ms、**pv7s の 8 件のうち late 7 件は 1.9〜8.8 ms**(12: 3.9・1.9・8.8、15: 4.4、17: 2.9・4.5・4.7)。p0mt の late は『エンジンが何も描かず終わった入力に別の frame が来た』形、pv7s の late は『エンジンは数 ms かけて描いたが答えが credit されなかった』形に見える(読み)。
- late の Esc は全て直前の入力が button up(File メニューを開いた直後)。

**読み(未検証)**: idle 経路の入力の答えは、処理された入力の seq を『次に publish される scene の完了時、その scene が画素を変えたとき』に `desk-answers` として送る(`idleAnswerSeqs`、§3h-6)。入力の結果の画素を、答えを付ける scene より前に publish された別の scene が先に持ち去ると(矩形の消費)、答えを付ける scene は画素を変えず(result 0)、seq は credit されない。V7 は WANT を frame step の後に立てるため、engine の publish の時刻が変わり、この競合の出方が変わった可能性がある。V4(idle で処理した入力を、injection と settled の後に publish されて画素を変えた scene が既にあればそれで答える、§3f・3q)はこの競合を直す変種で、coop の P 単独で late を 6/70 → 2/70 にした。**確かめる実験**: 計器つきの V7(pv7)に scene 単位の記録(publish 時刻・result・付いた seq)を足し、late の Esc の前後の scene を見る。または V7 + V4 の ABBA。

### 3x. 製品の形 P + V7 + V4(`pv74s` = 最終の srcP + V7 + V4、計器なし、`app-dist-pv74s`、port 4972)の ABBA(05:08〜05:46、p0mt → pv74s → pv74s → p0mt → pv74s → p0mt。`tables/pv74s-pt-table.md`・`pv74s-true-T.md`・`pv74s-late-table.md`、`logs/rt-ptv-{p0mt-21,pv74s-22,pv74s-23,p0mt-24,pv74s-25,p0mt-26}`、`logs/memnote.txt`)

実 Chrome 156 headless、実エンジン E12(7a9b0800)、協調、この機械(SwiftShader)、依頼者の機械は 0 件、各 3 本。各 run の前に `tools/pre-run-note.sh`(free・vmstat の si/so)を残した: si/so は全て 0、MemAvailable 16〜26 GB、chain 開始時の loadavg は 0.46・2.31・2.63・2.03・1.29・2.19(≤ 3)。他の重い job は ps に無し。

| run | present/s | 仮想 s/実 s | File T 中央値 / p90 | Esc T 中央値 / p90 | Esc: 答え / late / none (35) | 再生前の Esc |
|---|---|---|---|---|---|---|
| p0mt-21 | 14.3 | 0.649 | 20.0 / 32.8 | 5.2 / 18.4 | 34 / 1 / 0 | window, window |
| pv74s-22 | 30.0 | 0.896 | 19.7 / 28.1 | 7.8 / 41.3 | 35 / 0 / 0 | window, window |
| pv74s-23 | 30.0 | 0.942 | 21.3 / 27.7 | 5.4 / 19.5 | 35 / 0 / 0 | window, window |
| p0mt-24 | 14.4 | 0.663 | 19.7 / 25.3 | 4.3 / 8.6 | 35 / 0 / 0 | window, window |
| pv74s-25 | 32.2 | 1.000 | 19.8 / 24.1 | 7.6 / 19.2 | 33 / 2 / 0 | **late**, window |
| p0mt-26 | 17.4 | 0.739 | 19.0 / 26.0 | 4.6 / 16.7 | 35 / 0 / 0 | window, window |
| **p0mt 3 本(n=105/104)** | | | **19.7 / 27.2**(B 1.0、C 17.8) | **4.9 / 14.8**(B 0.6、C 3.7) | 104 / 1 / 0 | |
| **pv74s 3 本(n=105/103)** | | | **20.1 / 27.7**(B 0.5、C 19.6) | **6.8 / 19.9**(B 0.4、C 5.0) | 103 / 2 / 0 | |

| E14 の基準 | 結果 | 判定 |
|---|---|---|
| (1) 仮想 s/実 s ≥ p0 | pv74s 0.896・0.942・1.000 対 p0mt 0.649・0.663・0.739 | ○ |
| (2) T の中央値が p0 + 2 ms 以内 | File 20.1 対 19.7(+0.4)、Esc 6.8 対 4.9(**+1.9、ぎりぎり**。run 別: pv74s 7.8・5.4・7.6 対 p0mt 5.2・4.3・4.6) | ○(Esc は縁) |
| (3) 答えの出ない入力が p0 と同水準 | 再生中の Esc: pv74s 2/105(late 2、どちらも pv74s-25)対 p0mt 1/105。File 0 対 0。Fisher 両側 p = 1.0。**再生前の Esc**: pv74s 1/6(pv74s-25 の late)対 p0mt 0/6 | ○ |
| (4) p90 が p0 + 10 ms 以内 | File 27.7 対 27.2(+0.5)、Esc 19.9 対 14.8(+5.1。run 別の pv74s-22 は 41.3) | ○ |
| (5) Firefox の fallback | §3p・3v | ○(模擬あり) |

- **状態**: pv74s の 3 本は 3 本とも低い状態(30.0〜32.2/s)。V7 を持つ run は pv7・pv7s・pv74s の 9 本中 9 本が低い状態。
- **late**: P + V7(V4 なし)の pv7s は再生中の Esc で 8/140、P + V7 + V4 の pv74s は 2/105(p0mt 1/105)。V4 の追加で late が p0 水準に近づいた形だが、n が小さい(pv7s 8/140 対 pv74s 2/105 の Fisher 両側 p = 0.195。読み)。再生前の Esc(dialog に送る Esc)が late になる run が P 側に 2 回(pv7s-12、pv74s-25)、p0 側に 0 回(約 10 run)あり、原因は未調査。
- PM の再計算(再生前の Esc 込み、事象ごと): Esc の late 1/111 対 3/111(Fisher 両側 0.62、3 件は pv74s-25 の 1 本に固まる)。File は PM が button up など他の File 系の入力も数え 30/120 対 11/108(基準側が多い)。私の数え方は『再生中の 35 件ずつの Esc down と File click down』。
- p0mt の present/s と仮想 s/実 s は 03:00 以降の遅い状態(14〜17/s、0.65〜0.74)が続き、02:16 前(19.7〜20.6/s、0.815〜0.844)と違う。基準が遅いので (1) は有利に出る。T の比較は基準が遅い状態での値。
- Esc の T の中央値は pv74s の 3 本とも p0mt の 3 本のどれよりも高い(5.4〜7.8 対 4.3〜5.2)。+1.9 は +2 以内だが、run 別の差は +0.2〜+3.5。

## 5. 自己点検(結果を踏まえた追記)
- P の追加は『待ちの削減』で、エンジンの速度は上がった(0.889 → 1.000)が、**入力の答えの遅れ(Esc +3〜8 ms)という退行を作った**。方針の (a)(b)(c) のどれにも綴じない新しい構造で、PM/オーナーの判断が要る。T の退行を直すか、受け入れ条件(T が協調 +2 ms 以内、0306)を P に課すなら §3b の (a)〜(c) を先に試す。

## 6. 出典なしの選択(S-gpu8-N)
- S-gpu8-1: P の frame loop の sleep 閾値 `DESK_SLEEP_AFTER_MS = 100` は engine Worker のときの値をそのまま使った。
- S-gpu8-2: link は 1 slot・1 scene in flight(複数 slot にして先読みさせる案は、矩形の消費を扱う複雑さに見合わないと判断)。
- S-gpu8-3: 答えの出ない入力の定義(§3d)は page の `latency.ts` の `expire()`(LATE_MS = 150 ms)をそのまま使った。
- S-gpu8-4: T の ABBA は 1 組 = 2 本ずつ(File クリックと Esc それぞれ 35 件 × 2)。run 間の揺れ(±2 ms)より大きい差だけを結論にした。
- S-gpu8-6: 後退経路の確認の探針 `delete Atomics.waitAsync`(`tools/worker-init-no-waitasync.js`)は、Firefox < 145 の実機が無いための代わりで、platform の能力が無い状態の模擬。製品の木は変えない。
- S-gpu8-7: V7(WANT を frame step の後の task で立てる)。PM の区分: P 層(我々自身の追加の層)の中の順序の修正 = 側効果の除去(旗・環境変数・窓/class の分岐・ピクセルなし)。決定 0311 には『P 層の順序の補正』と書く。コメントの「(measured: such a publish waits up to the next frame)」は pmd2 の 3 本の実測(§3s)による。
- S-gpu8-5: 計器は `GlDeskPresenter.present()` の所要時間・link の答えの経路の計数・engine の経過の記録・P の wake latency。製品の差分に入れない(別 diff)。

## 7. 自己申告
1. 20:24:44 に ci1 の 'pure end' の行を終了と読み、smoke を始めた(PM が kill)。2 本目の smoke(20:26)は PM の訂正の info を読む前に始め、ci1 の extra pairs と約 4 分重ねた(ci1/PM に報告済み、ci1 の該当 run は参考扱い)。
2. `timeout 1 true` を含むコマンドを 1 回投げた(ハーネスに拒否された。禁止コマンド。実行はされていない)。
3. 計器の link の Int32(word 16・17)とメッセージの Float64(presentAt、byte 64〜71)が重なる私のバグで、pm-2・pv1-3 の `ansSkipBusy`・`ansEarlyShown` は無効(21:31 に気づき、word 20・21 へ移した)。T の値には影響しなかった(presentAt の下位 bit が変わる程度)。
4. 最初の修正版 p2 は『答えの試行の間隔が原因』という誤った読みで作り、T を直さなかった。矩形の消費(H1)と present() の GL 待ち(H2)の2つの読みも実測で否定された(§3h)。真の原因の1つ(IdleDriver の idle 経路の入力に P の arm が答えない)は私の実装の穴だった。
5. プレビュー server を `timeout` 付きの tool 呼び出しで起動して killed にした(21:09、起動し直した)。run の結果には影響しない。
6. 窓の規則: PM の info『window gpu1』を待って chain を始めた。ただし 21:18 に opt1 の Wine の batch が走っていたため、PM の指示で待ち、21:20:11 の終了と ps の確認の後に始めた。
7. Firefox の機能の確認 3 本(`ffsmoke-p0`・`ffsmoke2-p3`・`ffsmoke3-p3`、各 約 6 分)は私の click の座標の誤り(スクリーンショット上の座標を使った)でダイアログの Ok を押せず、再生まで行かなかった(p0 でも同じなので P のせいではない)。4 本目で `clickIn`(ダイアログ基準)に直して通った。Firefox 約 15 分を無駄にした。
8. 後退経路の確認の 1 本目(`--disable-3d-apis`)は後退経路を通っていなかった(Worker の OffscreenCanvas の WebGL2 は使える)。`--disable-software-rasterizer` と Worker の探針に替えた。
9. 00:16:10 に V6(2 frame に 1 回の present のテスト)の Chrome run を、PM の『batch の窓は Chrome なし』の info を読む前に始め、00:16:50 に自分の PID だけを止めた(run のディレクトリは削除、結果は使っていない)。PM に報告済み。
10. PM への info『状態を決める物』の見出しの時刻を 02:05 と書いたが、実際は 01:53(時計を見ずに書いた誤記)。内容の数値は logs/ と tables/ の実測。
11. V4 の計時の chain 開始時の loadavg は 2〜4 本目が 3.42・4.44・3.49 で、PM の『開始 load ≤ 3』を満たさなかった(chain の中で自分の前の run の loadavg が残るため。run 間で load を見て待つ仕組みを入れていない)。PM に報告済み。
12. 03:12:31 に Firefox 158 の pv7s の機能確認を、PM の元の手順 (4)(ABBA の後に Firefox)に従って始めたが、03:13 の PM の info(Firefox は延長 ABBA の後)を読む前だった。Chrome とは重ねていない(03:17:06 終了、03:17:20 に Chrome 開始)。PM に報告済み。
13. 時刻の誤記: PM への info『状態を決める物』の見出し 02:05(実際 01:53)。ほかに、V4 の chain の 2〜4 本目の開始 load が 3 を超えた件(11)。
14. `vitest` を最初に木の外の `../lift` を読むテストが失敗する形(10 件失敗)で実行した。一時的に `srcP/lift` → 本体の `lift` の symlink を張って 1321 passed に直し、実行後に削除した(失敗は木の配置の問題で、V7 の差分とは無関係)。
15. 03:2x に `sleep 0` と末尾 `&` を含むコマンドを 1 回投げた(ハーネスに拒否され、実行されていない。禁止コマンド)。


## PM の判断と取り込み(2026-11-06 23:26、docs のみ。製品の差分なし。gpu1 の途中版〔V5 の実測前〕の取り込み。最終版が出たら本文を差し替え、この節は残す)

取り込み: gpu1 の 0311(present を別 Worker P に出す案)の文書・表・道具・差分(`docs/investigations/gpu8/{tables,tools,diffs}`、生ログ `logs/` 〔約 21 MB〕は入れない。原本は `lift/out-git/gpu8-results/logs/`)を置いた。実 Chrome 156 headless で実エンジンを動かした実測(E12 本番の wasm 無変更、および mt3 の B1 bundle `fl-engine-ci-8301687b`)。**依頼者の機械は 0 件、本番 R2 の `perf/` に beacon は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷(G9 未到達)での値。SwiftShader、各腕 1〜2 本**。

PM の検証:
1. gpu1 の最終の差分 3 本(`0311-present-worker-src.diff`〔穴の修正 `idleAnswerSeqs` 入り〕・`-test.diff`・`-tsconfig.diff`)は、main 5043915f の `git archive HEAD app` の複製に `patch --batch` で reject 0 で当たる(src/test は -p0、tsconfig は app/ の中で -p2)。`npm run typecheck`(4 構成)rc=0。`npx vitest run --maxWorkers=1`(APP_REPO_ROOT = lift/runtime と lift/harness/web の取り出し)で 70 files passed・1 skipped、**1,320 passed・8 skipped・0 failed**。実 Chrome での e2e は未実施。
2. 協調 + P(穴の修正後、p0mt 2 本 対 pmw 2 本。PM が `lat-abba.py` と outcomes で再計算して gpu1 の表と一致): 純再生 present 21 → 34/s、仮想 s/実 s 0.883〜0.935 → 1.001〜1.002、エンジン Worker 0.84 → 0.89 + P 0.27、File T 17.8 → 18.4(+0.6、p90 +6.5)、Esc T 4.2 → 6.0(+1.8、プール。pmw-3 単独 +2.7)、Esc の答えの出ない入力 1/74 → 6/74(File は 0)。present() は最大 7.4 ms。P の event loop の wake latency は publish の約 10% で 8 ms 超(最大 43 ms)。
3. B1 + P(`ptv` の ④、p0 2 本 対 b1p 2 本): present 21.4 → 50.5/s、UI Worker 0.84 → 0.39〜0.42(+ P 0.39、mixer 0.59〜0.61)、**B は 1.2 → 0.4 ms に消えた**が、File T は 17.6 → 10.0 ms(p90 25.1 → 45.8)、**Esc T は 4.4 → 23.7 ms**(p90 35.8、答え 67/70)。Esc の答えが C の終わりから約 20 ms 遅い。gpu1 の読み(V5): 実スレッドの Worker 0 は idle の合間(約 16 ms)に P の WANT/DONE の共有フラグを見ない。

E14 の 5 基準(PM 20:58)の判定(P 単独、協調):
| 基準 | 結果 |
|---|---|
| (1) 純再生の仮想 s/実 s ≥ 基準 | 満たす(1.001〜1.002 対 0.883〜0.935) |
| (2) File・Esc の T の中央値が基準 +2 ms 以内 | 満たす(File +0.6、Esc +1.8。Esc は境界で、pmw-3 単独は +2.7) |
| (3) 答えの出ない入力が基準と同数 | **満たさない**(Esc 6/74 対 1/74。穴の修正で idle 処理の入力の答えは 0/6 → 8/9 に戻ったが、残りは『エピソード内の遅い答え』) |
| (4) p90 が基準 +10 ms 以内 | 満たす(File +6.5、Esc は基準以下) |
| (5) Firefox 145 以上で動き、古い版・WebGL2 なしで後退する | **実装済み・未実行**(Firefox 158 の P の smoke と、後退経路の実行が要る) |

判断:
1. **E14 はまだ push しない**。(3) と (5) の実行、ci1 の first idle(E13 の本番 bundle + P の page、交互 10 組、`lift/out-git/ci1-results/0277/e14/PLAN-e14-first-idle.md`)、e2e、Firefox の smoke が要る。V4(`...V4-answer-by-the-scene-already-shown.diff`、worker.ts 27 行)と V5(`...V5-on-srcP.diff`、+3 行)は読んだ(旗・環境変数・URL パラメータ・FL の class/窓の分岐・FL の挙動の代行なし。区分: 通信の経路の修正)。**未測定**。V1・V2・V3 は効果がなかった/原因でなかった(V1 は無効)。
2. 次の Chrome 窓(PM の info、約 00:00〜00:25、gpu1): 計器つき b1pm → b1pv5 → b1pv5 → b1pm の 4 本(B1 + P の Esc の約 20 ms を ep-trace と wake latency で確かめつつ V5 を当てる)。V5 が B1 + P の Esc を直せば、同じ V5 を協調 + P(pv5 対 p0mt 各 2 本)に当てて未応答 0 の確認を兼ねる。ci1 の E13 pure はその後。
3. 実スレッド版(B1)は、**B1 + P で T の B が消えたこと**は確認できたが、Esc が基準 +2 ms 以内でない間は main に出さない(決定 0312 の PM の節のまま)。S0+S1+P の bundle は B1 の bundle に『積み重ね + P』の page を組み合わせる(決定 0312f)。gpu1 の `app-dist-b1p` と mt3 の `mk-s0s1p-page.sh` の作りが同じかの突き合わせは未了。
4. 未確認: V4・V5 の効果、32/s と 43/s の二状態の原因(P 単独の pmw は 34/s の側だけ)、Firefox での P、older browser の後退の実行、実機。


### 追記(2026-11-07 03:42、gpu1 の最終版〔V5・V7・製品の形の ABBA 7 本・Firefox 158〕の取り込み、docs のみ。製品の差分は push していない)

本文を gpu1 の最終版(03:45。V4・V5・状態の再解析 H*・V7 の計器つき 4 本と製品の形 7 本の ABBA・Firefox 158 の機能確認と後退経路・late の読み)に差し替えた。上の 11-06 23:26 の節は残す。**実エンジン(E12 7a9b0800)・実 Chrome 156 / Firefox 158・この機械・headless・模擬入力。依頼者の機械は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷での値。**

PM の検証(自分で lat-abba.py・pt-table.py・state-summary.py・outcome の数え直し): (1) 製品の形の P + V7 の ABBA 通算(基準 p0mt 3 本 n=105/104、P + V7 4 本 n=140/132): File の T 19.3/28.6 → 21.0/29.2 ms、Esc 4.9/13.7 → 5.9/19.7 ms、仮想 s/実 s 0.629〜0.844 → 0.868〜0.999、present 13.8〜20.4/s → 29.5〜31.2/s。Esc の late は V7 8/140〔gpu1 の数え方。再生中 35 件ずつ〕対 基準 1/105(Fisher 両側 0.082)。File の late は V7 の方が少ない。(2) PM の木(main の app + src・test・tsconfig・V7 の 4 diff)で tsc 4 構成 rc 0、vitest 70 files・1,320 passed・8 skipped・0 failed。(3) Firefox 158 + P + V7 の機能確認(画面・読み込み・30 s 再生・opErrors 空・first idle 4.3 s・present 34.7/s)を driver のファイルで確認。ただし perf の窓の仮想 s/実 s 0.765・再生位置が P 単独より約 11% 遅い(n=1)は未決。

**判断**: (a) **区分は『P 層(我々自身の追加の層)の中の順序の補正』**(lift の忠実性の方針: 我々が足した層の副作用の除去。旗・環境変数・URL 引数・窓や class の分岐・画素の処理なし)。(b) **E14 = P + V7(V4・V5・V6 なし)を本命にするが、まだ push しない**。理由: Esc の late の数が基準と同数でない(8 対 1、両側 p≈0.08)。gpu1 の読み(§3w、既存ログのみ): late の Esc は idle 経路の入力で、答えが『次に publish される画素の変わる scene』に付くため、先に別の scene が矩形を持ち去ると失われる形(計器つきの突き合わせでは late は全て inline の episode が無い)。V4(idle で処理した入力を、すでに表示した scene で答える)がこの形を直す方向で、P 単独では late を 6 → 2/70 にした。(c) **次の実験: P + V7 + V4(製品の形、計器なし)の ABBA を同じ窓で 3 本**(基準との交互)。V4 は V7 とは別のファイル(worker.ts)で衝突しない。V4 を加えても Esc の T が基準 +2 ms 以内・late が基準並み・仮想 s/実 s ≥ 基準なら、それを E14 とする。(d) 基準側が 03:00 以降 13.8〜14.8/s(仮想 0.63〜0.66)と 02:16 以前の 19.7〜20.6/s より遅くなっている(原因未調査)ので、E 間の比較は同じ窓で交互に取る。(e) 残り: ci1 の first idle(Chrome 10 組・Firefox 6 組)、scenes・Export・alpha7(ci1 が 5063 の page-e14-PV7 で実施中)、Firefox で p3 と pv7s の速さの対(各 2 本)、e2e、再検証、deploy CI が走っていないこと。


### 追記(2026-11-07 05:54、gpu1 の最終版の再取り込み〔P + V7 + V4 の製品形 ABBA 3+3 本を含む〕と PM の判断、docs のみ。製品の差分は push していない)

本文を gpu1 の最終版(473 行)に差し替えた(11-06 23:26 の節と 03:42 の追記は残す)。**実エンジン(E12 7a9b0800)・実 Chrome 156・この機械・headless・模擬入力。依頼者の機械は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷での値。**

PM の検証(lat-abba.py・pt-table.py・state-summary.py・outcome を自分で数え直し): 製品形の P + V7 + V4(pv74s-22/-23/-25)対 基準 p0mt(-21/-24/-26)、05:08〜05:45。File の T 19.7/27.2 → 20.1/27.7 ms(+0.4/+0.5)、Esc 4.9/14.8 → 6.8/19.9 ms(+1.9/+5.1)、仮想 s/実 s 0.65〜0.74 → 0.90〜1.00、present 14〜17/s → 30〜32/s、wake > 8 ms 0%。Esc の late(再生前を含む私の数え方)は基準 1/111 対 P + V7 + V4 3/111(Fisher 両側 0.62)で有意でない。基準側は 03:00 以降の遅い状態(14〜17/s)のまま。

**判断**: (1) **E14 の最終の組は P + V7(desk-worker.ts のみ、+4/−1)**。V4(worker.ts)は採らない(Esc の中央値の差が +1.9 ms と P + V7 の +1.0 ms より大きく、P + V7 の late の差も有意でない。変更を最小にする)。(2) E14 はまだ push しない。残る関門: ci1 の first idle(Chrome 10 組・Firefox 6 組)と e2e(page-e14-PV7、port 5063)、Firefox で p3 と pv7s の速さの対(各 2 本)、再検証(検証木 lift/out-pmchk-e14f/tree で tsc 4 構成 rc 0、vitest 1,320 passed・8 skipped・0 failed 済み)、deploy CI が走っていないこと。(3) 再生前の Esc の late が P 側で 2 回(pv7s-12、pv74s-25)、基準側で約 10 run に 0 回: 原因は未調査(gpu1 の宿題)。
