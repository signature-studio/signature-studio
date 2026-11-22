## 目的

依頼者の指示: 内蔵のプラグインを、起動後にバックグラウンドで届ける(1 プラグイン 1 wasm)。@@KIND@@のうち、次の 3 フォルダ(@@NAMES@@)に入る DLL を、#{{015}} と #{{019}} で確かめた型に乗せる。

## 背景

- 対象は@@KIND@@の 3 フォルダ。フォルダは、プラグインの単位(`Plugins\Fruity\<種別>\<フォルダ>\`)で、本体の DLL と、その補助の DLL(本体が使う。入口を持たない)は同じフォルダに在る。名前・大きさ・SHA-256・export・import・hook の要否は、分類の表(`inputs/plugin-classes.tsv`。#{{103}} の成果)の値を写した。
@@PLUGINS@@
- hook 点: 本体の `hook の要否` が 要 の物は、hook 点の表(`inputs/plugin-hooks.tsv`。#{{104}} の成果)の 3 つの RVA を使う。既存 の物は、既存の 3 点(`lift/harness/hooks/fruity-delphi-effect-hook-points.txt`)を使う。
- 型: #{{015}}(Fruity Balance を起動後の取得に移す、最初の 1 件)と #{{019}}(Fruity Filter と Fruity Delay 2)で、1 プラグイン 1 wasm の型を確かめる。この Issue は、その型を 3 フォルダに当てる。
- 親: @@PARENT_REF@@(@@PARENT_LABEL@@)の子。
- OpenGL を使う VFX 系のフォルダ(フォルダの DLL に `VFX` が yes の物が在る。補助の DLL が OpenGL を import する場合を含む)は、この Issue に含めない(`inputs/vfx-deferred.tsv` に在る。#{{039}} で全群の後に入れる)。

## 読む物

- `inputs/plugin-classes.tsv`(この 3 フォルダの行)
- `inputs/plugin-hooks.tsv`(hook 点の表。#{{104}} の成果)
- `ci/extract-dlls.tsv`(抽出の表。#{{015}} と #{{019}} の行の形)
- `lift/harness/b2d2/modules.tsv`(登録の行の形と、既存の行の base)
- `lift/harness/web/pack.args`(`--args` の行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(seed の表の行)
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(hook 点)
- `tools/delivery-split/measure.mjs`(起動後の取得の測り方)
- `tools/boot-check/boot-bundle.mjs`(起動確認)
- `pm-work/issue-drafts/015-fruity-balance-lazy.md` と `pm-work/issue-drafts/019-fruity-filter-delay2-lazy.md`(型の手順)

## 手順

TODO(#1315 の結果で確定): 1 DLL を足す手数(一覧・表・seed・pack.args・hook 点・lift と起動確認)は、#{{015}} の結果で決める。下は #{{015}} の手順の写しで、確定までの仮。
1. 3 フォルダの本体の DLL それぞれが、起動前の取得と起動後の取得のどちらに入っているかを、変更の前に確かめる(報告に書く)。
2. 3 フォルダの本体の DLL を起動後の取得に移す。1 プラグイン 1 wasm の形で、起動後に届くようにする。補助の DLL は、同じフォルダの物として、同じ取得の単位に入れる(入れ方を報告に書く)。届いた後に、engine がどう読み込むか(インスタンス化の場所、Worker の場所)を、報告に書く。
3. engine が起動時に本体の DLL のいずれかを要求する場合は、要求を満たす形(空の代替、遅延の読み込みの待ち、等)を決め、理由を報告に書く。
4. 道具の出力を確かめる。本体の DLL が起動前の取得に無く、起動後の取得に在り、取得の開始が起動前の取得の完了以後であること。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-lazy.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const P=[@@PATTERNS@@];const has=(a,re)=>a.some((f)=>re.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&sum(r.lazy)>0&&P.every((re)=>!has(r.preStart,re)&&has(r.lazy,re)&&r.fetches.some((f)=>re.test(f.name)&&f.startMs>=r.preStartDoneMs)));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-lazy.jsonl"
```

判定の基準: 3 回の起動で、本体の DLL が全て起動前の取得に無く、起動後の取得に在り、取得の開始が起動前の取得の完了以後であること。取得の開始が engine-idle より前にある場合は、その事実を報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: engine が本体の DLL を起動時に要求するかの読みと結果、補助の DLL の入れ方、変えたパス(lift/・app/ を含む)、起動前と起動後の合計の大きさ(変更の前後)、elapsedMs の 3 回の値。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限(例: 他のプラグインの扱い)を、この作業の判断で増やさない。

## 他のタスクとの関係

- blocked_by: #{{015}}(1 プラグイン 1 wasm の型)と #{{019}}(Fruity Filter と Fruity Delay 2 の型)。
- 同じ親の下の他の束の Issue と、表(`ci/extract-dlls.tsv`・`lift/harness/b2d2/modules.tsv`・seed・`lift/harness/web/pack.args`)の行を、同じ箇所に足しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
