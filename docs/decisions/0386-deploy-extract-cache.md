# 0386: 抽出の結果を R2 のキャッシュに入れ、同じ入力なら復元する(deploy-engine と作業用エージェント)。前任の決定 0351 の「キャッシュは入れない」を置き換える
分類: c 通信・変換の削減(AGENTS.md §6)。インストーラの取得(1,106,033,456 B)と展開を、同じ入力の 2 回目以降で省く。bundle の出力は変えない(下の実測)。R2 の鍵を抽出の段に限る決定は運用の決定で、lift の処理ではないので (a)(b)(c) の対象外。
撤回条件: (1) 復元した tree の SHA-256 が保存時と違う、または、同じ入力のキャッシュ無しの実行と wasm の SHA-256 が違う、が 1 回でも起きたら、復元を止めて取得と抽出に戻す。(2) 復元の段が、取得と抽出の合計(実測 9〜12 秒)より長い実行が 2 回続いたら、この決定の効果を見直す。(3) R2 の鍵を抽出の段以外(モデルの段・ビルドの段・配信の段)に渡す変更が入ったら、この決定の違反として差し戻す。
日付: 2026-11-16   ROADMAP: R6-8(表に行は無い。0387 の一覧の C3 に当たる)   一覧: C3(evidence/R6-9.md)   コミット: 939f734(抽出の層と deploy-engine)、abce180(作業用エージェント)、71616ce(注記)、この文書の commit(後続)

## 検索(範囲と件数。依頼者の過去の発言と、置き換える前の物)
- `~/work/handover/setup/S16-intent/catalog.md`(84 行。依頼者の発言を字義どおりに読んだ事例の一覧。発言の全文ではない): 「キャッシュ」0 件、「抽出」0 件、「cache」0 件。
- `docs/ops/intent.md`: 「キャッシュ」1 件(77 行「抽出キャッシュの欠落: 出典は未確認」)。
- `docs/decisions/`(`grep -rlc キャッシュ`): 0351 の 1 件(22 行)。0387 は、段階キャッシュの 12 件(C1・C2・C4・C6〜C14)を「戻す」と分類している。抽出のキャッシュ(C3)は evidence/R6-9.md の 39 行で「残っている」と書かれ、この決定で実装する。
- 依頼者の 2026-11-16 の発言(msg_1e28f3a20001GYppSYhLxztoMh。「前任者と同じことをやっているのですよね?」): evidence/R6-9.md の 92 行に出典。
- 未確認: `opencode api session.message.get` による発言の全文の検索(この worker の権限では未確認)。

## 決定
決定 1: deploy-engine の抽出の層と、作業用エージェントの抽出の層は、同じ鍵の抽出キャッシュを使う。キャッシュは R2 の fl-web-cache の extract-cache/<鍵>/(host.tar・host.tar.sha256・SHA256SUMS の 3 件。SHA256SUMS を最後に書く)。根拠: 依頼者の指摘(msg_1e28f3a20001GYppSYhLxztoMh)。前任の設計は段ごとのキャッシュを持っていた(archive/ci-old/deploy.yml 226〜233 行の lookup extract、33 行の R2_CACHE_BUCKET)。

決定 2(鍵): 鍵 = SHA-256(「extract-layer v1」の行 + ci/artifacts.lock の FL Studio 5653 の行 + `git ls-files -s` の 7 項目: ci/extract-dlls.tsv・ci/lib/extract_fl.py・tools/extract_nsis2.py・tools/nsis_script_dump.py・lift/harness/seeds/gen_install_tree.py・lift/harness/seeds/vfs-seed-idle1.tsv・lift/harness/seeds/os-fonts)。根拠: 前任の鍵の入力(archive/ci-old/ci/plan.sh 15〜16 行)と同じ。抽出は repo の os-fonts を書くため、前任の鍵に無かった os-fonts を加える。prefix の鍵(deploy-engine.yml の key 段)と同じ方法(git ls-files -s)。

決定 3(復元の検査): 保存時の記録と照合する。(a) host.tar の SHA-256 = host.tar.sha256 の記録、(b) tar の中身に絶対パス・`..`・リンクが無い、(c) 展開後のファイル数 = SHA256SUMS の行数、(d) 各ファイルの SHA-256 = SHA256SUMS。いずれかが違えば、キャッシュを使わず(HOST を消して)取得と抽出をする(hit=false)。根拠: 依頼者の指示(不一致ならキャッシュを使わず抽出し直す)。保存の後は、tar と記録を読み直して一致を確かめる。

決定 4(鍵の範囲): R2 の鍵は 2 組ある。「fl-web-artifacts の読み取り」(R2_ARTIFACT_*)と「fl-web-cache と fl-web-assets の読み書き」(R2_CACHE_ASSETS_*)。後者は配信バケット fl-web-assets(本番が読む)を書ける(docs/requests/0001-cloudflare-actions-secrets.md 22〜23 行)。決定: R2 の鍵は、抽出の層の段(.github/actions/extract-layer。deploy-engine の段 layer、agent の extract ジョブと worker ジョブの復元の段)にだけ渡す。モデルが動く段(ci/agents/run-task.sh の opencode run)・ビルドの段・配信の段には渡さない。根拠: 作業用エージェントの後続の段は、同じジョブでモデルを動かすので、そこに鍵を渡すと配信バケットを書き換えられる。CLOUDFLARE_API_TOKEN も同じ段にだけ渡す。根拠: 復元の endpoint のアカウント ID を実行時に得るため(deploy-engine の acct の段と同じ方法。値は出さない)。依頼の 2 組の鍵に、このトークンは含まれていなかった。追加の理由は、endpoint を repo に書けないこと。

