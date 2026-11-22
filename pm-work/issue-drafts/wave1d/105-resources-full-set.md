---
draft: "105"
title: "言語パック・テーマ・プロジェクトテンプレート・デモを、インストーラの全件、配信に入れる(言語・テーマ・テンプレートは起動前、デモは起動後)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示: 「言語パック、テーマ、プロジェクトテンプレートなど、FL Studio 自体が必要とする諸々が全く揃っていない」。#{{017}} は、範囲を絞って seed 表に 1 件(テンプレートの NFO)だけを足して閉じた。依頼者の指摘は解決していない。この Issue で、FL の言語の一覧・テーマの一覧・テンプレートの一覧が、インストーラと同じになる状態にする。

配信の方針(依頼者の指示): 起動の前に取る物は、FLEngine・言語・テーマ・テンプレートなど。起動の後にバックグラウンドで取る物は、プラグインとアセット。この Issue は、言語・テーマ・テンプレート(起動の前)と、デモ(起動の後)を扱う。

## 背景

- 一覧 `inputs/fl-installer-files.tsv`(guest・bytes・sha256・kind・imports。先頭に注釈 1 行と見出し 1 行。guest は FL Studio の install 先からの相対パス。kind は dll・exe・other のみで、ディレクトリの行は無い)の件数と大きさ:
  - `System\Languages\` 15 件(大きさは一覧に無い)
  - `Artwork\` 80 件(Themes 39・Wallpapers 29・Skins 7・Fonts 5。大きさは一覧に無い)
  - `Data\Templates\` 70 件(30,844,262 B = 29.4 MiB)
  - `Data\Demo projects\` 46 件(49,079,912 B = 46.8 MiB)
  - `Data\System\` 1952 件(77,820,758 B = 74.2 MiB)
  - `System\`(Languages を除く)849 件。大きさが一覧に在るのは 55 件(101,585,888 B)だけで、残り 794 件は空
  - `Data\Patches\` 17782 件(657,048,638 B。プリセット)。この Issue の範囲外
- seed 表 `lift/harness/seeds/vfs-seed-idle1.tsv`(248 行。1 行目は注釈。列は guest・host・SHA-256。ディレクトリの行は host と SHA-256 が `-`)。上の接頭辞の既存の行は、`System\Languages\` が 1(en.svg)、`Artwork\` が 5(Skins\Default 3・Wallpapers 2)、`Data\Templates\` が 2(NFO 1 件と、ディレクトリの行 1)、`Data\Demo projects\` が 2(NewStuff.flp と、ディレクトリの行 1)、`Data\System\` が 5(ファイル 4 件と、ディレクトリの行 1)。
- 作業ジョブの抽出(`.github/workflows/agent.yml` の resources の段。226〜262 行で取り出し、501〜506 行で `lift/out-app/host-extra/resources/` に置く)。取り出しの接頭辞は `System\Languages\` `Artwork\` `Data\Templates\` `Data\Demo projects\` `Data\System\` `Data\Projects\` の 6 つで、`System\` の残りは含まれない。出力は、一覧の guest を `\` で分けた相対パスのファイルと、`MANIFEST.tsv`(列は guest・bytes・sha256。guest は一覧と同じ書式)。SHA-256 と大きさは、この MANIFEST から取る(推測で書かない)。注意: 接頭辞の一覧は agent.yml にあり、キャッシュの鍵(`hashFiles`)に入らない。接頭辞を増やすなら、キャッシュの鍵が変わるように直す。
- 抽出(`app/scripts/extract-fl.mjs`): seed 表の行ごとに、guest をインストーラの木で引き、行の SHA-256 と一致したものを、`lift/out-app/host/<host 列>` に書く。一致しない行があれば 2 で止まる。キャッシュの鍵に `vfs-seed-idle1.tsv` が入る。表を変えて push すると、次の run の抽出が新しい表で `lift/out-app/host` を作る。
- host 列は `lift/out-app/host` からの相対パス。#{{017}} の行(`resources/Data/Templates/Minimal/Basic with limiter/Basic with limiter.nfo`)と同じ形にする: 新しい行の host は `resources/<一覧の相対パス>`(`/` で区切る)。この run では、`lift/out-app/host-extra/resources/` の物を `lift/out-app/host/resources/` にコピーして、ビルドと起動確認に使う(受け入れの 1 行にコピーを含める)。
- seed パック(`app/scripts/pack-seed.mjs`、`lift/harness/web/seed.args`): 表の host の物を `lift/out-app/host` の下から読み、SHA-256 を表と照合する。guest は大文字・小文字を区別せず 1 回だけ書ける(重複は止まる)。`--lazy HOST` の行はパックに入れず、manifest の `left_out` に書く。`lift/harness/web/pack.args` の `--lazy-file SRC=PATH` は、起動後の取得に入る。
- 起動時の読み(要確認): `lift/harness/vfs_seed.h` の `vs_run`(481〜498 行)は、表の全てのファイルの行を起動の時に `vs_read_host`(247 行)で読み、SHA-256 を検査する。`--lazy` の行も、この読みに入る。ブラウザでは、取得が終わるまでの読みは `Atomics.wait` で待つ(`app/src/engine/lazy-file.ts` の 1〜5 行)。つまり `--lazy` の行だけでは、起動が待たされうる。#{{015}} では Balance の取得を起動後に移しても起動は前倒しにならず、#{{101}} の背景では、engine が起動の途中で Balance を要求して待ったと書かれている。この読みが待たせていたかは、この Issue で確かめる。
- 別の遅延の仕組み: 決定 0314 の `@lazy:` の索引(`lift/harness/vfs_lazy.h`。索引の書き手は `app/src/engine/vfs-index.ts`)は、中身を読まずに名前・大きさ・時刻を VFS に置き、読みは要求の時に行う。どちらを使うかは、作業者が測って決める。
- 起動: P1(起動 10 秒以内)。今は約 6.4 秒(runner の値。この機械とは別枠)。起動前の取得の合計は 272,181,853 B(復号後。#{{015}} の判定の値)。
- 起動の記録の限界: `tools/missing-paths/measure.mjs` は、guest が要求して無かったものだけを出す。列挙(`FindFirstFile`)で見えないものは出ない。列挙の件数を出す仕組みが要る(やること 5)。

## 前回の試行で分かったこと

前回の作業者の試行(run 45059606254)は、判定の (2) で落ちた。作業は push されず、差分(`work.diff`)として保存された。次の作業は、その差分を作業ツリーに載せ、`.prior-work.md` を付けた状態から始まる(判定を通らなかった作業を、続きから行う)。以下は、その報告の数字と結論(runner の値。この機械とは別枠。nproc 4、loadavg 0.2〜2.6)。

- 判定の結果: (1)(3) は受理。(2) は差し戻し(missing が 0 件でなく、elapsedMs が 10,000 を超えた)。
- 変えた物: `lift/harness/seeds/vfs-seed-idle1.tsv`(新規 251 行。ファイル 158・ディレクトリ 42・demo 45・ディレクトリ 6)、`lift/harness/vfs_seed.h`(`@lazy-file:` の行は、起動時に読まず、大きさだけを VFS に置く)、`lift/harness/vfs_lazy.h`(遅延の上限 32 → 128)、`lift/runtime/fle_vfs.c`・`fle_vfs.h`(列挙の `enum-dir:` 行。投入中の調べは `missing-path` に数えない)、`lift/harness/web/pack.args`(demo 46 件の `--lazy-file`)、`app/scripts/pack-seed.mjs`・`extract-fl.mjs`(印を読む)、`tools/delivery-split/measure.mjs`(待ちの時刻を出す)。`seed.args` と `fe-build` は元のまま。
- 足した物(MANIFEST の値。接頭辞は本文と同じ物):
  - 起動の前: `System\Languages\` 14 件 6,285,250 B(既存の 1 件を除く)、`Artwork\` 75 件 20,261,618 B(既存の 5 件を除く)、`Data\Templates\` 69 件 30,844,125 B(既存の 1 件を除く)、ディレクトリの行 42 件。
  - 起動の後: `Data\Demo projects\` 45 件 48,766,556 B(既存の NewStuff を含めて 46 件 49,079,912 B)。
- 起動前の合計(復号後): 257,164,662 → 314,318,672 B(+57,154,010)。br: 39,387,195 → 64,307,246 B。seed パック: 9,549,689 → 66,701,852 B。
- 起動の待ち: 起動時に読む版(`--lazy`)で 165 件を入れた計測では、起動が 12,016〜12,274 ms で、初回の待ちが 2.9〜3.9 s。起動を待たせた。表の印 `@lazy-file:` の版では、demo の待ちは 0。残る待ちは Fruity Delay 2 の DLL の要求(3 回中 2 回、50・72 ms)。
- elapsedMs: **変更の前から 10 秒を超えていた**。変更前(delivery-split・3 回): 11,208.8・11,216.8・10,232.7 ms。変更後: 11,585.2・11,564.6・11,471.1 ms(delivery-split)、12,308・12,261・12,275 ms(missing-paths)。
- missing: 変更後 1,563 件(3 回とも同じ件数)。内訳: インストール先の下 1,015 件(うち 321 件は、新しく見えるようになったフォルダに対する `<フォルダ名>.nfo` の要求で、一覧に無い)、ユーザーのフォルダ 211 件、Windows のフォント 127 件、名前だけの要求 145 件など。**一覧に在る物は 0 件**(判定 (2) の式で 0)。missing を 0 件にできないのは、要求の大半がインストーラに無い物(実行時に作られる物・無い設定のファイル)だから。前回の判定の式(missing が 0 件)は、この点で誤っていた。
- 列挙: `Data\Templates\*` 7 件、`Data\Demo projects\*` 5 件で、一覧の直下の件数と一致。`System\Languages\*` と `Artwork\Themes\*` は、この起動では列挙されなかった(未確認)。
- 足さなかった: `Data\System\`(1,952 件・77.8 MiB)は、起動の記録で要求が 1 件(一覧に無い)。`System\`(Languages を除く 849 件)は、要求が 15 件。うち 4 件は `System\Config\Thumbnails\edition-*.png`(一覧に大きさが無く、SHA-256 を取れない)、残りは一覧に無い。3 回の記録だけで判断した(UI の操作での要求は未確認)。
- 残りの課題:
  - P1(10 秒以内)が未達。下の 7 と 8 を行う。
  - PM の判断: 起動前に取る物(言語・テーマ・テンプレート)は、方針どおり起動前に入れる。10 秒に収めるのは、取り込みの費用を下げることで行う。
  - 未確認: UI の操作で demo が要求されるか。`System\Languages\`・`Artwork\Themes\` の列挙。

## 読む物

- `inputs/fl-installer-files.tsv`(一覧)、`lift/harness/seeds/vfs-seed-idle1.tsv`(seed 表)
- `.github/workflows/agent.yml`(226〜262 行、501〜506 行: resources の段)、`tools/fl-installer-extract.mjs`(出力の書式)
- `app/scripts/extract-fl.mjs`、`app/scripts/pack-seed.mjs`、`lift/harness/web/seed.args`、`lift/harness/web/pack.args`
- `lift/harness/vfs_seed.h`(247〜267 行、481〜498 行)、`lift/harness/vfs_lazy.h`(決定 0314)
- `app/src/main.ts`(677 行 `startLazy`)、`app/src/engine/lazy-file.ts`
- `tools/missing-paths/measure.mjs`、`tools/delivery-split/measure.mjs`(起動 3 回の JSON: preStart・lazy・fetches・elapsedMs)
- `lift/runtime/fle_vfs.c`(ディレクトリの列挙の経路)
- #{{017}} の commit(8a39671c)の行の形

## やること

1. 変更の前に測る。`node tools/delivery-split/measure.mjs <bundle> 3` で、起動前の取得の合計(復号後)と elapsedMs を取る。br の大きさは、bundle の manifest の値で取り、どのファイルから取ったかを書く。
2. 起動の前に取る物(`System\Languages\`・`Artwork\`・`Data\Templates\` の全件 165 件)を、seed 表に足す。
   - 1 行ごとに、guest(表の既存の行と同じ接頭辞を使い、その接頭辞を本文と報告に書かない)、host(`resources/<相対パス>`)、SHA-256(MANIFEST の値)。
   - 同じ guest(大文字・小文字を区別しない)の行が既に在るものは足さない。既存の行は消さない・変えない。
   - 対象の全てのディレクトリの行も足す(`<guest>\` と `-` と `-`。例: `Artwork\Themes\DEDZII\`)。
3. 起動の後に取る物: `Data\Demo projects\*`(46 件。既存の 1 件を含む)を、#{{015}} の `--lazy` の仕組みで、起動の後に取る。`seed.args` の `--lazy` と、`pack.args` の `--lazy-file`(1 件 1 行。件数が多ければ 1 つの pack にまとめてよい)。前回の試行は、`--lazy` の行が起動を待たせたため、表の印 `@lazy-file:` で入れた(前節を見る)。
   - 起動の時の読み(`vs_run`)が、この行で起動を待たせないことを、`tools/delivery-split/measure.mjs` の `fetches`(`startMs`・`endMs`)と `elapsedMs` で確かめる。待たせるなら、起動時の読みを遅らせるか、0314 の `@lazy:` の索引にするかを選び、理由を報告に書く。#{{101}} と同じ読みの経路を使うので、衝突したら pull して直す。
4. `Data\System\`(1952 件。77.8 MiB)と `System\`(Languages を除く 849 件)は、範囲を作業者が決める。決め方: 起動の記録(列挙の件数のログと missing)と、FL の起動・UI に要る物の実測で判断する。起動に要るものは seed 表に足す(`System\` の残りは抽出の接頭辞の外なので、やること 1 の注意の通り、接頭辞とキャッシュの鍵を直す)。要らないものは足さない。どちらにしたかを、件数と大きさで報告に書く。
5. 列挙の確認: 起動後に guest が `System\Languages\` と `Artwork\Themes\` を列挙したとき、直下の全件が見えること。engine のログに、列挙したディレクトリと件数を 1 行で常に出す(環境変数・`#ifdef` は作らない。`missing-path:` の行とは別の行の形にする)。報告に、列挙の件数と、一覧から数えた直下の件数を並べる。
6. 変更の後の起動前の合計(復号後と br)と、elapsedMs の 3 回の値を報告に書く。elapsedMs が 10000 を超えたら、7 を行い、それでも超えるなら 8 を行う。
7. 起動時の取り込みの費用を下げる(デモ以外を起動後に回す前に行う)。起動時に、seed の表の全行を読んで、各ファイルの SHA-256 を検査し、VFS にコピーする所(`lift/harness/vfs_seed.h` の `vs_run`・`vs_read_host`)を、次のように変える。
   - (1) ファイルの本体は、初回の読みまで遅らせる。起動の時には、ディレクトリ・ファイルの名前と大きさだけを作る。本体は、seed パックの領域を参照して、guest が読んだ時に写す。
   - (2) SHA-256 の検査は、パックの作成時(ビルド)と、初回の読みの時に行う。起動の時に全件を検査しない。
   - (3) 列挙(`FindFirstFile`)は、名前の表だけで答える。
   - 検査を弱めない。読んだ物は、必ず検査する。製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
   - 変えた後で、起動前の合計と elapsedMs の 3 回の値を、報告に書く。
