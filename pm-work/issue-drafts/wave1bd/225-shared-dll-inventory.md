---
draft: "225"
title: "FL が読み込もうとして見つからない共有の DLL を、全て棚卸しし、足す束の Issue の下書きを作る(Shared・ffmpeg・Python・System の DLL)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "plugin-builtin", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

FL は、機能ごとに共有の DLL(インストール先の `Shared\`・`Shared\ffmpeg\x64\`・`System\` など)を遅延で読みに行く。ビルドに無い DLL を読むと、`LoadLibraryW` が 126(見つからない)で失敗し、その先で NULL を使って engine が止まる。#1429 の `Shared\Reverb_x64.dll` が、この型の止まりだった。音が出ない・言語が出ない・保存や読み込みが効かない、なども同じ型の疑いがある。

この Issue の目標は次の 3 つ。
1. FL が読みに行く共有の DLL を、計測で全て洗い出し、表 `inputs/shared-dll-requests.tsv` にすること。
2. 各 DLL を、日常の機能に要るか・まれか・対象外かに分け、足す順を決めること。
3. 日常の機能に要る DLL を、1〜3 本ずつの束にして、足す Issue の下書きを `pm-work/issue-drafts/wave3/` に書くこと。

この Issue では、製品の DLL の行を足さない(`lift/plugins/`・`ci/extract-dlls.tsv` は変えない)。起票もしない。下書きは PM が見て決める。

## 背景

- インストーラの一覧 `inputs/fl-installer-files.tsv` で、`Plugins\` の外の DLL は 79 行・計 260,501,104 B。`Shared\` の DLL が大半で、ほかに `FLEngine_x64.dll`・`AudioRestore.dll`・`FLMManaged.dll`・`iZAudioRestore.dll`・`speaker_x64.dll`、`System\Plugin\`(VSTi・ReWire)、`System\Tools\`(LilyPond・BeatSlicer など)がある。
- 既に扱った例: `Shared\ILSvg_x64.dll`(#1395。言語の DLL。`ci/extract-dlls.tsv` に行があり、bare 名 `ILSvg_x64.dll` で読まれる)と、`Shared\Reverb_x64.dll`(#1429。`ci/extract-dlls.tsv` に行が入った。bare 名 `Reverb_x64.dll` で読まれる)。
- 読み込みの仕組み(`lift/runtime/fle_load.c` の `load_by_name`): 名前に `\` が無い読み込みは、まず `fle_realmod_find`(`lift/runtime/fle_realmod.c`。キーとの完全一致。名前は小文字化され、拡張子が無ければ `.dll` が足される)で実モジュールを探す。実モジュールが無ければ、ボリュームの検索(`search_volume`)に進み、見つからなければ `not_found`(NULL と 126)になる。ボリュームに在っても実モジュールでなければ、名前で止まる。
- 足し方: 実モジュールにするには `lift/plugins/<名前>/plugin.tsv` の行を足す(key・guest・role・sha256・size・hook・args・omit)。抽出の表 `ci/extract-dlls.tsv` の行は、その DLL を作業の木に取り出すだけで、実モジュールにはしない。plugin.tsv を足したら、同じ DLL の表の行は外す(二重の抽出を避ける。#1395 と #1429 の注釈を見本にする)。
- 注意: 既存の検査(`lift/crates/fe-build/src/plugins.rs` の `validate`・`plugin_db_names`、`app/scripts/lib/plugin-rows.mjs` の `validate`)は、guest が `Plugins\...\ファイル` の 3 要素以上を求め、本体の行を 1 つ求め、その folder 名を Plugin database の名にする。`Shared\` の DLL はこの前提に合わない。束の下書きには、この直しが要ることを書く(直しは #1429 の作業で行われる見込み。main の版で確かめる)。
- Data(`Plugins\...\Data\`)は #1478 の `data.tsv` の仕組みで扱う。この棚卸しの範囲外。
- 読み込みの失敗のログの形(`lift/runtime/fle_vfs.c` の `fle_vfs_note_missing`): `missing-path: <API> <コード> "<名前>"`。例: `missing-path: LoadLibraryW 126 "Reverb_x64.dll"`。

## 読む物

- `inputs/fl-installer-files.tsv`(`Plugins\` の外の DLL の行。`imports` の列)
- `ci/extract-dlls.tsv`(`ILSvg_x64.dll`・`Reverb_x64.dll` の行と注釈)
- `lift/runtime/fle_load.c`(`load_by_name`・`not_found`・`search_volume`・`normalize`)・`lift/runtime/fle_realmod.c`(`fle_realmod_find`)
- `lift/runtime/fle_vfs.c`(`fle_vfs_note_missing`)・`lift/runtime/fle_modules.c`(`get_module_handle`)
- `lift/crates/fe-build/src/plugins.rs`(`validate`・`plugin_db_names`)・`app/scripts/lib/plugin-rows.mjs`(`validate`)
- `pm-work/findings/missing-paths.md`(既存の、欠けた path の記録)
- `tools/crash-hunt/hunt.mjs`・`tools/e2e/run.mjs`・`tools/piano-roll-tools/measure.mjs`(計測に使える道具)
- `pm-work/issue-drafts/wave2/` の 1 つ(束の下書きの形)と `pm-work/tools/gen-wave-plugins.mjs`
- #1395(ILSvg)・#1429(Reverb)・#1394(FL の Python のスクリプト。`Shared\Python\` の扱い)の本文

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 一覧を作る: `inputs/fl-installer-files.tsv` の `Plugins\` の外の DLL(79 行)を、表の形で書き出す。一覧のファイルは書き換えない。
2. 計測(実エンジン): 起動 → Welcome → Default の再生 → demo を開く → File の各項目(Export の形式: wav・mp3・ogg・flac・mid・video。Import)→ Options の窓 → Tools の窓 → Mixer のスロットにプラグインを挿す → Channel rack に内蔵のジェネレータを挿す、の順に操作する。その間の engine のログの `missing-path: LoadLibraryW 126 "<名前>"` の行と、`GetModuleHandleW` の 126 の行(この API が同じ行を出すかは、作業の最初に確かめる)を全て拾う。`tools/e2e/` のシナリオと `tools/crash-hunt/` の道具を使ってよい。新しい道具を作る場合は `tools/shared-dll/` に置く。
3. 表を作る: `inputs/shared-dll-requests.tsv`(先頭は見出し行。列: `dll`・`guest`・`trigger`・`count`・`stoppedAfter`)。`dll` は DLL の名、`guest` は一覧の綴り、`trigger` は最初に出た操作、`count` は件数、`stoppedAfter` はその後に engine が止まったかを true / false で書く。行は計測の出力から作る(手で書かない)。
4. 分類: 各 DLL を (a) 日常の機能に要る(落ちる・機能が効かない)、(b) まれ、(c) この製品の対象外(ReWire・LilyPond・Python のツールなど。理由を書く)に分け、分類の表 `pm-work/findings/shared-dll-inventory.md` に書く。表には、各 DLL の大きさ・依存(一覧の `imports`)・lift の見込み(分かる範囲)を添える。(a) は足す順(停止を起こす物が先)を決める。
5. 束の下書き: (a) の DLL を 1〜3 本ずつの束にし、`pm-work/issue-drafts/wave3/` に 1 束 1 ファイルで書く。形は `pm-work/issue-drafts/wave2/` に倣う。各束に次を含める: `lift/plugins/<名前>/plugin.tsv` の行(role・hook・args・omit。`hook` と `omit` は、手で推測せず、lift の結果で確かめる)、key の決め方(bare 名で読まれる DLL は、その名を小文字にした物。#1395・#1429 の前例)、`ci/extract-dlls.tsv` の行の扱い(plugin.tsv を足す時に外す)、前提(上の `validate` の直し)、受け入れ(その DLL を要求する操作で `LoadLibraryW` の失敗が無くなり、engine が止まらないこと)。`blocked_by` は PM が決める。下書きは commit する。起票はしない。
6. 報告を書く(下の報告の規則)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && test -s pm-work/findings/shared-dll-inventory.md && node -e 'const L=require("fs").readFileSync("inputs/shared-dll-requests.tsv","utf8").trim().split("\n");process.exit(L.length-1>=10&&L.every((l)=>l.split("\t").length===5)&&L[0]==="dll\tguest\ttrigger\tcount\tstoppedAfter"?0:1)' && test "$(ls pm-work/issue-drafts/wave3/*.md 2>/dev/null | wc -l)" -ge 3 && bash guard/scan.sh $(git ls-files -co --exclude-standard pm-work/issue-drafts/wave3 pm-work/findings/shared-dll-inventory.md inputs/shared-dll-requests.tsv)
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle で、`standIn` が false、`elapsedMs` が 12,500 以下。
- 流れ A〜D(4 行)。各行が、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `inputs/shared-dll-requests.tsv` が在り、見出しを除いて 10 行以上、全行が 5 列。行は計測の出力から作られていること(報告に計測の出力の行を付ける)。
- 分類の表 `pm-work/findings/shared-dll-inventory.md` が在る。
- `pm-work/issue-drafts/wave3/` の下書きが 3 件以上。
- 下書きと表に、`guard/scan.sh` の指摘が無い。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 計測の操作の一覧と、各操作で出た DLL の件数、表の行数、分類の内訳(a)(b)(c)の件数、(a) の足す順の先頭 5 本、作った束の一覧(件数と題名)、流れ A〜D の 4 行、起動確認の `elapsedMs`・`preBootBytes`。
- 確認できなかった物は「確認できない」と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- この Issue では、製品の DLL の行(`lift/plugins/`・`ci/extract-dlls.tsv`)を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- R2 のバケット名・アカウント ID・ホスト名・鍵を、ファイルにも報告にも書かない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回(engine の作り直しは約 35〜40 分、bundle は約 15 分)。フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- #1429(`Shared\Reverb_x64.dll` の止まり。この棚卸しの例の 1 つ。Reverb の行は #1429 が扱う)。
- #1395(`Shared\ILSvg_x64.dll`。言語の DLL)。
- #1394(FL の Python のスクリプト。`Shared\Python\` の DLL の扱いは、そちらの範囲と重なる。この Issue では分類だけをする)。
- #1478(Data の `data.tsv`。この棚卸しの範囲外)。
