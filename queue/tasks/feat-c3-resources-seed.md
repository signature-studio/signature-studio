# feat-c3-resources-seed: 言語・テーマ・テンプレートの扱いを起動前の seed に揃え、見つからなかったファイルの記録を出す(C-3。P1)

mode: agent
規模: 大

## 目的
依頼者の言葉: 「言語パック、テーマ、プロジェクトテンプレートなど FL Studio 自体が必要とする諸々は全く揃っていない」(台帳 L-033 の (3))。queue/plan.md の C-3 を、今の抽出の木の範囲で実装する。実装の中心は次の 2 つ。(a) 言語・テーマ・テンプレートを、起動前に取る seed pack に入れる(L-035 の区分)。(b) エンジンがファイルを要求して見つからなかったパスを記録し、queue/findings/ に出す(後で、インストーラの一覧と突き合わせる材料)。

## 背景(survey の実測。出典: queue/findings/survey-c3-fl-resources.md の `## 実測`)
- 抽出の木(lift/out-app/host)は 238 件。seed 表のファイル行 227 件と DLL 表の 11 件に、件数で一致する。名前の照合で、seed 表の 227 件はすべて抽出の木にある(missing = 0)。つまり、抽出の木は seed 表と同じ範囲で、表に無い言語・テーマ・テンプレートのファイルは、今の木には無い。
- 分類の件数(seed 表。実測): 言語 System\Languages\ は 1 件(en.svg、899 B)。テーマ Artwork\Skins\ は 3 件(343,555 B)。テンプレート Data\Templates\ はディレクトリの行だけで、ファイルは 0 件。デモ Data\Demo projects\ は 1 件(313,356 B)。既定の Data\System\Empty.flp は 1 件(46,361 B)。
- 起動の記録(survey-c3 の 5 節): engineLog の 322 行に、「見つからない」「not found」「ENOENT」「no such」「not exist」の語は 0 件。失敗らしい行は 3 種類だけ(InternetOpenUrlW の 2 件、描画の破棄 1 件、entry からの戻り 1 件)。つまり、今の記録の仕組みでは、見つからなかったファイルは見えない。
- seed pack: FLSEEDP1。先頭 8 バイトは magic、続く u32(LE)がヘッダの長さ、その後が JSON のヘッダ(ファイルの path と size)。全体 20,780,949 B(survey-c3 の 6 節。ヘッダ件数 231 = 表のファイル 227 + extras 4)。作り方: app/scripts/pack-seed.mjs(1〜8 行の注釈)。
- 起動の時間(survey-c1・c3 の実測。runner の値): navigate から engine-idle の確認まで 10205〜10584 ms(poll を含む)。

## 方針(設計の細部は作業者が決める。制約は下の「制約」)
- (a) 分類: seed pack の中身を、言語(System\Languages\)・テーマ(Artwork\Skins\)・テンプレート(Data\Templates\)・プロジェクト(Data\System\、Data\Demo projects\)の 4 つに分けて件数と大きさを出す道具を作る。seed pack は起動前の取得に入っている(L-035 の区分の「起動前」)。
- (b) 見つからなかった記録: エンジンのファイル要求(VFS の open・stat・CreateFile など)が、ホストにファイルが無くて失敗した時、その guest のパスを、engine のログの 1 行(既存の pushLog の経路。app/src/engine/worker.ts)に出す。どこで失敗を見るかは、lift/runtime の VFS(lift/runtime/fle_vfs.c)を読んで決める。新しい環境変数・#ifdef を足さない。
- (c) 出力: 見つからなかったパスの一覧を、tools/feat-c3-resources-seed/measure.mjs が stdout に JSON で出す。作業者は、その一覧を queue/findings/feat-c3-missing-paths.md に書いて commit する(実行の出力を貼るだけでなく、ファイルとして commit する)。
- 件数を増やすことは、この task の受け入れに入れない。抽出の木が seed 表と同じ範囲なので、増やせるのはインストーラの中身を抽出した後(別の作業: inventory と extract の変更)である。報告に、増やせなかった理由をこの事実で書く。

