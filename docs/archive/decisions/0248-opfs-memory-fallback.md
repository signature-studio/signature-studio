# 0248: ブラウザがページに OPFS を使わせないとき、ファイルマネージャーの木をタブのメモリに置く(能力検出は `detectRoot` の 1 か所)

状態: **仮の決定**。依頼は PM(Firefox 159 の private 窓などで OPFS が使えないときのページの挙動)。実装・測定・文書は作業者 `ci1`(2026-11-05 JST)。基準 HEAD 2eea92b1。差分: `lift/out-git/ci1-results/ff-opfs.diff`(HEAD に `git apply --check` で当たる。app/src の 3 ファイル+新規 1、app/test/unit の新規 1、app/test/mutation/run.mjs)。旗・URL パラメータ・設定・環境変数は無い(自動の能力検出だけ。`docs/policies/no-options.md`)。依頼者に頼むことは何も無い(`docs/policies/owner-contact.md`)。

区分の凡例は決定 0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コード・文書を読んだ解釈。**出典なしの選択**＝§9。**未確認**＝確かめていない。環境は『この機械』= 他担当の負荷が常にある Linux VM(load 5〜20)。依頼者の機械・実 GPU・実機 Windows の値は 0 件。

## 冒頭の三点

1. **何を実測したか**: 実 Firefox 158(pref `browser.privatebrowsing.autostart` の private 窓)で、現行ページ(HEAD 2eea92b1)と修正版を、本番 bundle 5c4b6902 の手元再構築(CI とバイト一致)を載せて対で動かした。現行は Files ボタンが無効になり File>Open はエンジンへ「キャンセル」を返すだけで、ファイルを開く手段が無い。修正版は Files 窓が開き、Sampler.flp を File>Open で開き、名前を付けて保存した物が Files 窓に出て、参照プロジェクトの zip(53.5 MB)を Files に入れて File>Open で zip を二度押しすると、ページが解凍(49 ファイル 62.7 MB)してエンジンが 7.2 秒後に開く。通常窓では OPFS が 16 回中 16 回選ばれ、判定の費用は初回の 1 回だけ(Chrome 中央値 +10 ms、Firefox +74 ms)。単体テスト 21 件、mutation 17/17 検出(公式 runner、lift/ 込みの HEAD)。
2. **結論**: 直す。判定は `detectRoot()` の 1 か所(実 OPFS で 1 バイトを書いて読んで消せなければ、メモリ上の木)。OpfsFs とその上(Explorer・ops・engine-dialog・zip 解凍)は OPFS 版と同じコードで、違うのは『ハンドル』だけ(経路は 1 本)。利用者に見える変更は、保存行の 1 文と窓の副題だけ。
3. **未確認**: 依頼者の Firefox 159・実 Windows・実 GPU は 0 件。実ネットワーク越しの起動時間(private では起動資産の stored copy が使えず、1 回の読み込みで brotli 30.6 MB を取り直す。loopback では差が出ない)。Firefox の通常窓でプロファイルのディスクが小さいとき(先の `/tmp` の『Could not create actor』)にメモリ版へ落ちることは単体テストの模擬でだけ確認(実機では先の再現環境に戻して見ていない)。Chrome のシークレット(OPFS が使えるので対象外)。1 GB 級のプロジェクトのメモリ。

## 1. 背景
- Firefox の private 窓では `navigator.storage.getDirectory` は関数として存在するが、呼ぶと `SecurityError: Security error when calling GetDirectory` で失敗する(**実測**、Firefox 158)。通常窓は成功する。geckodriver の `-private` 引数だけでは窓は private にならず、pref `browser.privatebrowsing.autostart=true` が要る。
- ci1 の報告3の、プロファイルが小さい tmpfs にあるときの `Could not create actor`(OPFS が getDirectory の後で失敗する)も、同じ『OPFS が使えない』の一つ。

