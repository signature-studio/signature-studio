# 7 種の FL 内蔵モジュールの確認状況（2026-11-08）

状態: **仮**（静的な調査のみ。docs を 1 本書いた。コミットは PM）。基準は main の HEAD `76407c72`。

区分: **実測**＝この作業で読んだログ・ファイルの内容そのもの。**文書の記録**＝決定文書や docs に書かれた値（この作業では再計算していない）。**読み**＝解釈。

## 0. 実際に実行した物

実行したのは読み取りと検索だけ。wasm の build、Chrome、Wine、node、cargo は**実行していない**。

読んだ docs（全文または該当の行範囲）:

- `docs/milestones/performance-targets.md`（grep。26 行）
- `docs/milestones/roadmap-to-goal.md`（25〜69 行）
- `docs/progress.md`（grep。51・68 行）
- `docs/investigations/goal-ladder.md`（1〜12 行）
- `docs/decisions/0309-g9-entry-estimate.md`（1〜168 行、全文）
- `docs/decisions/0313-g9a-balance-module.md`（grep と範囲読み。9・16・128〜129・142・191〜194・218〜220・224・449〜452・514・801〜803・821・842〜845 行）
- `docs/decisions/0315-delay2-step-and-next-build-candidates.md`（grep。9・16〜22・38・48〜51 行）
- `docs/decisions/0315e-frequency-shifter-step.md`（1〜51 行、全文）
- `docs/decisions/0319-owner-observations-contract-reading.md`（grep。385〜454・624〜639 行）
- `docs/decisions/0320-opt1-resource-inventory.md`（grep。476〜491・602〜605・905〜907 行）
- `docs/decisions/0331-rtb-chrome-plan.md`（grep。3・20 行）
- `docs/investigations/gpu8/0313-V3-chrome-g9a.md`（1〜24 行）
- `docs/investigations/gpu8/0319-cand1-functional-checks.md`（1〜40 行）
- `docs/investigations/ci1/0331/REPORT-0331.md`（1〜14 行）
- `docs/investigations/gpu8/tools/scn-rtb-v3.json`、`rtb-read.py`（grep）
- `docs/feedback/2026-11-07-owner-observations.md`（grep）
- `docs/decisions/INDEX.md`（1〜123 行。索引として読んだ。本表の根拠には使っていない）

読んだログ・結果（RT-B の V3 の run。Chrome 156、実エンジン）:

- `lift/out-git/gpu8-results/0331/logs/rtb-v3/engine.log`（20〜21・263〜265・278〜285・2202〜2207・2310〜2315・2453・2462〜2476・2523〜2524・2804 行を読んだ。`fle: unsupported` は grep で 0 件）
- `lift/out-git/gpu8-results/0331/logs/rtb-v3/rtb-read.txt`（全体）
- `lift/out-git/gpu8-results/0331/logs/rtb-v3/report.json`（先頭と 88 行付近）、`report.md`、`stdout.log`
- `lift/out-git/gpu8-results/0331/logs/rtb-v3/problems-pic.png`（画像として読んだ）
- `lift/out-ci1/kit/out/rtb-export-*`、`lift/out-git/gpu8-results/0331/logs/rtb-o1b/`（存在の確認のみ。ls）

git（読み取りのみ）:

- `sed -n '48p' app/scripts/build-engine-bundle.sh`（ワークツリー。HEAD と同じ行）: `EXTRA_MODULES="oggio elastique"; NEXTRA=2`
- `ci/extract-dlls.tsv` の数: 全 11 行、コメント 3 行、本体 8 行（`grep -v -E '^\s*(#|$)' | wc -l` = 8）。Fruity / Soundgoodizer / FLEX / Frequency / Parametric を grep: 0 件
- `git grep HEAD` で 7 種の名前を `app/scripts`・`ci`・`lift/harness/seeds`・`lift/harness/b2d2/gen-direct.sh` から検索: 0 件（ヒットは無関係なフォントのバイナリだけ）

画像: 依頼に添付された FL の画面（Problems 窓。「5 plugin(s) could not be opened」、一覧は Fruity Wrapper / FLEX / Frequency Shifter / Soundgoodizer / Fruity Parametric EQ 2）。

