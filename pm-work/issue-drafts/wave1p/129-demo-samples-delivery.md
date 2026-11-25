---
draft: "129"
title: "demo 2 件(DEDZII・Mehran Abbasi)の素材を、起動の後の取得で配信し、P3 の入力に音が出る demo があるかを確かめる"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["121", "122", "128"]
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

P3 の測定(#1335)の入力に、音が出る demo が無い。前回の測定(51 件の候補のうち開けた 30 件)は、すべて `nonSilentQuanta` が 0 だった。原因の一部は、demo の素材が配信されていないこと。

この Issue は段階 1(素材の収集)。配信は段階 2(#1403。`blocked_by` にこの Issue)で行う。この Issue の目標(PM の決定。前回の目標の「`nonSilentQuanta` が 1 以上」は、この Issue から外した。音が出る demo の選定は #1401 が行う):
1. DEDZII - NIGHT TERROR と Mehran Abbasi - With You を File > Open で開き、ゲストが要求して見つからなかった素材(`missing-path` の行)を、インストーラの一覧(`inputs/fl-installer-files.tsv`)と照合する。一覧に在る素材(`inventory`。DLL を除く)の完全なパスを特定し、`pm-work/findings/demo-assets-inventory.tsv` に書く。その完全なパスを `ci/resources-prefixes.txt` に、ファイルの行として足す。
2. DLL(`dll`。`.dll` の物。一覧に在るか否かを問わない)と、一覧に無い物(`notInInstaller`)の件数を分けて出す。`notInInstaller` の先頭 10 件も出す。
3. DEDZII が開く(`error` が null で `engine-idle`)。Mehran の `error` は報告に書き、判定には入れない。

## 背景

- インストーラの素材は、`Data\Patches\Misc\Used by demo projects\<曲名>\` に、曲ごとのフォルダで置かれている(`inputs/fl-installer-files.tsv`。623 件・約 82 MB。曲のフォルダは 26 件)。
- 2 フォルダを選んだ理由: 一覧で小さい(DEDZII 4 件・877,543 B。Mehran Abbasi - With You 5 件・694,532 B。合計約 1.57 MB)。DEDZII は、#1365 の段階 1 で、File > Open で開けて再生が進むことが確かめられた。
- 取り出しの行(`ci/resources-prefixes.txt`): `\` で終わる行は接頭辞(その下の全ファイル)、`\` で終わらない行は完全なパス(インストール先からの相対。一覧の guest と同じ綴り。大文字・小文字を区別しない)。道具(`tools/fl-installer-extract.mjs`)が行の形で分ける(PM の commit 8391dcaa)。完全なパスの行の大きさの合計は 30 MB 以下(`spec/workflow.test.sh` の (r5) が検査する)。
- 取り出しは main の一覧で行うので、この Issue の run の中で足した行は、この run の作業ツリーには届かない。次の run から届く。
- 欠けた素材の記録: エンジンのログの `missing-path: <api> <err> "<path>"` の行。パスは、インストール先の絶対パスか、工場データの変数(`%…%\`)の下にある。一覧との照合は、その root を外した相対パスで、大文字小文字を無視して行う。root の値は道具に書かない(ログから求める)。
- 配信の仕組みは #1366 と同じ(起動の後の取得。seed 表の `@lazy-file:` の行と `pack.args` の組)。段階 2 で行う。
- 前回の run の実測(段階 1 の前): 一覧に在る素材は、DEDZII で 27 件(接頭辞の外の `Packs\Drums (ModeAudio)\`・`Packs\Drums\`・`Packs\Risers\`・`Packs\SFX\`・`Impulses\` の下。合計 4,830,812 B)。Dance の 12 件と Misc の 9 件は、前回の差分で配信済み(下の「前回までの試行で分かったこと」)。

## 前回までの試行で分かったこと

- 1 回目: run 45392098052。`fe-build bundle` の seed の段で止まった(9 件が作業ツリーに無かった。接頭辞の一覧に 2 行が無かったため。main の c43f05ee で足した)。
- 2 回目: run 45436255034(16:23〜19:02 UTC)。判定は「差し戻し」(受け入れの後段が Mehran の測定で exit 1)。push していない。commit は `bfb28a16`(作業者の環境)。runner の値。nproc 4、loadavg 1.6〜1.9。
  - 9 件は配信された(seed 表の 9 行が `@lazy-file:`)。DEDZII の `projectMissing` に 9 件は一件も出ない。
  - 前回の差分には、Mehran の `.ogg` の修正が在る(`app/scripts/pack-assets.mjs`・`app/test/unit/pack-assets-fruity.test.ts`)。manifest の URL に生の `#` が入り、`#` より後ろが fragment として切られて読めなかった。`#` `%` `?` を `%XX` にする。この修正は取り込んで残す。
  - DEDZII: 開いた(`error` null・`engine-idle`・`quantaWhilePlaying` 5765・`underrunQuanta` 366)。`nonSilentQuanta` は 0 で、無音。
  - Mehran: 開けない。`.ogg` を読んだ後、遅延読み込みの `fruity_balance` の後で `ExitProcess`(exit 0)。#1372 の範囲(#1372 は閉じた)。
  - 2 回目の「一覧に無い 183 件」の分類は誤り(工場データの変数の root を外していなかった。下の 3 回目で訂正)。
- 3 回目: run 45474905246(判定は差し戻し。push していない。commit は作業者の環境の `6133a835`。runner の値。nproc 4、loadavg 1.4〜1.9)。
  - 取り込みの衝突は `lift/harness/web/pack.args` ではなく `bundle.txt` だけで、main の版(#1369 の `@lines` の形)を採った。
  - 結果: DEDZII は開いた(`error` null・`engine-idle`。「Problems loading the project」の箱が 1 つ)。`nonSilentQuanta` は 0(参考)。`missingDistinct` 3375。Mehran は開いた(`error` null)。`missingDistinct` 117。
  - 旧い分け(DLL を含む)では、一覧に在る物は DEDZII 64 件(素材 27 件・DLL 37 件)、Mehran 31 件(DLL だけ。素材は 0 件)。DLL の内訳は、プラグイン 28 件(`Plugins\Fruity\…`)とエンジン・共有のモジュール 9 件。
  - 一覧に無い物(DEDZII 3311 件。推定。未確認): 文書の中の物(`C:\Users\…` の下。1682 件)、`Data\…` の候補の場所への問い合わせ(1572 件。同じ名前の素材が一覧の別のフォルダに在る物が多い。エンジンが候補を順に探した跡と見られる)、DLL 40 件、その他 17 件。
  - 段階 1 に分けた理由: 27 件の完全なパスは、この run の中では作業ツリーに届かない。だから、この run で seed 行と `pack.args` の組を足すと、実物が無いので bundle が落ちる。配信は、行が main に入った後の run でしか検証できない。
  - 接頭辞の folder 単位では 30 MB を超える(素材が入る folder 12 件の合計は約 87 MB。`Risers\` だけで約 17 MB)。完全なパスの行にした(PM の commit 8391dcaa)。
  - 27 件の完全なパスは、前回の report・judge の出力・artifact のどれにも残っていない。段階 1 の run で、道具の出力から取り直す。
  - 前回の差分のうち残すもの: seed 表の `@lazy-file:` の 21 行(Dance の 12 件・Misc の 9 件。大きさと SHA-256 は一覧の値と一致)、`lift/harness/web/pack.args` の 21 組、`app/scripts/pack-assets.mjs` の `%XX` 化と試験(14 件通る)、`tools/p3-audio-probe/measure.mjs` の内訳の道具。
  - 注意: 前回の boot の `elapsedMs` は 12,524 で、判定の 12,500 を超えた(負荷の揺れ)。判定は下の基準のまま。超えたら、報告に書く。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(開いて `nonSilentQuanta`・`projectMissing` を出す道具。ここに内訳を足す。前回の差分の版が在る)
- `tools/missing-paths/measure.mjs`(`missing-path` の行の読み方)
- `inputs/fl-installer-files.tsv`(guest 列・bytes 列・sha256 列。読むだけ)
- `ci/resources-prefixes.txt`(接頭辞と完全なパスの行)、`tools/fl-installer-extract.mjs`(行の形で分ける)、`spec/workflow.test.sh` の (r5)
- `lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/pack.args`(前回の差分の 21 件が在る。段階 1 では新しい行を足さない)
- `pm-work/workflow-design.md`(「抽出物の接頭辞の一覧は」の段落)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
1. 前回の差分(保存された作業)が取り込まれているかを確かめる: seed 表の `@lazy-file:` の 21 行・`pack.args` の 21 組・`app/scripts/pack-assets.mjs` と その試験・`tools/p3-audio-probe/measure.mjs` の内訳。`grep -c` で二重になっていないことを確かめる。衝突の印が在れば解消する。21 行と 21 組は残す(実物は取り出し済みの接頭辞の下に在り、bundle が通ることを前回の run で確かめた)。
2. 道具 `tools/p3-audio-probe/measure.mjs` の内訳を、次の定めで出す(既存の項目は残す):
   - `missingDistinct`(数): エンジンのログの `missing-path` の行の全て(`.wav` に限らない)を、重複を除いた件数。
   - root を外す: インストール先の絶対パス(ログに最も多く出る、先頭の部分)と、工場データの変数(`%…%\`)の 2 種類だけ。道具に値を固定で書かない(ログから求める)。`C:\Users\…` の物は、root を外しても一覧に在るとは見ない。
   - 判定の順: (a) 末尾が `.dll`(大文字・小文字を無視)の物は `dll`(数)。(b) それ以外で、root を外した綴りが一覧の guest に在る物は `inventory`(数)。全件を `inventoryList`(配列。各要素は guest の綴り・`bytes`・`sha256`。一覧の値)に出す。(c) それ以外は `notInInstaller`(数)。その先頭 10 件の綴りを `notInInstallerFirst10`(配列)に出す。
3. `inventoryList` の全件を、`pm-work/findings/demo-assets-inventory.tsv` に書く。1 行目は見出し `demo<TAB>path<TAB>bytes<TAB>sha256`。`demo` は、その素材を要求した demo(`DEDZII`・`Mehran`、両方なら `both`)。`path` は一覧の guest の綴り(インストール先からの相対。接頭辞も root も付けない)。`bytes`・`sha256` は一覧の値。1 素材 1 行。
4. `ci/resources-prefixes.txt` に、`path` を 1 行ずつ足す(ファイルの行。`\` で終わらない)。既に同じ行が在るか、接頭辞の下に在る物は足さない。既存の行は保つ。完全なパスの行の大きさの合計は 30 MB 以下(`spec/workflow.test.sh` の (r5) が検査する)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない(ジョブの最後の判定が通ったときだけ push される)。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-129 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-129/dedzii.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-129/mehran.flp && for f in dedzii mehran; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-129/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["dedzii","mehran"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));const num=(r)=>["inventory","dll","notInInstaller"].every((k)=>typeof r[k]==="number")&&Array.isArray(r.notInInstallerFirst10)&&r.notInInstallerFirst10.length<=10&&Array.isArray(r.inventoryList);process.exit(L[0].error===null&&L[0].state==="engine-idle"&&L.every(num)?0:1)' && node -e 'const fs=require("fs");const t=fs.readFileSync("pm-work/findings/demo-assets-inventory.tsv","utf8").trim().split("\n");const h=t.shift();const P=fs.readFileSync("ci/resources-prefixes.txt","utf8").split("\n").filter((l)=>l!=="");const low=(s)=>s.toLowerCase();const ok=(p)=>P.some((l)=>low(l)===low(p)||(l.endsWith("\\")&&low(p).startsWith(low(l))));process.exit(h==="demo\tpath\tbytes\tsha256"&&t.length>0&&t.every((l)=>{const c=l.split("\t");return c.length===4&&ok(c[1])})?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && bash spec/workflow.test.sh
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null。
- DEDZII と Mehran の `measure.mjs` の行で、`inventory`・`dll`・`notInInstaller` が数値、`notInInstallerFirst10` が 10 件以下の配列、`inventoryList` が配列。DEDZII の `error` が null で、`state` が engine-idle。Mehran の `error` は報告に書き、判定には入れない(#1372 は閉じた)。
- `demo-assets-inventory.tsv` が在り、見出しが上の形で、全ての行の `path` が `ci/resources-prefixes.txt` の行(同じ綴りの行、または `\` で終わる接頭辞の下)に在る。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)と `bash spec/workflow.test.sh` が通る(接頭辞の (r5): 完全なパスの大きさの合計が 30 MB 以下)。
- 参考(判定に入れない): 前回の run の実測で、DEDZII の `inventory` は 27 件・合計 4,830,812 B。今回の数と違えば、報告に理由を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: DEDZII と Mehran の `missingDistinct`・`inventory`・`dll`・`notInInstaller`・`notInInstallerFirst10`(先頭 10 件の綴り)、`inventoryList` の件数と合計の大きさ、追加した完全なパスの行の数と大きさ、変えた接頭辞(無ければ「無し」)、DEDZII の `nonSilentQuanta`(参考。判定には入れない)、前回の差分の取り込みの衝突の有無、`app/` を変えたこと(push すると `deploy.yml` が動く)、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 空の代替(空ファイル・0 埋め)を置かない。大きさと SHA-256 は、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- `inputs/fl-installer-files.tsv` は読むだけ。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名・Cloudflare の識別子・インストール先の既定の値(絶対パス・工場データの変数の綴り)を、本文とファイルに書かない。道具は、ログの値から求める。
- Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 段階 1 では、seed 表・`pack.args`・`bundle.txt` の新しい組を足さない(前回の差分の 21 件は残す)。配信は段階 2。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 32〜46 分(前回は 33 分)。bundle は約 13 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 範囲外(この Issue では直さない)

- 段階 2(配信: seed 行・`pack.args`・lazy の件数の上限)。#1403。
- インストーラに無い素材(FL がオンラインで取る素材)。件数と先頭 10 件を出すまで。配信はしない。
- 他の曲の素材(`Used by demo projects` の残りの 24 曲)。音が出る demo の選定は #1401。
- Mehran の `fruity_balance` の遅延読み込み(#1372。閉じた)。
- DLL(`dll`)の配信。プラグインと、エンジン・共有のモジュールは、別の Issue の範囲。
- FLEX・Emphasizer の読み込み(#1322・#1323 の範囲)。

## 実物の置き場

- 完全なパスの行を足した後、その実物は次の run の取り出し(main の `ci/resources-prefixes.txt`)で `lift/out-app/host-extra/resources/` に届く。この Issue の run では届かないので、この Issue の受け入れは実物を要求しない。
- 前回の 1 回目の止まりは、`host/resources` の下の `Misc` の 9 件が取り出されていなかったための ENOENT だった(接頭辞の一覧の 2 行が無かった。上の「前回までの試行で分かったこと」)。

## 他のタスクとの関係

- blocked_by: #1365・#1366・#1372(いずれも閉じている)。
- 後続: 段階 2(配信。#1403。`blocked_by` にこの Issue)。#1401(音が出る demo を決める)は段階 2 の後。#1335(P3 の測定)は #1401 を待つ。
- #1372(Exoplanet)・#1387(Riff machine)・#1400(Sytrus)も、`pack.args`・seed 表を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