## 2. OPFS を使う箇所と、失敗したときの挙動(**読み**+実測)
| 箇所 | 用途 | 失敗時(修正前) | 利用者に見えるか | 機能の欠落 |
|---|---|---|---|---|
| bundle/assetstore.ts(loader.ts が呼ぶ) | 起動資産の stored copy | loader が受けて `stored copy: failed (code)`、ネットワーク/HTTP キャッシュから読む | ログ・性能パネル・beacon | なし(取り直しで遅い) |
| files/mount.ts | Files 窓 | ボタン無効、title に理由 | ツールチップのみ | **あり** |
| files/engine-dialog.ts select | File>Open・名前を付けて保存・フォルダ選択 | エンジンへ `cancel(...)` | 何も出ない | **あり** |
| files/engine-dialog.ts onSavedFile | 保存の書き戻し | warn のログ、ダウンロード一覧に残る | ログ | 縮退で動く |
| files/explorer.ts storageInfo・persist・容量の検査 | 使用量・永続化・入れる前の容量検査 | try/catch で unknown。**容量検査は OPFS の quota で行う** | 「unknown」 | なし(§3 の修正で検査を OPFS のときだけにした) |
| files/ops.ts・zip-read.ts・select.ts | OpfsFs 経由のみ | 上に従属 | — | 上に従属 |
`ui/filedlg.ts` には `<input type=file>` の旧い箱(buildOpen/buildSave)が残るが、main.ts が常に `select` を渡すので使われない。

## 3. 設計
- `files/opfs.ts` の `detectRoot(nav)`: `navigator.storage.getDirectory` が無い → 'unsupported'。呼べても、root で『名前を持つ一時ファイルを作り、1 バイト書き、閉じ、読み戻して大きさ 1 を確かめ、消す』のどれかが失敗 → 失敗(SecurityError・NotAllowedError は 'blocked'、他は `failed: <名前>: <メッセージ>`)。失敗なら `files/memory.ts` の木。一時ファイルの名前はページごとに乱数(別タブが同時に判定しても、Firefox の『1 ファイルに書き手は 1 つ』で互いを落とさない)。
- `storageRoot()` はページごとに 1 回決めて保持する。Files 窓とエンジンのダイアログは `OpfsFs.open()` で同じ木を見る(メモリ版では必須)。
- `files/memory.ts`: OpfsFs が使う `FileSystemDirectoryHandle` の部分(getFileHandle・getDirectoryHandle・removeEntry・entries、ファイルの getFile・createWritable の write/close/abort)だけを実装。OPFS に合わせた点: 失敗の DOMException 名(NotFoundError・TypeMismatchError・InvalidModificationError・NoModificationAllowedError)、書いた内容は close で初めて見える(abort で元のまま)、1 ファイルに書き手は 1 つ、消したファイルのハンドルは使えない、書いた物は複製を持つ。`move` は持たない(OpfsFs は『コピー+削除』に落ちる)。
- 経路は 1 本: OpfsFs・Explorer・ops・engine-dialog・zip 解凍にメモリ専用の分岐は無い。違いは『ハンドル』だけ(OPFS 版のコードは変えていない)。
- 利用者に見える変更: ①メモリ版のとき Files 窓の保存行が「These files are kept in this tab's memory only: this browser does not give the page file storage (a private window, or site data blocked). They are gone when the tab is closed or reloaded.」(理由ごとに文が変わる。`memoryNotice`)。persist の問い合わせ・使用量の表示は出さない。②窓の副題の『(OPFS)』が『(this tab's memory)』/『kept in this tab's memory』。③エンジンのダイアログを初めて出すときページのログに 1 行(warn)。④保存の書き戻しのログの行き先の語。
- **quota の扱い**: 入れる前の容量検査(`explorer.ts` の 2 か所)は `navigator.storage.estimate()` の quota と比べていた。メモリ版のときこの quota は無関係で、先の tmpfs の例(quota 389 MB)のように小さいと、メモリ行きの物まで断ってしまう。`roomLeft(fs, info)` に出し、メモリ版では『制限なし』(null)にした。

