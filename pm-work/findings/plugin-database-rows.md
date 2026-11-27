# Plugin database の Installed に、プラグインの行が出ない: 読みによる切り分け

状態: 読みの結果(この作業では製品のビルド・実行をしていない)。`pm-work/issue-drafts/wave1bn/249-plugin-database-installed-rows.md` の根拠(下書き番号は 249。246〜248 は別の作業者の下書き)。
読んだ範囲: `git` の main(cf8e3a24 の時点で読み、3a647c57 まで pull した。読んだ製品のファイルは pull の前後で変わっていない)。`.tmp-art/1508/`(#1508 の作業者の報告。読むだけ)。

## 0. 結論

1. **E2E の判定の欠陥(確定。読みで)**: `tools/e2e/lib/plugins.mjs` の `databaseHas` は、「プラグイン名を含み、かつ `Plugin database\Installed` を含むエンジンのログの行」を求める。エンジンのログは、Installed について件数の行(`enum-dir: N "…Installed\*.*"`)しか出さず、名前は出ない。さらに、成功した読み・開きは記録されない(記録されるのは「無い」と「列挙」だけ)。よって、製品の状態に関係なく「no entry」になる。e2e-full-2 の 146 本の「no entry」は、製品の状態の証拠にならない。
2. **製品の欠陥(最有力だが推定)**: Installed の一覧に子が出ない(#1508 の手動の探索。こちらでは再現していない)。原因の最有力は、FL の Plugin database の木が読む**分類データ**(`Data\Patches\Plugin database\` の `Installed.nfo`・`Effects.nfo`・`Generators.nfo`)が VFS に無いこと。`pm-work/findings/missing-paths.md`(#1316。3 回の起動。#1484 より前)で、この 3 本は 3 回とも探されて無かった(コード 3)。インストーラには在る(大きさ 67・71・61 B)が、seed の行が無いので抽出されず、bundle に入らない。
3. **製品の項目の形式も違う(推定)**: 製品が作る Installed の項目は `Tip=<名前>\r\n`(名前 + 6 B)。インストーラの実物の `Plugin database (alphabetical)` の `.nfo` は、136 件すべて「大きさ − 名前の長さ = 11」。実物は `.fst`・`.png` も持つ。製品は `.nfo` だけ。
4. **候補の判定**: (a) 一覧の元データが無い → Installed の中の 3 件は VFS に在る(enum-dir: 3)ので、「Installed が空」は退ける。分類データの欠落は (a) の変種として残る。(b) WebView2 が被る → Plugin database の木には根拠が無い(確認できない)。(c) E2E の測り方 → **確定**(上の 1)。(d) 読む時機 → 根拠が無い(確認できない)。
5. **位置のシナリオは main では必ず落ちる**: `tools/e2e/scenarios/plugins/position-*.mjs` は、Plugin database を開いて「その名前の窓」を探すだけで、挿入をしない。挿入の段は main に無い(`pluginSteps` の「unconfirmed in this build」)。#1508 の挿入の作業(commit `0c650834`)は main に入っていない(`git branch -a --contains 0c650834` は空)。#1508 は OPEN で `failed` のラベル。よって、#1508 が入るまで、位置のシナリオを受け入れに使えない。

## 1. 読んだ物(file:line)

- `tools/e2e/lib/plugins.mjs`: `DB_TEXT` 16 行(`/Plugin database\\Installed/`)、`databaseLines` 95 行(ログの行の絞り込み)、`databaseHas` 101 行(名前とパスの両方が 1 行に在ること)、`pluginSteps` 137–140 行(`no entry for … (the Installed folder is empty in this build)`)、挿入の注記 `unconfirmed in this build`(`pluginSteps` の中)。
- `tools/e2e/scenarios/plugins/position-effects-fruity-balance-fruity-balance-x64.mjs`: `displays: ['4k-dpr1', '4k-random']`(4k-dpr2 は含まない)、`expectFail: '#1484'`、`run` は `openPluginDatabase` → `pluginInDesktop(guest, "Fruity Balance")`。
- `tools/e2e/scenarios/smoke/plugin-database-lists-builtins.mjs`: 行の正規表現 `enum-dir: (\d+) "…Plugin database\\Installed\\…"`(件数だけを読む)。
- `tools/e2e/lib/scenario.mjs`: `displays` の既定値と宣言(`DEFAULT_DISPLAYS`)。`tools/e2e/run.mjs` 20–30 行: 行は宣言した表示でしか走らない。
- `lift/runtime/fle_vfs.c`: `note_listing` 519–524 行(件数と要求の文字列だけを出す `enum-dir: %zu`)、`fle_imp_FindFirstFileW` 1333–1366 行(`nk` は一致した子の数。`.` と `..` は数えない)、`note_missing` 136 行(`missing-path` の行は、無いパスだけ)。
- `lift/crates/fe-build/src/plugins.rs`: `PLUGIN_DB_GUEST` 37 行(`C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Installed\`)、`plugin_db_names` 611 行(内蔵の項目の名前の一覧。フォルダの名前)、`plugin_db_rows` 638–649 行(`Tip={name}\r\n` を書く。seed の行を作る)、`generate` 716–719 行(`vfs-seed-idle1.tsv` にだけ足す)。
- `app/scripts/extract-fl.mjs`: 8 行(`--seed` は seed の行の全部を抽出する)、84 行(seed の行ごとに、インストーラの木から取り出し、SHA-256 を検査する)。
- `inputs/fl-installer-files.tsv`: 6508–6511 行(`Data\Patches\Plugin database.nfo`・`…\Plugin database\Effects.nfo`(71 B)・`Generators.nfo`(61 B)・`Installed.nfo`(67 B))。`Data\System\Plugin databases\Plugin database (alphabetical|full|simple)\…` と `New\…` の `fst`・`nfo`・`png`(alphabetical だけで fst 138・nfo 139・png 136 件)。
- `lift/harness/seeds/vfs-seed-idle1.tsv` 382 行: `…\Data\Patches\Plugin database.nfo` の 1 行だけ。`lift/harness/`・`ci/`・`app/scripts/` を `Patches\Plugin database\` で探すと 0 件。
- `pm-work/findings/missing-paths.md`(#1316。`tools/missing-paths/measure.mjs` の 3 回の起動。表の「起動」の列が 1・2・3 のもの): `Data\Patches\Plugin database\Effects.nfo`・`Generators.nfo`・`Installed.nfo`(コード 3)、`…\Effects\`・`…\Generators\`・`…\Installed\`(コード 3)、`Presets\Plugin database\Installed\.Plugins.ini`(コード 3)。`Presets\Plugin database\Effects.nfo`・`Generators.nfo`・`Installed.nfo`・`Presets\Plugin database.nfo`(コード 2)。この文書は、要求の時点で seed 済みの木が見えない箇所があることも書いている(`Artwork\Skins\Default`。原因は未確認)。#1484 より前の記録。
- `pm-work/findings/crash-hunt-2026-11-25.json`(2026-11-25。#1484 より前の記録): `Data\Patches\Plugin database\Installed.nfo` が missing(コード 3)。ほかに `…\Plugin database\Installed\Effects.nfo`・`Installed\Generators.nfo`・`Installed\Effects\Fruity.nfo`・`Installed\Generators\Fruity.nfo`・`INSTALLED\EFFECTS\NEW\`・`INSTALLED\GENERATORS\`・`Effects\`・`Effects\Voice\`・`Generators\` が探されて無い。`Installed\Effects.nfo` のような下位のパスは、インストーラに無い(推定)。
- `pm-work/findings/e2e-full-2.md` 8 節(201–206 行): plugins 302 本中 ok 8 本。挿入の段で止まる 146 本(`no entry`)、窓を探す段 119 本、picker が開かない 26 本。
- `pm-work/findings/browser-panel-1355.md` 18 行: 黒い面は、起動の時に選ばれている**クラウドのタブ**の `TEdgeBrowser` の面(WebView2)。
- `.tmp-art/1508/report.md`: (a) の探索の記録(下の 2 節)。
- `gh issue view 1484`: 完了のコメント(`plugin database: 2 entries listed`)、本文の方式(`.nfo` を build で作り、seed に入れる)。

## 2. 実測・記録からの転記(こちらでは再現していない)

- `pm-work/findings/missing-paths.md`(#1316。#1484 より前): 上の 1 節の通り。3 本の分類データが、3 回の起動の全てで探されて無い。
- `.tmp-art/1508/report.md`(#1508 の作業者。エンジンのログを読んだ記録): Installed の件数は `enum-dir: 3`(Fruity Balance・Fruity Filter・Sytrus の 3 本の folder に当たる)。名前はログに出ない。Installed を選ぶと情報欄の題名は変わるが、矢印・二度押し・→・Enter・Space・↓・ホイールでも子が出ない。ピッカーの「More plugins…」の一覧は `0 effects`。2 本目以降のシナリオ(同じページ)では `enum-dir` の行が出ない(一覧はページごとに 1 回)。
- `.tmp-art/1508/report.md`: 挿入は未確認(行が描かれないので項目を選べない)。
- PM の引き継ぎ(#1524 の作業者の報告。この repo に無い): Installed の一覧は 4 行だけ描かれ、Fruity Balance も Fruity Filter も名前が出ない。ログに `webview2: put_IsVisible … isVisible=0` の行がある。**this repo では確認できない**。
- `missing-paths.md` と `crash-hunt-2026-11-25.json`(上の 1 節): FL が `Data\Patches\Plugin database\` の分類データを探して、無い。どちらも #1484 より前の記録なので、main の現在の状態とは同じとは限らない。現在の main での再測定が要る。

## 3. 読みで確かめた事実

- E1. `databaseHas` は、名前とパスを 1 行に求める。ログに名前が出る行は、`missing-path` でそのパス名が探された場合だけ。記録の中の探された名前は `Fruity.nfo`(Fruity Balance ではない)。**名前は、現在のログの形では出ない**。
- E2. ログは「無い」と「列挙の件数」だけを出す(`fle_vfs.c` 136・519–524 行)。成功した読みは出ない。よって「ログに読んだ跡が無い」は、読まなかった証拠にならない(1508 の推定は使えない)。
- E3. 製品の Installed の項目: `plugin_db_rows` は `Tip=<名前>\r\n` を書く(名前 + 6 B)。seed は `vfs-seed-idle1.tsv` に、名前ごとに 1 行。Installed の件数 3 は、この 3 行を列挙した数(`nk`)。
- E4. インストーラの分類データ 3 本(`Data\Patches\Plugin database\Effects.nfo`・`Generators.nfo`・`Installed.nfo`)は、seed に行が無い(E4 の検査: `Patches\Plugin database\` を含む行は `lift/harness/`・`ci/`・`app/scripts/` の 0 件)。`Plugin database.nfo`(その上の 1 本)だけは seed に在る(382 行)。
- E5. 抽出は seed の行で決まる(`extract-fl.mjs`)。seed に無い 3 本は、bundle に入らない(推定: bundle の中身は未確認。抽出の仕組みから)。
- E6. 実物の `Plugin database (alphabetical)` の `.nfo` は、136 件すべて「大きさ − 名前の長さ = 11」(名前の長さは awk の `length`)。例: `Fruity Balance.nfo` 25 B(`.fst` 142 B・`.png` 18969 B)、`Fruity Filter.nfo` 24 B、`Sytrus.nfo`(Generators)17 B。製品の `Tip=` は名前 + 6 B で、形が違う。
- E7. #1508 の挿入の作業は main に入っていない(`0c650834` は main の祖先に無い)。main の位置のシナリオは挿入をしない(E8)。
- E8. 位置のシナリオは、窓の名前(`pluginInDesktop`)を探すだけ。挿入が無ければ窓は出ない。表示の宣言は `4k-dpr1`・`4k-random`(`4k-dpr2` は走らない)。
- E9. 黒い面(1355)は、クラウドのタブ(起動の時に選ばれている)の WebView2 の面。Plugin database の木の描画については、記録が無い。

## 4. 確認できないこと(実測ではない)

- 木が Installed の下に子を作ったかどうか(作ったが描かれない・作られない)。
- FL が `.fst`・`.png` を要るかどうか(記録の missing-path に、Installed の `.fst`・`.png` の探索は出ていない。ただし、読んだ・探したが無かった時に出るので、FL が探していない可能性もある)。
- `.nfo` の中身の形式(実物の中身は repo に無い。大きさの規則(E6)だけ分かる)。
- 一覧の行が、WebView2 の面の下で隠れているかどうか(PM の引き継ぎ: `put_IsVisible … isVisible=0`。repo では確かめられない)。
- 読む時機(非同期の列挙)かどうか(enum は 1 ページに 1 回。時間差の記録は無い)。
- 上の記録(missing-paths・crash-hunt)は #1484 より前のもの。現在の main で、分類データが今も探されて無いかは確認していない(再測定が要る)。
- `Presets\Plugin database\Installed\.Plugins.ini`(記録の上では探されて無い)の役割。

## 5. 候補の判定

| 候補 | 判定 | 根拠 |
|---|---|---|
| (a) 一覧の元データが無い | **部分的に退け、変種が残る**(分類データの欠落は記録で支持される) | Installed の 3 件は VFS に在る(enum-dir: 3)。ただし分類データ 3 本(E4)が、記録(missing-paths の 3 回の起動)で探されて無い。推定: 木は分類データを読んで Installed の子を付けるので、欠けると子が出ない。項目の形式(E6)も違う。`.Plugins.ini`(Installed の中の隠しファイル)も探されて無い(役割は未確認)。 |
| (b) WebView2 が被る | **確認できない** | 1355 の黒い面はクラウドのタブの面。Plugin database の木の描画の記録は無い。PM の引き継ぎの `isVisible=0` は repo で確かめられない。 |
| (c) E2E の測り方 | **確定** | E1・E2: 名前がログに出ないので、`databaseHas` は製品の状態に関係なく偽。146 本の「no entry」は、製品の証拠にならない。 |
| (d) 読む時機 | **確認できない** | 時間差の記録が無い。 |

## 6. 起票の方針

- Issue(下書き 249): 題名「Plugin database の Installed に、プラグインの行が出ない: 分類データ(`Data\Patches\Plugin database\*.nfo`)を VFS に入れ、E2E の『行がある』判定を画素の読みに直す」。priority Urgent、size L、`blocked_by: []`、マイルストーン・parent(012)・labels は #1484 の下書きと同じ。
- 最初の段は**原因の特定**(この job で、現在の bundle に対して): 画面の保存と画像の直接の読み(`read`。OCR は使わない)で、Installed の展開の矢印と子の行を見る。エンジンのログの `missing-path` を集める。判定は (a)・(b)・(c)・(d) のどれかを、実測で決める。
- 直す段: (a) なら分類データ 3 本を seed の行に足す(sha256 と大きさは `inputs/fl-installer-files.tsv`)。項目の形式は、実物(136 件の規則)に合わせる案を比べ、`.fst`・`.png` の要否を `missing-path` の行で決める。E2E は、`databaseHas` を画素の読み(展開の前後で一覧の画素が変わる)に直し、新しい smoke のシナリオ(`plugin-database-rows-drawn`)を足す。
- 受け入れの扱い: 依頼者の指示の受け入れ(`position-effects-fruity-balance-fruity-balance-x64` の ok)は、main では挿入が無いため**達成できない**。Issue の受け入れは、新しい smoke のシナリオ(一覧の行が描かれる)に置く。位置のシナリオの印の外しは、#1508 の挿入が入った後に行う(Issue 本文に書く)。
- 関係: #1508(plugins の E2E。`tools/e2e/lib/plugins.mjs` を触る。衝突しうる。main に未マージ)。#1484(閉じている)。#1524(`tools/plugin-run`。ファイルは重ならない)。

## 7. 本文に書かない物(制約の確認)

- 禁止の数値・台帳 ID・URL・ベンダー名・Cloudflare の識別子は、Issue 本文に書かない。`Plugin database.nfo` の大きさ(禁止の数値と重なるので、本文では書かない)。