## 制約(守る。台帳の依頼者の指示)
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(既定で有効。L-002)。
- 抽出の段(app/scripts/extract-fl.mjs・.github/workflows/agent.yml の extract ジョブ・lift/crates/fe-build の extract_steps)を変えない。抽出の鍵が変わり、全件の作業が抽出し直しになるため。
- ベンダー名・Cloudflare のアカウント ID・ホスト名・絶対パスを書かない。wrangler.jsonc を書き換える仕組みを作らない。マスクの命令を置かない。Python を使わない。
- commit は環境の git の名義のまま(git config・git -c・--author・GIT_* を使わない)。commit に Claude の行を付けない。ブランチを作らない。
- app/・lift/ を変える push は main の deploy.yml を動かす。報告に、変えたパスを書く。
- 他のタスク(feat-delivery-lazy-load が seed pack と DLL pack の区分を変えうる。feat-c2-render-worker と feat-c1-hidpi は app/src/engine を変えうる)と衝突しうる。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、下の受け入れをもう一度通してから commit する。

## 読む物(実在を確認済み)
- lift/harness/seeds/vfs-seed-idle1.tsv(247 行。guest のパス、抽出先、SHA-256)、lift/harness/seeds/vfs-seed-fonts.tsv、ci/extract-dlls.tsv
- app/scripts/pack-seed.mjs(1〜8 行: FLSEEDP1 の作り)、app/scripts/pack-assets.mjs(seed pack の渡し方)
- lift/out-app/seed/seed.bin と seed.bin.manifest.json(build の出力。bundle の段で作られる)
- lift/runtime/fle_vfs.c(ファイルの open・stat・read の失敗の扱い)、lift/runtime/fle_load.c(LoadLibrary の失敗の扱い)
- app/src/engine/worker.ts(pushLog。engine のログの経路)、app/src/main.ts(engineLog の公開)
- queue/findings/survey-c3-fl-resources.md(`## 実測` の 5〜7 節)
- tools/boot-check/boot-bundle.mjs、tools/boot-check/cdp.mjs

## やること
1. 変更の前に、現状を 1 回測る: 起動の engineLog の全件(322 行付近)を取り、ファイルに関係する行の一覧を報告に貼る。seed pack の 4 分類の件数と大きさ(上の数字の確認)。
2. (b) の記録の実装: VFS の失敗の経路を読み、見つからなかった guest のパスを 1 行で engine のログに出す(既存の pushLog)。ログの行の形は、他の行と混ざらないものにする。
3. (a) の分類の道具: tools/feat-c3-resources-seed/measure.mjs。seed pack のヘッダ(FLSEEDP1 の JSON)を読み、4 分類の件数と大きさを出す。
4. 測定の道具の出力(stdout は JSON の 1 行): 3 回起動し、各回 {"state","standIn","error","elapsedMs","seedCounts":{"languageFiles","themeFiles","templateFiles","templateDirs","projectFiles"},"missing":[guest のパス]} の 1 行ずつを出す。seedCounts は手順 3 の分類の件数。missing は、起動の engineLog の記録から集める。
5. 見つからなかったパスの一覧を、queue/findings/feat-c3-missing-paths.md に書いて commit する(パス、回数、どの起動で出たか)。見つからなかったパスが 0 件なら、その事実と、確かめ方を書く。

## 受け入れ
下の accept の行が 0 で終わること。内容は次のとおり。
- 通常のビルド(app・cargo・fe-build engine・fe-build bundle)と、boot.json の起動確認(state が engine-idle、standIn が false、error が null)。
- 測定の道具の 3 行を node -e が読む。3 行とも state が engine-idle、standIn が false、error が null、seedCounts の 5 項目が整数、missing が配列。elapsedMs の 3 回の中央値が 12000 ms 以下。
- queue/findings/feat-c3-missing-paths.md が、空でないファイルとして commit されている。
- 件数の増加は判定しない(上の方針の注意)。報告に、増えなかった理由を書く。

accept: npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c3-resources-seed/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c3.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const med=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b)[1];const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&Object.values(r.seedCounts).every(Number.isInteger)&&Array.isArray(r.missing))&&med<=12000;process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c3.jsonl" && test -s queue/findings/feat-c3-missing-paths.md

## 報告の規則
- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg)を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 件数が増えなかった理由(抽出の木 = seed 表の事実)、見つからなかったパスの一覧の要点、変えたパス(app/・lift/ を含む)、elapsedMs の 3 回の値と中央値。
- 確認できなかった物は「確認できない」と書く。

## 禁止と注意
- 測定の値を、accept の判定のために書き換えない(道具の出力の生の値を、accept の node -e が読む)。
- 抽出の段を変えない(制約を参照)。
- sleep・timeout・nohup・disown・末尾の & は使わない(待ちは JS のタイマー)。
- 一時ファイルは .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。
- ログ・Artifacts・run を削除しない。
