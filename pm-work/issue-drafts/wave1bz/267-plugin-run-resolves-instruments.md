---
draft: "267"
title: "tools/plugin-run が、インストゥルメント(generators.*)を名前で解決できる"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: []
labels: ["plugin-builtin"]
size: "S"
priority: "High"
project: "signature-studio/3"
---

## 目的

`tools/plugin-run/run.mjs` は、効果(key が `effects.` で始まる本体)だけを名前で解決する。インストゥルメント(key が `generators.` で始まる本体)の名前は、`no effect plug-in named "…"` で止まる。第 2 波のインストゥルメントの束(#1463〜#1477 の 15 件)は、受け入れに `plugin-run` を使うので、この道具がインストゥルメントを解決できないと、受け入れが通らない。

この Issue は道具(`tools/plugin-run`)の直しだけを行う。製品(`lift/`・`app/`)は変えない。FLEX の plugin.tsv(#1544)とは独立。

## 背景

- `effectRows()` の判定は、`role` が `本体` で、key が `effects.` で始まる行だけを読む。`resolveNames` は、この行から名前を引く。
- 作業者が FLEX(#1544)の作業で、この判定を次の形にした差分が在ったが、main には入っていない。この Issue は、その要点を入れ直す。
  ```js
  if (r.role !== '本体' || !/^(effects|generators)\./.test(r.key ?? '')) continue;
  ```
- main に在るインストゥルメントの例: `lift/plugins/sytrus/plugin.tsv` の Sytrus(key `generators.sytrus.sytrus_x64.dll`、本体)。
- 補助だけのフォルダ(本体の行が無い): `lift/plugins/plugin-manager`(Plugin Manager。role は補助)。この直しの後も、名前では解決できないままにする(正しい)。
- 効果の候補の順: `candidatesFor(name, roots, max, pluginNames)` は、既定の `pluginNames` に `effectRows()` の名前を使い、候補の .flp が名前を含む「他のプラグインの数」を数える(#1537)。`effectRows()` を広げると、インストゥルメントの名前もこの数に入り、効果の候補の順が変わる。効果の測定を変えないため、既定の `pluginNames` は効果の行(key が `effects.` で始まる)の名前だけから作る。

## 変えること

1. `tools/plugin-run/run.mjs`
   - `effectRows()` の判定を上の 1 行にする。関数の名前は変えない(spec が使う)。関数の注釈と、ファイル先頭の `//` の説明(本体の key の説明)を、インストゥルメントも含む内容に直す。
   - `resolveNames` のエラー文の括弧の中を `(a folder of lift/plugins with an effects.* or generators.* main DLL)` にする。語 `no effect plug-in named` は変えない(試験が使う)。
   - `candidatesFor` の既定の `pluginNames` を、効果の行(key が `effects.` で始まる)の名前だけから作る。
   - 場面・判定・出力の欄は変えない。
2. `spec/plugin-run.test.mjs`(先に直して red を確かめる)
   - 「2) インストゥルメント(Sytrus)で終了コード 1」を、「本体の行が無い名前(Plugin Manager)で終了コード 1、標準出力は空、理由に `no effect plug-in named "Plugin Manager"`」に替える。Sytrus は使わない(解決が通ると bundle を読みに行くため)。
   - 「5) 効果の行」の「効果の行に、インストゥルメントは無い」を、「効果とインストゥルメントの行だけ(key は `effects.*` か `generators.*`)。Sytrus(key `generators.sytrus.sytrus_x64.dll`)が在る」に替える。
   - 「6) 名前の解決」に 1 件足す: `resolveNames(['Sytrus'])` が 1 行(folder `sytrus`、key `generators.sytrus.sytrus_x64.dll`)になる。
   - 1 件足す: `candidatesFor` の既定の順は、インストゥルメントの名前で変わらない。試験の作り方は作業者が決める。例: 同じ根に 2 本の .flp を作る。小さい方は `Fruity Delay` と `Sytrus` の名前を含み、大きい方は `Fruity Delay` だけを含む。効果だけを数えると小さい方が先になり、インストゥルメントも数えると大きい方が先になる。試験は前者を期待する。

## やること

1. 試験を先に直して、`node --test spec/plugin-run.test.mjs` が red になることを確かめる(失敗の行を報告に貼る)。
2. `tools/plugin-run/run.mjs` を直して、試験を green にする。
3. 受け入れを通す(下)。`fe-build engine` は 1 回だけ回す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node --test spec/plugin-run.test.mjs && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=314368711&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&L.map((r)=>r.flow).join("")==="ABCD"&&L.every(ok)?0:1)' "$RUNNER_TEMP/flow.jsonl" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Sytrus" > "$RUNNER_TEMP/pr.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L[0].plugin==="Sytrus"&&L[0].error===null&&L[0].instantiated===true?0:1)' "$RUNNER_TEMP/pr.jsonl" && npm test --prefix app && test -z "$(git diff --name-only origin/main...HEAD -- . ':!tools/plugin-run' ':!spec/plugin-run.test.mjs')"
```

判定の基準:
- `node --test spec/plugin-run.test.mjs` が通る。
- 起動確認(`boot.json`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 20,000 以下。
- 流れ A〜D(4 行): `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、B と D は `welcomeClosed` が true。
- `plugin-run` の Sytrus(1 行): `plugin` が `Sytrus`、`error` が null、`instantiated` が true。`processed` は問わない。
- `npm test --prefix app` が通る。
- 差分: `tools/plugin-run/run.mjs` と `spec/plugin-run.test.mjs` の外に変更が無い。

`instantiated` が false のときは、道具の判定を変えない。Sytrus を要求する .flp が scene (c) の候補に無いのが原因だと考えられる(推定。場面 (d) はインストゥルメントの要求を作れない)。その場合は、stderr の `scene (c) for Sytrus:` の行(候補の一覧)と `plugin-run` の 1 行を報告に貼って止まる。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「受理」「差し戻し」「未確認」のいずれかで書く。出力の行を付ける。確認できなかった物は「確認できない」と書く。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python・OCR を使わない。
- AI のベンダー名と Cloudflare の識別子を、ファイル・コミット・報告に書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義を変えない。commit に AI の署名の行を付けない。必要の無いブランチを作らない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルド(`fe-build engine`)は 1 回にする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- `git add` は、変えたファイルを 1 つずつ指定する。作業の最後に、受け入れが通らなくても commit する。

## 他のタスクとの関係

- #1463〜#1477(第 2 波のインストゥルメント 15 件)は、この Issue を blocked_by に持つ(起票の後に設定する)。この Issue が閉じるまで、その 15 件は起動しない。
- #1544(FLEX)の作業は、この直しを main に持っていない。この Issue で main に入る。