決定 5(agent.yml の形): extract ジョブを plan の後・matrix の前に 1 つ置き、キャッシュが無ければ作る。matrix の各 worker ジョブは、モデルの段より前に復元する(hit が期待)。根拠: 60 並列で同時に取得させない。worker は復元済みの $RUNNER_TEMP/host を読むだけ(ci/tasks/0002-extract-cache-check.md は読み取りだけ)。

決定 6(前任の決定の置き換え): 決定 0351 の 22 行「キャッシュは入れない(冷たい 1 回を先に成功させる。6 時間の上限の内)。」を、この決定が置き換える。0351 の本文は変えず、0351 に参照の 1 行を足す。根拠: 0351 は、旧の段階キャッシュ(archive/ci-old/ci/plan.sh・needs.sh)を書き直す前提で、抽出の再利用を捨てた(0387 の決定 7 の前提の訂正)。依頼者の指摘(msg_1e28f3a20001GYppSYhLxztoMh)で、その判断が誤りと確かめられた。

決定 7(bundle を変えない): キャッシュの有無で bundle の出力は変わらない。根拠: prefix(入力のハッシュ)が同じ fl-engine-ci-57fbed1a の 2 回の実行で、wasm の SHA-256 が一致する(下の実測)。

決定 8(範囲): 9 段のビルドのキャッシュは、この決定に入れない。根拠: 依頼の範囲は抽出の層。9 段は別の行(docs/ROADMAP.md の R6-10: 残りの段の鍵・判定と、段ごとの復元・保存)。

## 実測(evidence/R6-8.md に表)
- 実行 43804191943(R1。保存。headSha 1aff9bb、ジョブ 1563 秒、結論 success)。抽出の層(段 5)39 秒: 「抽出キャッシュ: 使わない(無い(404))」→「照合: 一致(大きさ 1106033456 B、SHA-256 一致)」→「抽出: 成功(rc=0。記録した SHA-256 の件数 232)」→「保存: 3 件を書き、読み直して SHA-256 を確かめた(tar 87121920 B)」。wasm: 143118528 B、sha256 b633337a…(prefix fl-engine-ci-57fbed1a)。
- 実行 43798605485(キャッシュ無し。prefix fl-engine-ci-57fbed1a)の wasm: 143118528 B、sha256 b633337a…。R1 と一致。
- 実行 43804510696(作業用エージェント。headSha abce180。結論 success)。extract ジョブと worker ジョブ(0002)の抽出の層: 「抽出キャッシュ: 復元した(232 件。保存時の SHA-256 と一致)」。worker の報告(成果物。evidence/R6-8.md): ファイル数 232、直下の名前 binaries・lift、変更 0。
- 実行 43806250386(R2。復元。headSha 71616ce、ジョブ 1664 秒、結論 success)。抽出の層(段 5)9 秒: 「抽出キャッシュ: 復元した(232 件。保存時の SHA-256 と一致)」。取得・照合・保存の行は出ていない。wasm: 143118528 B、sha256 b633337a…(R1 と同じ)。取得 + 抽出の 9〜12 秒と比べて、復元は同程度で、時間は減っていない(減るのは 1.1 GB の取得の通信)。

## 読み(解釈。実行していない)
- 取得 + 抽出の合計は、キャッシュ無しの実行 A・B で 9 秒・12 秒(実行の 0.35%・0.47%)。保存込みの段 5 は R1 で 39 秒。復元の時間は R2 の実測で決める。キャッシュの効果が大きいのは作業用エージェントの側(60 ジョブが各々 1.1 GB を取らずに済む)。
- 9 段の時間は実行ごとに違う(A 2439 秒、B 2450 秒、R1 1456 秒、R2 1586 秒)。同じ prefix の B と R1・R2 で違う理由は未確認。ジョブ全体は、B の 2573 秒に対して R1 1563 秒・R2 1664 秒と短いが、抽出の層の時間(9〜39 秒)では説明できない。差の原因は未確認。

## 未確認
- R2 の復元の時間と結論(実行 43806250386 で確かめる。evidence/R6-8.md)。
- 依頼者の発言の全文の検索(上の検索の節)。
- 公開ホストの確認(不要・不可)。

## 出典
- docs/decisions/0351-engine-bundle-workflow.md 22 行(置き換える文)
- archive/ci-old/deploy.yml 1、3〜5、33、226〜233 行。archive/ci-old/ci/plan.sh 15〜16 行。archive/ci-old/ci/actions/r2-cache/action.yml
- docs/requests/0001-cloudflare-actions-secrets.md 22〜25 行(R2 の鍵の範囲)
- evidence/R6-8.md(実測の表)、evidence/R6-9.md 39・92 行(機能 C3 と依頼者の発言)、docs/decisions/0387-old-ci-feature-diff.md(戻す物の分類)
