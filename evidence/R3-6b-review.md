reviewer の検査の報告(2026-11-16、対象 adf1f54)。判定: 差し戻し。判定語は reviewer の報告のまま
注: 原文のうち、ホームの絶対パスの表記を 1 箇所だけ省いて保存した(項目 1)。ほかは原文のまま。

判定: 差し戻し。数字と判断は一致(§3 の 12 行は report.json と全項目一致、等式 12/12 一致、判断は 0378 の規則 (4)→(3)→(1)→(2) どおりで (2)「再現せず」)。文面に禁止事項 1 件、出典の無い断定 5 件、未確認の書き方の過不足。git show --stat adf1f54 → 2 files changed, 129 insertions(+)(0378 +7、R3-6b.md +122)。ROADMAP・コード・他の記録は変更なし。

不一致の一覧:

1. 禁止: R3-6b.md L35 にホームの絶対パス(原文の表記は、この保存で省いた)。0378 は ~ で書いている。
2. L112「版は未確認」: 各 run の netlog-sdrive.json のリクエストヘッダが HeadlessChrome/156.0.0.0(確認した 14 フォルダすべて)。記録は同じ出典を見ていない。
3. §3 の「配ったページ」列の「7 件」は文字列の出現数。行数では 6(1 行に URL が 2 回入る)。数え方が書かれていない。
4. 出典の無い断定: U1 L1「P3 は音切れが残る」/ U4 L37「失敗して出力は無く、副作用は無い」(前置したコマンドの文字列・終了コード・後続の実行の有無が無い。2>/dev/null で出力が消えているので「出力は無い」は根拠にならない)/ U5 L37「測定の run は 1 回だけ行った」(出力フォルダは r36-r36arma-chrome-1〜6 の連番のみで余分な測定の出力は無い=支持材料)/ U6 L97「0260 の取りこぼしが腕 A で起きる条件が再現しなかった」(0 件から言えるのは『起きなかった』まで。条件は測っていない)/ U7 L116「main の app/dist は配っていない」(netlog の名前は腕 A と main で同一。R3-6a §3 が区別不能と書いている)/ 弱い U3 L35「foreground、timeout 1800000」(実行の記録が無い)。
5. §0 の「写し」(『変えていない』)は 0378 から省略あり: (3) の『腕 B の値によらない』、(1) の『除去は取り込まない』、(4) の『取りこぼしの数は参考として書く』。規則の中身は変わっていない。
6. §4 の出典 node $TMPDIR/r36b-table.mjs: reviewer は道具を確認できず($TMPDIR 未設定、/tmp/opencode に無い)。B の行は『…同じ…』と省略。
7. 観察(記録に無い): 腕 B chrome-3 の report.json だけ ioStart: 6.97(他は 0)。0378 の数えない回の定義には当たらない。記録は触れていない。

範囲の逸脱: 無し。腕 B(1fc537e)は pointer.ts 1 挿入・20 削除、テスト 54 削除で focusout 復帰の除去だけ。腕 A(20224a6)は tools の 2 ファイル(73 行)のみ。main に除去は入っていない(pointer.ts 78〜122 行の focusout 復帰は残る)。観察: 腕 A の基点 813bc1f から main まで app/ の差は 2 テストファイルのみ(84bfd2e、4 行の差し替え)。app/src は同一。0378 の『main と同じ commit』は測定時点で文字どおりではない。out/ に R3-6a の道具確認の 2 フォルダ(r36shakea・r36shakeb)が残るが記録の 12 行には入っていない(正しく除外)。

代役の混入: 0 件。注: 等式 popupSeen + notVisible + popupMissingSurface = completed は、停止が無い限り道具の構造上必ず成り立つ。整合性の確認にはなるが、実測の裏付けとしては弱い。

実エンジンの実行: 12 フォルダの report.json に timeline(wasm_instantiated・engine_start・main_start・first_pixel)、windowsEnd は TFruityLoopsMainForm@0,0 1920x1080、state=engine-idle、stopped=false、opErrors=[]。集計: A・B とも seconds 合計 1282、popupSeen 600、notVisible 0、popupMissingSurface 0、n 600(記録と一致)。

再現できなかった数字(未確認): rc=0(12 回)は report.json に無い/§2 の free -m・/proc/loadavg・ps は保存されていない/§6 の再計算の出力(0.015847913…・1.4657027…e-09・0.015696005…)は reviewer が python を実行できず、0378 の値と丸め桁で一致(文書上)/§4 の集計道具の所在を確認できない(数字は report.json から独立に再計算して一致)/手順逸脱の前置コマンドは文字列が記録に無い/L112 の『ls で見えず』は reviewer の ls でも /usr/local/share/playwright が無く記録どおり。

差し戻しの指示案(7 項目):

(1) L35 のパスを ~ に。確かめ grep -n "/home/" evidence/R3-6b.md が 0 件。
(2) L112 を『netlog-sdrive.json の User-Agent が 12 回とも HeadlessChrome/156.0.0.0。実行ファイルの版の直接の確認は未確認』に。
(3) 『配ったページ』列に『7 件 = index-<hash>.js の出現数(行数では 6)』を足す。
(4) U1・U4・U5・U6・U7・U3 を直す(下のやること 4)。
(5) §4 の出典: 集計道具をリポジトリ内に置くか、所在と B の値を省略せずに書く。
(6) §0 は 0378 の該当文を省略せずに写すか『要点の写し』と書く。
(7) §1 に腕 A の基点 813bc1f と main の app/ の差(2 テストファイルのみ)を 1 行足す。

禁止語の確認: github.com を含むワークツリーの絶対パス表記(L26・L27・L120)と /usr/local/share/playwright(L112)は、ホスト名・ベンダー名に当たるかを main が決める。