## 1. 結論（先頭）

1. 7 種（`performance-targets.md:26` の一覧）のうち、**Balance・Filter・Delay 2 は、テスト bundle（診断入り・push 前）で wasm の読み込み（`LoadLibraryW` → `GetProcAddress("CreatePlugInstance")`）まで実行済み**。これは **main に入っていない**テスト bundle の上の確認である（§1 の 2）。Balance と Filter は Export を Wine の原版と照合した（結果は表のとおり）。Delay 2 の Export の照合は文書の記録のみ。
2. **main（HEAD `76407c72`）には 7 種のどれも入っていない**。根拠は次の 2 つ。(a) `app/scripts/build-engine-bundle.sh:48` の `EXTRA_MODULES="oggio elastique"; NEXTRA=2`（decision 0230 の modules 6 と 7 のみ）。(b) `ci/extract-dlls.tsv` は本体 8 行（コメント 3 行を除く。FLEngine_x64.dll・dsp_ippv2_x64.dll・Blend2D.dll・QuickFontCache_x64.dll・freetype_x64.dll・ILTools_x64.dll・oggio_x64.dll・elastique_x64.dll）で、この tsv が読むのは FLEngine と dsp・Blend2D など。**FL のプラグイン DLL は入っていない**。上の「実行済み」は、テスト bundle の上での確認である。
3. 残り 4 種（Fruity Parametric EQ 2・Frequency Shifter・Soundgoodizer・FLEX）は、**wasm では読み込まれていない**。RT-B の V3 では Problems 窓に 1 行ずつ残る。
4. 依頼の画像の 5 行は、RT-B V3 の結果（件数 5、窓 691×295、位置 614,392）と一致する（`rtb-read.txt:9`、`report.json:88`）。Balance・Filter・Delay 2 の行は無い。**画像とファイル `problems-pic.png` のバイト照合は行っていない**（件数・窓の位置・名前の一致だけを確かめた）。
5. 古い記述（確認できた範囲。直していない）:
   - `docs/milestones/roadmap-to-goal.md:156`（G9 の行）の「同梱 160 種は未 lift」。この行は `roadmap-to-goal.md:129` の 2026-11-03 の版（履歴。基準 HEAD 7757f60。『更新されていない』と明記）の区分表にある。Balance・Filter・Delay 2 は、この版より後にテスト bundle で lift・読み込みまで確認されている（§2 の表）。
   - `docs/milestones/performance-targets.md:26` は、ミキサーのプラグイン数の記述で古い記述ではない（PM の確認）。ここには書かない。

## 2. 表（7 種）