8. それでも 10000 を超えるときだけ、起動後に回す。優先順位(P3 > P2 > P1 > サイズ)に従い、次の順に回す: デモ(既に起動後)、`Artwork\Wallpapers\`、`Artwork\Fonts\`・`Artwork\Skins\`(#{{016}} の `missing-paths` で、起動の最中に要求されない物だけ)。テーマ・言語・テンプレートは、起動前のまま。回した物と、回した理由を報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/missing-paths/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/missing.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const inv=new Set(fs.readFileSync("inputs/fl-installer-files.tsv","utf8").replace(/\r/g,"").split("\n").filter((l)=>l&&!l.startsWith("#")).slice(1).map((l)=>l.split("\t")[0].toLowerCase()));const dec=(p)=>p.replace(/\\u([0-9A-F]{4})/g,(_,h)=>String.fromCharCode(parseInt(h,16)));const left=L.flatMap((r)=>r.missing).map(dec).filter((p)=>{const m=/^[^\\]+\\[^\\]+\\[^\\]+\\FL Studio [0-9]+\\/i.exec(p);if(!m)return false;const rel=p.slice(m[0].length);return /^(System\\Languages\\|Artwork\\|Data\\Templates\\|Data\\Demo projects\\)/i.test(rel)&&inv.has(rel.toLowerCase())});process.exit(L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.elapsedMs<=10000)&&left.length===0?0:1)' "$RUNNER_TEMP/missing.jsonl" && node -e 'const fs=require("fs");const rows=(f)=>fs.readFileSync(f,"utf8").replace(/\r/g,"").split("\n").filter((l)=>l&&!l.startsWith("#")).map((l)=>l.split("\t"));const inv=rows("inputs/fl-installer-files.tsv").slice(1).filter((r)=>/^(System\\Languages\\|Artwork\\|Data\\Templates\\)/i.test(r[0]));const seed=rows("lift/harness/seeds/vfs-seed-idle1.tsv");const man=new Map(rows("lift/out-app/host-extra/resources/MANIFEST.tsv").slice(1).map((r)=>[r[0].toLowerCase(),r[2].toLowerCase()]));const root=seed.find((r)=>r[0].endsWith("\\Data\\Templates\\")&&r[1]==="-")[0].slice(0,-"\\Data\\Templates\\".length).toLowerCase();const byGuest=new Map(seed.filter((r)=>r[1]!=="-").map((r)=>[r[0].toLowerCase(),r]));let miss=0,bad=0;for(const r of inv){const k=r[0].toLowerCase();const s=byGuest.get(root+"\\"+k);const sha=man.get(k);if(!s||!sha)miss++;else if(s[2].toLowerCase()!==sha)bad++;}process.exit(inv.length===165&&miss===0&&bad===0?0:1)'
```

