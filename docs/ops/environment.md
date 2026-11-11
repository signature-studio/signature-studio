# この機械の能力の事実表

目的: 後任が「できるはずなのにできない」と言わないための表。**「できない」と書く前にこの表を読み、該当行のコマンドを自分で実行する**。
出所の略: 「R1」= 前任の資料を調べた担当の実測(2026-11-09 13:09〜13:20 JST)。「再」= この文書の作成時に再確認(2026-11-09)。
記号: **[実測]** コマンド出力で確認 / **[文書]** 文書の記述 / **[未確認]** 見ていない。値(アカウント ID・ホスト名・トークン)は書かない。表の値は古くなる。**食い違ったら、`scripts/preflight.sh` と実コマンドの出力を採り、worker にこの表を直させる**。

## 1 GitHub

| 項目 | 事実 | 区分 | 出所・日付 | 確認コマンド |
|---|---|---|---|---|
| リポジトリ | `signature-studio/signature-studio`。remote `origin` は HTTPS。履歴は `Initial commit` の 1 つだけ(人間が web で作成)。追跡ファイルは `README.md` のみだった | [実測] | R1、2026-11-09 | `git log --oneline`、`git remote -v` |
| 公開範囲 | **PUBLIC。意図的**(依頼者の決定 P2: GitHub Actions を最大 60 並列で数時間動かすため。private ではその利用量を担えない)。公開されて困る物は git に入れない(`.gitignore`)。default branch `main` | [実測] 2026-11-10 | 依頼者の決定 P2(2026-11-10)、R1 の実測(2026-11-09) | `gh repo view signature-studio/signature-studio --json visibility` |
| シェル既定の identity | GitHub App の bot(個人ではない)。`gh` 2.103.0。credential helper は `gh auth git-credential` | [実測] | R1、2026-11-09 | `gh auth status` |
| **push** | identity は作業ディレクトリで決まる(依頼者の決定 P1)。前任期間の「403」はリポジトリ外のディレクトリで測った値で、誤り [文書]。`.permissions` は根拠にしない。**2026-11-10 08:27 JST にリポジトリのディレクトリで `git push --dry-run origin HEAD` を実行: rc=0。出力の先頭行は `To https://github.com/signature-studio/signature-studio.git`、次の行は `29966d6..a70e4ef  HEAD -> main`**。[文書・旧] 同日の前の記述「rc=1(PUBLIC のための `.githooks/pre-push` の拒否)」は前日の誤り(拒否した hook は修正済み。2026-11-10 にフックはスキャンだけに直った) | [実測] | 2026-11-10 | `git push --dry-run origin HEAD`(リポジトリのディレクトリ)、`gh repo view signature-studio/signature-studio --json visibility` |
| Secrets・Variables | R1(2026-11-09)の一覧は 403。新リポジトリに旧 5 secret があるか不明(旧リポジトリの secret は旧リポジトリと共に消えた)。登録は依頼者の権限(P3)で、Issue で頼む(`docs/ops/authority.md` §3) | [実測](R1) / 存在は [未確認] | R1、2026-11-09 | エージェントは読まない(一覧も含む。依頼者が許可していない)。確認は依頼者の連絡と、確認用ワークフローの実行結果 |
| workflow | なし(`total_count: 0`) | [実測] | R1、2026-11-09 | `gh workflow list` |
| commit 用 identity | **未設定**(`Author identity unknown`)。`git config user.name` が空 | [実測] | R1、2026-11-09 | `git var GIT_AUTHOR_IDENT` |
| 別 identity | 前任は別の identity を使った。**使用禁止**。新リポジトリで push できるかは確認していない | [未確認] | R1 | — |

- 解釈(2026-11-10 更新): 前任の「push は 403。公開範囲を理由に push を止める」は、リポジトリ外のディレクトリで測った値と、古い規則に基づく。依頼者の決定 P1・P2 により、PUBLIC は理由にならず、403 は作業ディレクトリで確かめる。`.githooks/pre-push` が PUBLIC を理由に拒否する現状は P2 と食い違う(フックは別の担当が直す)。
- 404 や 403 は identity で変わる。**404 を権限の喪失と書く前に `gh auth status`**(前任は identity の違いを権限の喪失と約 48 分誤報した)。

## 2 Cloudflare

