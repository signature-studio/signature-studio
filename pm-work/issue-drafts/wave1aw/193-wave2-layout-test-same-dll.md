---
draft: "193"
title: "第 2 波の束の `plugin.tsv` が入っても、`plugins.rs` の試験『160 本が像の領域に入る』が、同じ DLL の重複で落ちない形にする"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["plugin-builtin", "verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

`lift/crates/fe-build/src/plugins.rs` の試験 `the_second_wave_is_160_dlls_and_fits_the_image_area` は、第 2 波の DLL の像(SizeOfImage)が、GLOBAL_BASE より下の領域に、重ならずに入ることを確かめる。

今の試験は、`tools/plugin-layout/wave2-plugins.mjs` が作る表の 159 本を、`lift/plugins/` の行(第 1 波の Balance と Sytrus)に足して読む。第 2 波の束の `plugin.tsv` が main に入ると、同じ DLL が 2 回数えられ、試験が落ちる。この Issue は、試験を、束の `plugin.tsv` が `lift/plugins/` に在っても無くても通る形にする。製品の動作は変えない。

この Issue の目標は次の 3 つ。
1. 束の `plugin.tsv` が `lift/plugins/` に在る状態を、試験の中で作り(tempdir)、その状態で試験が通ること。その前に、同じ状態で試験が落ちることを確かめること(赤から緑へ)。
2. 同じ DLL を 2 回数えないこと。
3. `lift/plugins/` の行が、表の外の DLL(表が古い時)であれば、試験が落ちること。

## 背景

- 試験の中身: (1) `tools/plugin-layout/wave2-plugins.mjs` で表を tempdir に作り、`read_rows_from` で読む(159 本・本体 131)。(2) `lift/plugins/` の行を足す。(3) `validate`。(4) `inputs/plugin-image-sizes.tsv` を sha256 で引き、`assign_bases` で配置し、重なりと GLOBAL_BASE の下を検査する。
- 確かめた事実(作業者が再現して確かめる): 束の `plugin.tsv` の key は `pm-work/tools/gen-wave-plugins.mjs` の作り方で `<種別>.<dir>.<DLL の名>`(例: `effects.control-surface.control_surface_x64.dll`)。試験の表の key は `tools/plugin-layout/wave2-plugins.mjs` の作り方で `<種別>.<種別>-<名前>.<DLL の名>`(例: `effects.effects-control-surface.control_surface_x64.dll`)。2 つは一致しない。だから、落ちるのは key の重複ではなく、guest の重複(`validate` の『the guest path is not unique』)のはず。作業者は、落ちた行を出して、どちらかを確かめる。
- guest(`Plugins\Fruity\…\file.dll`)は DLL ごとに一意で、表と束の `plugin.tsv` で同じ綴りになる。同じ DLL を、guest(小文字)で 1 本と数える。
- 第 1 波の 4 フォルダ(Fruity Balance・Fruity Filter・Fruity Delay 2・Sytrus)は、表から外してある(`wave2-plugins.mjs` の `FIRST_WAVE`)。だから、`lift/plugins/` の行のうち表に無いものは、この 4 フォルダの DLL だけであるはず。
- 関連: #{{135}}(この試験と表を作った Issue)、`pm-work/findings/guest-layout.md`。

## 読む物

- `lift/crates/fe-build/src/plugins.rs`(`read_rows_from`・`read_plugins_from`・`validate`・`assign_bases`・`size_of_image` 周り・試験 `the_second_wave_is_160_dlls_and_fits_the_image_area`)
- `tools/plugin-layout/wave2-plugins.mjs`(表の作り方。出力は `<out dir>/<folder>/plugin.tsv`)・`inputs/plugin-image-sizes.tsv`(見出し 1 行。sha256・大きさ・SizeOfImage・guest)
- `pm-work/tools/gen-wave-plugins.mjs`(束の key と dir の作り方。読むだけ)
- `lift/plugins/fruity-balance/plugin.tsv`・`lift/plugins/sytrus/plugin.tsv`(第 1 波の行)
- `pm-work/findings/guest-layout.md`(#{{135}} の記録)

## やること

0. 作業の最初に `date -u` を記録する。
1. 試験を先に赤くする。試験の中で、束の `plugin.tsv` が `lift/plugins/` に在る状態を作る: tempdir(既存の試験と同じく `CARGO_MANIFEST_DIR` の下の `.tmp-*`。試験の後に消す)に `lift/plugins/` の内容をコピーし、表の 159 本を、dir ごとの `plugin.tsv` として書く。dir は表の folder 名から種別の接頭辞を外したもの。key は `<種別>.<dir>.<DLL の名>`(gen の形)。その tempdir を試験の入力にする。この状態で、試験が落ちることを確かめ、落ちた行(理由)を報告に書く。
2. 直す: 試験の入力の DLL の集まりを、guest(小文字)で一意にする。`lift/plugins/` の行を優先し、表の行で同じ guest の物は捨てる。その集まりに `validate` をかけ、sha256 で `inputs/plugin-image-sizes.tsv` を引き、元の試験と同じ式で配置と重なりを検査する。
3. 追加の検査: `lift/plugins/` の行の guest が、表の 159 本にも、第 1 波の 4 フォルダ(Fruity Balance・Fruity Filter・Fruity Delay 2・Sytrus)にも無い時は、試験が落ちる(表が古い)。その検査を、試験の中で、表の行と、この 4 フォルダの一覧で行う。
4. 元の試験の検査は残す(件数 159・本体 131・サイズの合計・先頭の base・重なり・GLOBAL_BASE 以下)。件数の注釈(表の 159 本と、第 1 波の DLL を足した数)は、元の試験の期待値と違う時だけ、報告に書く。期待値は、事実に合わせて直し、その根拠を書く。
5. 赤から緑へ: 直す前の出力の行(落ちた理由)と、直した後の出力の行を報告に付ける。

## 受け入れ

```accept
cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/crates/fe-build/src/plugins.rs')"
```

判定の基準:
- `cargo test -p fe-build` が通る。この試験は、束の `plugin.tsv` が `lift/plugins/` に在る状態(tempdir)で通る。
- 差分は `lift/crates/fe-build/src/plugins.rs` だけ。製品の関数(`validate`・`read_rows_from`・`assign_bases` など)の動作は変えない。変えるのは試験の中と、試験の中だけで使う補助関数。
- 報告に、直す前の赤の出力の行と、直した後の緑の出力の行を付ける。

## 報告の規則

- 1 行目は「試験: <何を実行し、どの出力を観測したか>」で始める。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 赤の出力の行(落ちた理由と、重複した guest または key)、表の key と束の key の形が一致するかの確かめ、直した方式、件数の注釈(期待値を直したなら、その根拠)。

## 制約

- 製品の動作と内容を変えない。`lift/plugins/`・`ci/`・`inputs/`・`lift/harness/`・`tools/`・`pm-work/` を変えない(読むだけ)。変えるのは `lift/crates/fe-build/src/plugins.rs` の試験の中だけ。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。Python と OCR を使わない。
- AI のベンダー名と、Cloudflare の識別子を、ファイルに書かない(`guard/scan.sh` が検査する)。URL を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、試験の後に消す。

## 作業の進め方

- ビルドは要らない(試験だけ)。`cargo test` の初回のコンパイルは数分。シェルツールの `timeout` 引数が要るなら 5400000 まで。
- 1 時間以内に終える。受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- #{{135}}(この試験と表を作った Issue)。
- #{{126}}・#{{171}}・#{{192}}(Data の宣言)は、`lift/plugins/` の行を増やすか、`plugins.rs` の生成を変える。この試験の直しは、その行が main に入る前に入ること。
- 第 2 波の段 1 の束(下書き #{{041}}〜#{{055}})は、この Issue が閉じた後に起動する(PM が blocked_by に足す)。
- 同じファイル `lift/crates/fe-build/src/plugins.rs` を #{{192}} が変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、試験の部分の衝突を直し、受け入れをもう一度通す。