## 4. 変更
| ファイル | 内容 |
|---|---|
| app/src/files/memory.ts(新規、約 120 行) | メモリ上の木 |
| app/src/files/opfs.ts | `detectRoot`・`storageRoot`・`memoryNotice`・`roomLeft`、`OpfsFs.open` が `storageRoot` を使う、`OpfsFs` に `persistent` と `why` |
| app/src/files/explorer.ts | 保存行(メモリ版の文)、副題、容量検査 2 か所を `roomLeft` に |
| app/src/files/engine-dialog.ts | メモリ版のとき 1 回だけログ、保存の語 |
| app/test/unit/files-memory.test.ts(新規) | 21 件 |
| app/test/mutation/run.mjs | `files-memory-*` 17 本 |
触らない物: beacon の形式、worker、scripts、ci、assetstore(起動資産の stored copy は OPFS でなくなっても要らない)。

## 5. テスト(`tables/RESULTS.md` に値)
- 単体 21 件: メモリ木が OPFS と同じ失敗名・同じ順序規則・複製を返す、その上の OpfsFs の操作(書く・読む・一覧・stat・コピー・移動・削除・du・名前の重複)、`detectRoot` の分岐(getDirectory 無し・SecurityError・書けない・大きさが違う・正常・同時の 2 ページ)、2 つの入口が同じ木を見る、通知文、`roomLeft`。
- mutation 17 本(公式 runner、`git archive` の HEAD に差分を当てた lift/ 込みの木、APP_WORK はディスク): **17/17 検出**、基準の単体 1,273 通過、戻した後の木は同一。
- e2e(Files の場面、実 Firefox 158 と Chrome 156、直接 OPFS を読むハーネスの 10 か所だけ『取れなければページの root』に差し替え): Firefox private 101/103(落ちる 2 件は persist と estimate の計数で、メモリ版には無い)、Firefox 通常 55/56(落ちるのは `persist()` の直後の Firefox の許可の問いによる 30 秒の script timeout。現行ページでも同じ所で同じ)、Chrome 通常 105/105。
- エンジンを載せた実測は『冒頭の三点』と RESULTS §2〜§5。

## 6. 結果(RESULTS.md のとおり)
- 現行→修正版(Firefox 158 private、エンジン込み): Files 窓 なし→開く、File>Open キャンセル→窓が開く、Sampler.flp が開く、名前を付けて保存→Files 窓に出る(111,172 B)、参照 zip が 7.2 秒で開く(通常窓の OPFS は 16.4 秒)、first idle は差なし(4.5〜4.8 秒)、JS エラー 0。
- メモリ: 116.2 MB(zip 53.5+解凍 62.7)を持つとタブのプロセスが +64〜+120 MB。
- 通常窓: OPFS を 16/16 選ぶ。初回の判定の費用 Chrome +10 ms、Firefox +74 ms(1 ページ 1 回、起動時ではない)。

## 7. 制限・既知の差
- メモリ版はタブを閉じる・再読み込みで消える(通知に書いた)。OPFS 版の『Not protected』『Ask the browser to keep it』は出ない。
- 判定の結果はそのページの間変わらない(途中で OPFS が直っても戻らない。木が二重にならないため)。
- 上限が無い。大きなプロジェクトはタブのメモリを使う(未測定: 1 GB)。
- 判定の一時ファイルをブラウザが書いている最中にタブを閉じると、OPFS の root に `.fl-probe-…`(1 バイト)が残りうる(次の判定は別名なので誤動作はしない。Files 窓の一覧には見える)。
- 通常窓で OPFS が部分的にしか壊れていない(判定は通るが、後の大きな書き込みで quota が尽きる)場合は従来どおりエラーを出す(メモリ版へは動的に切り替えない。出典なしの選択 S-ci1-opfs-sticky)。

## 8. 未確認
冒頭の三点の 3 のとおり。加えて: Firefox の private 窓で OPFS を除く保存先(IndexedDB など)を使う案は取らなかった(OPFS と同じ API の木で足りる)。