| 項目 | 事実 | 区分 | 出所・日付 | 確認コマンド |
|---|---|---|---|---|
| `wrangler` | **後任は実行しない(deny)**。読み取りも含む(依頼者の決定 P4)。強制は worker・reviewer の permission の deny(`AGENTS.md` §11)。前任期間に `~/.local/bin/wrangler` が OAuth でログイン済みだったのは [文書](R1、2026-11-09) | [文書] | 依頼者の決定 P4(2026-11-10)、R1(2026-11-09) | 確認は GitHub Actions の結果か Issue(`docs/ops/requests.md`)の確認コマンドで行う。確認コマンドは書かない |
| `cf` CLI | **無い** | [実測] | R1(再: `command -v cf` 出力なし) | `command -v cf` |
| 旧 Worker と R2 | 前任期間の記録 [文書](R1、2026-11-09): Worker `fl-studio-web`、R2 `fl-web-*`。後任は確認も変更もしない(P4)。**触ってよいのはこの 2 つだけ**(依頼者の方針。他の資源には触らない) | [文書] | R1、2026-11-09、依頼者の決定 P4 | 確認は GitHub Actions の結果か Issue の確認コマンド |
| 旧公開ホスト | 前任期間の記録 [文書](R1、2026-11-09): 認証なしの GET が Access のログインへ転送。稼働の確認は Issue で頼む(後任は確認しない) | [文書] | R1、2026-11-09 | Issue(`docs/requests/`)の確認コマンド |
| Access・DNS・custom domain | 後任の権限外。変更は Issue で頼む(`docs/ops/authority.md` §3 の 2) | [文書] | 依頼者の決定 P3・P4 | Issue |
| Cloudflare の API トークン・R2 の S3 キー | 発行・登録は依頼者の権限(Issue で頼む。`docs/ops/authority.md` §3 の 1) | [文書] | 依頼者の決定 P3 | Issue |

## 3 ツールチェーンと PATH

| 項目 | 事実 | 区分 | 出所・日付 | 確認コマンド |
|---|---|---|---|---|
| node | v24.19.0。`~/.local/bin/node`(`/opt/emsdk/node/...` への symlink) | [実測] | R1、2026-11-09(再: `command -v node` で確認) | `node --version` |
| **npx** | **PATH に無い**。実体は `/opt/emsdk/node/24.19.0_64bit/bin/npx` | [実測] | R1、2026-11-09(再: `command -v npx` 出力なし) | `source scripts/env.sh && command -v npx` |
| **emcc** | **PATH に無い**。`/opt/emsdk/upstream/emscripten/emcc` で 6.0.12(`EM_CONFIG=/opt/emsdk/.emscripten`)。旧 CI の想定と一致 | [実測] | R1、2026-11-09(再: `command -v emcc` 出力なし、ファイルは存在) | `source scripts/env.sh && emcc --version` |
| wasm-opt | PATH 上は `/usr/bin/wasm-opt` = version 120。`/opt/emsdk/upstream/bin/wasm-opt` = version 133(旧 CI の想定)。`scripts/env.sh` で後者を前に置く | [実測] | R1、2026-11-09 | `wasm-opt --version` |
| rust | rustc / cargo 1.99.0(stable)。target `wasm32-unknown-unknown`。`wasm-pack` あり | [実測] | R1、2026-11-09 | `rustup show` |
| clang / gcc / make / cmake | clang 21.1.8、gcc 15.2.0、make 4.4.1、cmake 4.2.3。ninja・wasm-ld あり | [実測] | R1、2026-11-09 | `clang --version` |
| wine | wine-10.0。prefix の状態・実行は未確認 | [実測] / 動作は [未確認] | R1、2026-11-09 | `wine --version` |
| Chrome | `google-chrome` 156。headless 起動は未確認。chromium は無い | [実測] / 起動は [未確認] | R1、2026-11-09 | `google-chrome --headless --version` |
| playwright | **無い**(npm の registry には到達できる)。手元の検証に限って入れてよい。CI には置かない | [実測] | R1、2026-11-09 | `command -v playwright` |
| brotli CLI / bats | 無い。要否は旧 CI を読んで worker が決める。`sudo -n` で apt が使える見込み | [実測] | R1、2026-11-09 | `command -v brotli` |
| その他 | jq 1.8.1、shellcheck 0.11.0、sqlite3 3.46.1、7z 26.00、python3 3.14.8、pnpm 12.9.1、gh 2.103.0、opencode v2.2.6、docker 29.9.2(他プロジェクトのコンテナが稼働) | [実測] | R1、2026-11-09 | `--version` |
| sudo | `sudo -n true` が rc=0。`(root) NOPASSWD: ALL` | [実測] | R1、2026-11-09 | `sudo -n true` |
| `systemctl --user` | 可(linger あり)。goal-nudge の timer を使う | [実測] | R1、2026-11-09 | `systemctl --user list-timers` |
| ネットワーク | npm・GitHub・Rust の配布先に到達できる(HTTP 200) | [実測] | R1、2026-11-09 | `curl -sS -o /dev/null -w '%{http_code}'` |
| OS | Ubuntu 26.04.1 LTS、Linux 7.0.0 x86_64 | [実測] | R1、2026-11-09 | `uname -r` |