判定の基準: (1) 起動 1 回(boot-bundle)で、engine-idle・standIn が false・error が null。(2) 3 回の起動(missing-paths)で、engine-idle・standIn が false・error が null・elapsedMs が 10000 以下(3 回の全て)。そのうえで、3 回の `missing` の各行(パス)を次の式で数え、0 件であること。(i) 行は絶対パスで、非 ASCII は `\uXXXX` と書かれているので、戻してから比べる。インストール先のフォルダより下の物だけを取り、相対パスにする。(ii) その相対パスが、接頭辞 `System\Languages\`・`Artwork\`・`Data\Templates\`・`Data\Demo projects\` のどれかで始まる(大文字小文字を区別しない)。(iii) `inputs/fl-installer-files.tsv` に、同じパス(大文字小文字を区別しない)の行が在る。範囲の外・一覧に無い物(実行時に作られる物・無い設定のファイル・既定のフォルダ)は数えない。(3) 一覧の `System\Languages\`・`Artwork\`・`Data\Templates\` の 165 件(15 + 80 + 70)の全てが、seed 表に同じ guest パスで在り、SHA-256 が MANIFEST と一致する(欠け 0・不一致 0)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く:
  - 足した件数と大きさを、接頭辞ごとに、起動の前に取る物と後に取る物に分けて。Languages と Artwork の大きさは MANIFEST の値。
  - `Data\System\` と `System\` の判断: 足したもの、足さなかったもの、その理由(件数・大きさ)。
  - 起動の前の取得の合計(復号後と br。変更の前後)。
  - 起動の後の取得(デモ)の件数と大きさ。`--lazy` の行の読みが起動を待たせたかどうか(`fetches` と `elapsedMs` で)。
  - 列挙の件数(`System\Languages\`・`Artwork\Themes\`)と、一覧から数えた直下の件数。
  - elapsedMs の 3 回の値。
  - 変えたパス(app/・lift/・tools/・.github/ を含む)。`app/`・`lift/`・`ci/` の push は main の deploy を動かすので、その旨を書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。SHA-256 と大きさは、MANIFEST(インストーラの実物から取った値)と照合したものだけを置く。
- SHA-256 を推測で書かない。`-` を SHA-256 の欄に書かない。
- `inputs/fl-installer-files.tsv` は読むだけ。
- ベンダー名と Cloudflare の識別子を、新しく書く本文・報告・コードに書かない。インストール先の接頭辞を、本文と報告に書かない。表の guest 列の接頭辞は、既存の行と同じ物を使う(この接頭辞は既存の状態で、扱いは PM が決める。報告に一言添える)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限を、この作業の判断で増やさない。

## 他のタスクとの関係

- #{{017}}(言語・テーマ・テンプレートの範囲を絞った版。1 件を足して閉じた)の続き。この Issue が、残りを全件入れる。
- #{{012}}(親。配布の一覧)。
- #{{014}}(起動の前と後の区分)と #{{015}}・#{{101}}(像と seed の読みを要求の時まで遅らせる)が、同じ読みの経路(`lift/harness/vfs_seed.h`・`app/src/main.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{035}}(起動を 10 秒以内にする)と、起動の前の量は同じ予算を使う。