## 9. 出典なしの選択
| ID | 選んだ値 | 根拠 |
|---|---|---|
| S-ci1-opfs-probe | 判定は『名前を持つ一時ファイルを作って 1 バイト書いて閉じて読んで消す』 | getDirectory が成功してから書けない型(先の tmpfs の例)を拾うため。root を見るだけでは拾えない |
| S-ci1-opfs-probe-name | `.fl-probe-<8 文字の乱数>` | 同時に判定する別タブを互いに落とさない(Firefox は 1 ファイルに書き手 1 つ) |
| S-ci1-opfs-sticky | 判定はページ単位で 1 回、変えない | Files 窓とエンジンのダイアログが別の木を見ない |
| S-ci1-opfs-notice | 保存行の文面(英語、理由ごと) | 窓の他の文面が英語。『消える』ことを必ず書く |
| S-ci1-opfs-nolimit | メモリ版に上限を置かない | 上限の値に根拠が無い。OPFS 版の quota は別の物 |

## README の行(`docs/README.md` の決定の表。列は番号・ファイル・要約・初コミット・仮)
`| 0248 | [0248-opfs-memory-fallback.md](decisions/0248-opfs-memory-fallback.md) | **ブラウザがページに OPFS を使わせないとき(Firefox の private 窓は getDirectory が SecurityError)、Files 窓・エンジンの File>Open/名前を付けて保存が全部使えなかった。能力検出を detectRoot の 1 か所(実 OPFS で 1 バイトを書いて読んで消せなければ、メモリ上の木)にし、OpfsFs とその上は同じコードのまま、違いは『ハンドル』だけ(経路 1 本、旗・URL・設定なし)。実 Firefox 158 の private で現行と修正版を対で比べた。依頼者の機械・実機 Windows 0 件。** 現行は Files ボタン無効・File>Open はキャンセル。修正版は Sampler.flp が開き、名前を付けて保存した 111,172 B が Files 窓に出て、参照 zip(53.5 MB)を二度押しで 49 ファイル 62.7 MB 解凍して 7.2 秒で開く(通常窓の OPFS は 16.4 秒)。通常窓は OPFS を 16/16 選び、判定の費用は 1 ページ 1 回 Chrome +10 ms・Firefox +74 ms。メモリは 116 MB を持つとタブ +64〜+120 MB。単体 21 件、mutation 17/17 検出、Firefox private の e2e 101/103(落ちる 2 件は persist/estimate の計数)。**実ネットワークでの起動、Firefox 159、1 GB 級は未確認。** | 11-05 | **仮** |`

## PM の決定と取り込み(2026-11-05)

取り込んだ基準: HEAD a6331e92 に `ff-opfs.diff`(6 ファイル:`app/src/files/{engine-dialog,explorer,opfs,memory}.ts`、`app/test/unit/files-memory.test.ts`、`app/test/mutation/run.mjs`)を `patch -p1`(offset・fuzz・reject なし、`-` 行は import・OPFS 前提の文言・storageInfo の呼び出しの置き換えだけで全部読んだ)。PM が測った値(この機械、`lift/out-pmchk-ff/`): typecheck rc=0、vitest 65 ファイル通過 / 1 skip、1,274 passed + 7 skipped(基準 1,253 + 21 の新規)、worker 49/49、e2e(エンジン無し)172/172。

1. **採る。** 判定は `detectRoot()` の 1 か所(実 OPFS で 1 バイトを書いて読んで消せるか)で、人も設定も選ばない。以後の層(OpfsFs・Explorer・ops・engine-dialog・zip 解凍)は同じコードで、経路は 1 本。依頼者に頼むことは無い。
2. 費用: `detectRoot` は Chrome の中央値 11.5 ms(現行 1.1 ms)、Firefox 75.0 ms(現行 1.3 ms)で、起動の critical path 上の 10 ms 台〜70 ms 台。P1 に効く量ではないが、Files 窓を開くまで待てる処理なら遅延させる余地がある(未着手)。メモリ版の RSS は参照 zip で +64〜+120 MB。
3. 未確認(ci1 の報告のまま): 依頼者の Firefox 159・実 Windows、実ネットワーク越しの private 窓の起動時間(stored copy が使えず 1 回の読み込みで取り直す)、1 GB 級のプロジェクトのメモリ。