| モジュール名 | 状況 | 根拠ファイルと行 | 次の作業 |
|---|---|---|---|
| Fruity Balance | **実行済み（テスト bundle、Chrome 156）**。`LoadLibraryW` → `0x6800000`、`GetProcAddress("CreatePlugInstance")` → `0x6abdde0`、`fle: unsupported` 0 件（実測）。Export は、原版 FL（Wine）と 5,715,911 フレーム中 5 フレームだけ ±1 LSB（文書の記録）。GUI は client 領域が描かれない（未解決）。HEAD 未収録。lift は CI の段（test build wasm 150,851,575 B）と fe-lift 単体（`0309` の表）。 | 実測: `lift/out-git/gpu8-results/0331/logs/rtb-v3/engine.log:2453`（LoadLibraryW。内容は `g9-diag: LoadLibraryW "…Fruity Balance_x64.dll" -> 0x6800000` と一致）、`rtb-v3/rtb-read.txt:6`（PASS）、`engine.log:263`（seed の行）。文書の記録: `0313-g9a-balance-module.md:128〜129`（初回の V3）、`:821`（Export の照合）、`:451`・`:845`（GUI）。`progress.md:51`（push 前）。`0309-g9-entry-estimate.md:48`（fe-lift）。HEAD: `app/scripts/build-engine-bundle.sh:48`。 | 【実行】GUI の client が描かれない原因の切り分け。`0313-g9a-balance-module.md:845` の `diag-g9-fl-paint` を次の test bundle で 1 回取る（0313 の手順）。本番に入れるかは PM。 |
| Fruity Filter | **実行済み（テスト bundle、Chrome）**。V3 合格: `LoadLibraryW` → `0x7000000`、`GetProcAddress` → `0x72cb670`、unsupported 0 件（実測）。Export: 原 FLP では Filter が音に効かず判別できない（文書の記録）。変種 ins1-lp は Wine の elog16-ins1-lp と一致しない（差 RMS 5,325.57、文書の記録）。HEAD 未収録。lift は fe-lift 単体（`0309` の表）。 | 実測: `rtb-v3/engine.log:2804`（内容は `g9-diag: LoadLibraryW "…Fruity Filter_x64.dll" -> 0x7000000` と一致）、`rtb-v3/rtb-read.txt:7`。文書の記録: `gpu8/0319-cand1-functional-checks.md:10`、`0313-g9a-balance-module.md:842〜844`。`0309-g9-entry-estimate.md:49`。`progress.md:51`（push 前）。 | 【実行】`0313-g9a-balance-module.md:844` の切り分け。`progress.md:68` の ci1 の ins1-bp の Export と、Wine の追加 run の結果を読み、不一致の原因（lift の欠陥・Wine の非決定・FLP の状態）を分ける。 |
| Fruity Delay 2 | **実行済み（テスト bundle、Chrome）**。V3 合格: `LoadLibraryW` → `0x7800000`、`GetProcAddress` → `0x7abf1a0`、Problems 窓 5 件（691×295）、unsupported 0 件（実測）。Export: RT-B の原 FLP と Wine elog17 は 19 フレームが ±1 LSB（RMS 0.0013。文書の記録、再計算していない）。GUI は未確認。HEAD 未収録。lift は fe-lift 単体（`0309` の表）と、0315 の差分の木での lift 段 1 回（`0315:10`。RT-B の build の中で lift されたかは未確認。wasm に入っていること自体は `engine.log:21` で実測）。 | 実測: `rtb-v3/engine.log:21`（`real module table: name=fruity_delay_2_x64.dll base=0x7800000`）、`:2523〜2524`（`g9-diag: LoadLibraryW "…Fruity Delay 2_x64.dll" -> 0x7800000` と `g9-diag: GetProcAddress(0x7800000, "CreatePlugInstance") -> 0x7abf1a0`。両方の内容を確認）、`rtb-v3/rtb-read.txt:8〜9`。文書の記録: `0319-owner-observations-contract-reading.md:634`（Export）、`0315-delay2-step-and-next-build-candidates.md:38`（GUI は未確認）。予測値は `0315:48〜50`。`0309:50`（fe-lift）。 | 【実行】Delay 2 の GUI を開いた run で、client の描画を確認する（`0315:38` が示す stage 3 の手順、`0313:451` の形）。 |
| Fruity Parametric EQ 2 | **未確認**（wasm で未読み込み。`LoadLibraryW` の行は engine.log に無い）。Problems 窓に 1 行として残る（実測）。lift は fe-lift 単体の census のみ（CI の段は未）。依存の `Frequency Splitter\engine_x64.dll`（IPP、3.35 MB）の import は全部行あり（文書の記録）。Wine 参照は elog22 が clean（効果 RMS 8,058.8、文書の記録）。 | 実測: `rtb-v3/engine.log:2471〜2476`（vfs-miss、status 3）、画像の Problems 窓の 1 行。文書の記録: `0309-g9-entry-estimate.md:51`、`0319-owner-observations-contract-reading.md:396`、`:406`、`0320-opt1-resource-inventory.md:476`、`:488`。 | 【静的】`0313-g9a-balance-module.md:220` と `0309:40` に従い、ildsp が NULL のまま通るかを読む。【lift】`0315` の差分と同じ形（`0315:16〜22` の表、9 ファイル）で lift する。【実行】V3 で `LoadLibraryW` と `GetProcAddress` の行、Problems 窓の件数の減少を見る（`0313:9` の V0〜V3）。 |
| Frequency Shifter | **未確認**（wasm で未読み込み、`engine.log:2310〜2315` の vfs-miss）。CI の lift 段は rc 0 で通った（本体 12,966 関数、Engine 501 関数、Engine の未対応 692 = VEX/FMA。文書の記録）。build と wasm の実行は未（`0315e:3`）。Engine の path が Shared に解決され、path 表に当たらない契約の穴がある（文書の記録）。 | 文書の記録: `0315e-frequency-shifter-step.md:3`（実行の範囲）、`:10`（lift 段）、`:31`（Engine の 501 と 692）、`:36`（V3 の予測）、`:44`〜`:45`（契約の穴と直し案）。実測: `rtb-v3/engine.log:2310〜2315`。`0309:52〜53`（fe-lift）。 | 【静的】`0315e:45` の GetModuleFileNameW の直し（`0315-getmodulefilename-loaded-path.diff`）を、RT-B の結果の後の build に入れる（`0315e:50` の判断点）。【実行】その build で V3 を取り、Engine の `dispatch` の観測（`0315e:36`）を見る。Engine の VEX が SSE2 固定で選ばれるかは `0319:407` の未確認項目。 |
| Soundgoodizer | **未確認**（wasm で未読み込み。`engine.log:2462〜2467` の vfs-miss）。lift は fe-lift 単体の census のみ（CI の段は未）。ildsp と AudioRestore / iZAudioRestore の文字列はあるが、呼ばれるかは未確認（文書の記録）。Wine 参照は FLEX と合わせた elog20 のみで、分離した参照は無い（文書の記録）。elog20 の Problems 件数『2 plugin(s)』の理由は未解決（文書の記録）。 | 実測: `rtb-v3/engine.log:2462〜2467`。文書の記録: `0309-g9-entry-estimate.md:40`（ildsp）、`:54`（fe-lift）、`0319-owner-observations-contract-reading.md:398`、`:424`（Wine の modload で確認する手順）、`0320-opt1-resource-inventory.md:478`、`:488`、`:907`。 | 【静的】ildsp と AudioRestore が Wine の modload で読み込まれるかを見る（`0319:424`）。【lift】`0313-g9a-balance-module.md:220` と同じ NULL 分岐の確認の後に lift する。【実行】V3 の件数を見る。先に `0320:907` の『2 plugin(s)』の説明を付ける。 |
| FLEX | **未確認**（wasm で未読み込み。`engine.log:2202〜2207` の vfs-miss）。lift は fe-lift 単体の census で、本体は命令レベルの未対応 0（文書の記録）。engine_x64 は VEX 256 ビットの未対応 18,772 箇所、engineV2 は EVEX を含む 190,302 箇所（文書の記録）。engine の import に GL 36 行・dbghelp 6 行（文書の記録）。Wine の完全版は Problems 窓が原本の参照と画素一致（文書の記録）。 | 実測: `rtb-v3/engine.log:2202〜2207`。文書の記録: `0309-g9-entry-estimate.md:39`、`:55〜57`、`:61`、`:120`（見積もれない）、`0319-owner-observations-contract-reading.md:408`、`0320-opt1-resource-inventory.md:479`。 | 【静的】音の経路で GL・dbghelp の呼びが実行されるかを読む（`0319:408`）。【lift】VEX/EVEX の lift が前提（`0309:61`）で、`0309:120` の『見積もれない』を先に解く。未着手の前提があるため、実行には進めない。 |

## 3. 注意と限界

- 「実行済み」の 3 種は、いずれもテスト bundle（診断入り・push 前）の上の確認である。main の挙動ではない（§1 の 2）。
- Export の Wine 照合は、Balance（`0313:821`）と Filter（`0313:844`、不一致）の 2 種が実測の記録。Delay 2 は文書の記録だけで、この作業では再計算していない。
- 数値の区別: `rtb-v3` の値（LoadLibraryW・GetProcAddress・件数・窓の位置・unsupported の件数）は、読んだログの直接の値。それ以外の数値（フレーム数・RMS・lift の関数数）は文書の記録。
- 画像は件数・窓の位置・名前の一致だけを確かめた。バイト照合はしていない。
- Fruity Wrapper は 7 種に入らない（`performance-targets.md:26` の一覧に無い）。
- この文書は docs だけ。製品の差分、build、実行は 0 件。
