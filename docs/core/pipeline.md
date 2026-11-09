# lift パイプライン(x86-64 → C/wasm → bundle)

冒頭: (1) 下は旧資料の読みで、**この機械でビルドも実行もしていない**([未確認])。(2) 手順の正本は `app/scripts/build-engine-bundle.sh` の冒頭コメント(予定: 持ち込み後。決定 0243)。(3) 区分: [文書]=旧決定の記述、[実測: コマンド]=この文書の作成時に読んだ事実。

## 1 構成と流れ

```
FL インストーラ(NSIS) --tools/extract_nsis2.py--> FLEngine_x64.dll + 同梱 DLL 7 本(計 8 モジュール)
 fe-pe(PE32+ 読み)→ fe-decode(iced-x86 のデコードと到達解析)→ fe-lift(x86-64→C)/ fe-wasmgen(x86-64→wasm 直)
 → emcc(C→wasm、JSPI・SJLJ=wasm・共有メモリ)→ wasm-opt -Oz → wasm-merge(直 wasm を結合)
 → pack-assets.mjs → bundle(wasm・glue・DLL pack・seed pack・manifest.json)→ ページ(app/)の Worker が実行
```
製品の構成は **1 つ**(決定 0243: プリセット・オプションなし)。8 モジュール = FLEngine・dsp_ippv2・Blend2D・QuickFontCache・FreeType・ILTools・oggio・Elastique。

## 2 `build-engine-bundle.sh` の 9 段(各段は出力があれば飛ばす)

| 段 | 内容 | 出典 |
|---|---|---|
| gen | cargo で fe-lift を作り、`lift/harness/b2d2/gen-direct.sh` で 8 モジュールの C を生成。Blend2D のコンテキストは上流 Blend2D に置換、dsp の熱い関数はホスト実装、使われない IPP 変種は除外 | 0160・0162・0168 |
| resplit | 生成 C を 10 MB の単位に切る。レジスタファイルを固定アドレス(0x1000)へ | 0197・0243 |
| wasm | `build_nmod_wasm.sh`(重い段) | 0243 |
| hybn0 / weblink / wasmopt / hybrid | fe-wasmgen を仮アドレスで走らせ、emcc でリンク、`wasm-opt -Oz`、実アドレスで直 wasm を作り `wasm-merge` | 0157・0230 |
| seed / bundle | seed pack(FLSEEDP1: ゲストのホストファイル・レジストリ・claims)、`pack-assets.mjs` | 0243 |

- 環境: emcc 6.0.12・wasm-opt 133(`/opt/emsdk`。`source scripts/env.sh` が PATH に置く)・rustc 1.99.0・node 24 [実測: `docs/ops/environment.md`、R1 2026-11-09]。
- 必須の入力: 抽出済み DLL(`HOST_ROOT`)、`B2D_UPSTREAM_LIB`・`B2D_UPSTREAM_SRC`(`docs/core/inputs.md` §4)。**`WORK`・`JOBS` などはパスと数値で、何が作られるかを変えない**。他の環境変数・旗を足さない。
- 旧記録の所要: CI の段スクリプトで冷間 約 17〜22 分(16 コア。`JOBS=4`)。この機械では未測定 [未確認]。メモリ 7〜8 GB と数 GiB のディスク [文書: 旧 CI のコメント]。

## 3 決定の要約(出典番号)

| 決定 | 要約 | 区分 |
|---|---|---|
| 0157 | `fe-wasmgen`: 機械語から wasm を直接生成(試作として書かれたが、**今は本番の段**)。freetype 1,639 関数で C 経路との差分試験 482,464 反復・不一致 0。サイズ 2.73 倍(C 経路 -Oz は 5.20 倍) | [文書] |
| 0160 | Blend2D は「コア約 41 入口の全置換」が成り立たず、**コンテキストの 18 入口だけ**を上流 no-JIT に置換(`fe-lift --swap-entries`)。libm は DLL の関数へ橋渡し | [文書] |
| 0162 | dsp の熱い関数 `f_33d6e70`(IPP `ippiWarpAffineQuad_8u_C4R` の内側)を HLE。起動 CPU −29%(native)。**lift したエンジンは IPP の添字 0 の本体(CPUID=SSE2)を走らせ、AVX 系では実 DLL が選ぶ本体と画素が最大 1 違う**(Wine での観測) | [文書] |
| 0168 | CPUID 固定で選ばれない IPP の CPU 別コード(強 S・中 M)を C から外す。wasm 233.9→155.8 MB(当時) | [文書] |
| 0132 | ゲストの SEH(Delphi の try-except/finally)を実機の規則で実行(`fle_seh.c`、着地ラッパー) | [文書] |
| 0230 | 混成(直 wasm)を CI の段にした。リスト `lift/harness/seeds/hyb-U-<m>.txt`(8 ファイル、`0x` 行の合計 3,875 [実測: `grep -c`])。旧決定は 3,879/3,813 と書く | 食い違い、実測を採る |
| 0243 | 約 480 件のオプションを畳み、構成を 1 つにした。wasm 141,795,274 B(当時)。**畳んだ後の node 行は `FLE_READFILE_EOF_TRUE` の 1 点で旧と違う** | [文書] |

## 4 ホストとエンジンの境界(読み。決めない)

- ホスト `FL64.exe` は薄い。やる事は `LoadLibraryW` → `GetProcAddress("CreateFruityInstance")` → `(0xC0, 0xEDE95A, host, &instance)` で呼ぶ → 戻りで vtable[0]。ホストオブジェクト 0x38 B(`+8`=版 1、`+0xc`=Flags 0)、vtable 3 スロット [文書: `host-engine-boundary.md` §0・§9]。
- 重いのはホストの外: `CreateFruityInstance` は `Flags=0` で `TApplication.Run` 相当のブロッキングのメッセージループを内側で回す(JSPI で中断可能にする)。
- スレッドは **協調スレッド(T2。wasm は JSPI)** を最終形と推奨(`thread-model.md` §8 の読み。実装は 0122、JSPI は 0124)。本番(E13)は協調 1 Worker。実スレッド版(RT 系、0234 ほか)は本番に入っていない [文書]。
- 2 モジュールのリンク: 生成 C の重複定義(`R`・`X`・`FL` ほか 7 個)を 1 つに統合。全部入りの DLL は wasm の 1 関数 50,000 locals の上限に当たる(最大 389,948)ため、縮小して分ける [文書: `two-module-design.md`]。

## 5 注意

- 上のサイズ・時間は旧環境の値。S1(wasm が native の 2 倍以下)は `docs/core/performance.md`。
- 差分(未適用の契約修正・除去)は `docs/ROADMAP.md` の R1〜R3。`FLENGINE_PE` を設定しない `cargo test` は実 DLL の試験が `SKIPPED` で**成功扱い**になる [文書: 旧 README]。必ず設定し、`SKIPPED` を数える。