## 4 資源

| 項目 | 事実 | 区分 | 出所・日付 | 確認コマンド |
|---|---|---|---|---|
| ディスク(`~`・`~/work`・`/opt`) | 同じ `/`(ext4)。空き 648 GiB(使用 33%) | [実測] | R1(再: `df -h /` で 648G) 2026-11-09 | `df -h /` |
| **`/tmp`** | **tmpfs 3.7 GiB(空き 3.6 GiB)**。FL インストーラ(1.1 GB)の展開・大きな clone・lift の出力は入らない(前任の clone が容量不足で失敗した) | [実測] | R1(再: `df -h /tmp` で 3.6G) 2026-11-09 | `df -h /tmp` |
| 作業場所 | `TMPDIR` と作業場所を `/` 側に置く(`source scripts/env.sh`)。**大きな物を作ったら掃除する**。前任は 598 GB の占有を依頼者に是正させられた | [実測] / 掃除の規則は [推奨] | R1、2026-11-09 | `du -sx --block-size=1M <dir>`(background) |
| メモリ・CPU | 14 GiB(available 約 10 GiB)、swap 4 GiB、`nproc` = 16 | [実測] | R1、2026-11-09 | `free -h`、`nproc` |

## 5 入力と旧ワークツリー

| 項目 | 事実 | 区分 | 出所・日付 |
|---|---|---|---|
| 所定のプロジェクト | `~/ダウンロード/serina1-serum2.zip`(53,489,451 B)。中身は未解析 | [実測] | R1、2026-11-09 |
| FL インストーラ | `flstudio_win64_26.1.7.5653.exe`(1,106,033,456 B。旧 `ci/artifacts.lock` のサイズと一致。SHA-256 は旧記録と一致と前任が記録)。別版 5654 もある(lock に無い) | サイズ [実測] / ハッシュは [文書] | R1、2026-11-09 |
| 他のインストーラ | Serum 2、OTT、Vital(サイズは旧 lock と一致。ハッシュは未照合) | サイズ [実測] / ハッシュ [未確認] | R1、2026-11-09 |
| 旧ワークツリー | `~/work/` 直下に残っている前任のワークツリー(約 173 GB。場所は `docs/ops/lessons.md` に 1 回だけ書く)。**`.git` は無い**。参照用。読み取りのみ(複製・移動・削除をしない)。`find`・`du` を前景で回さない | [実測] | R1、2026-11-09 |
| 旧 `.github/workflows/` | `deploy.yml` のみ(1,128 行)。必要な Secrets は 5 つ(名前のみ。値は書かない) | [実測] | R1、2026-11-09 |

## 6 ハーネス(S2 が作る物。作成後に実測で確認)

| 項目 | 状態 |
|---|---|
| `scripts/preflight.sh`、`audit-session.sh`、`scan-forbidden.sh`、`env.sh`、`set-model-profile.sh`、`launch.sh`、`setup-repo.sh` | 作成済み(`ls scripts/` で 2026-11-10 に実在を確認)。存在しなければ `ls scripts/` の出力を報告に貼り、worker に作らせる |
| agent: `build`(Sonnet・読み取りと許可リストのみ)、`worker`・`reviewer`(Haiku)。`general`/`explore`/`plan` は無効 | 作成済み(`opencode.jsonc` に定義)。`scripts/preflight.sh` が解決結果を表で出す |
| モデル ID(参考) | Sonnet 5.5 と Haiku 5.5 は、前任が実際に動かした経路で指定できる(R3、2026-11-09 の調査)。設定ファイルが不正だと全設定が黙って無効になる。**起動前に `opencode debug agents`** で解決された model を確認する |
