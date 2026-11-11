# 決定 0260: File メニュー ×100 の約 0.7% の取りこぼしは、ブラウザがキーボードフォーカスを `#desktop` からページの別ボタンへ勝手に移すことが原因で、ページ側で戻す

状態: **仮の決定**。作業者 `ci1`(0260 担当)、2026-11-05 22:00〜2026-11-06 00:20。PM の判断と取り込み 2026-11-06 00:30。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=コード・文書・測定の解釈、**未確認**。この機械(16 vCPU の VM、実 GPU なし、headless Chrome 156、load は他の作業と共有で 11〜36)の値で、**依頼者の機械の値は 0 件**。報告の全文は `docs/investigations/ci1/0260/REPORT-0260-{1,2,3,3b}-to-PM.md`。

## 0. 先頭の 3 点

1. **測ったこと(実ブラウザ Chrome 156 + 実エンジン)**: File メニューを開いて Escape で閉じる周期を 1 セッション 60 周期で繰り返し、取りこぼし(ポップアップが 6 s 以内に見えない)を数えた。ページを変えない腕(A+B+C)は **46 セッション・2,760 周期で 19 件(0.69%)**。この 19 件の**全件**で、直前の周期の Escape がエンジン内の `fle_input_host_key` に届いていなかった(記録器の `REC injk` が無い)。ページ側の記録を付けた 16 件では、Escape を送った時点の `document.activeElement` が `#desktop` でなく **`panel-toggle`(ページの "Monitor" ボタン)**で、`focusout desktop → relatedTarget=panel-toggle` は**スクリプトの `focus()` もポインタ押下も Tab も無く**起きた(`document.hasFocus()` は真のまま)。CDP の `Input.dispatchKeyEvent` は全回成功、ドライバの `dropped` は 0。ランタイムの入力の時刻・順序(同じバッチの DOWN/UP、`fle_input_synth` の再入、ダブルクリック化)は約 1,000 回の注入で**観測されなかった**(`NESTED-synth` 0 件)。
2. **直したこと**: `app/src/input/pointer.ts` に、`#desktop` の `focusout` で、ユーザーの操作(直近 1 s のその行き先へのポインタ押下、または Tab)が無く、行き先がテキスト入力でなければ、次のタスクで `#desktop` にフォーカスを戻す処理を足した(旗・URL パラメータ・環境変数なし。`ci1-page-focus.diff`)。**修正した腕は 24 セッション・1,440 周期で取りこぼし 0 件**(割合が同じなら 9.9 件の期待、0 件の確率は約 0.005%)。同じ腕でブラウザがフォーカスを `panel-toggle` へ移した 26 回は全て戻され、Escape は全て届いた。実ブラウザの決定的な確認(最後の `#desktop` クリックの 0.4 s 後に `panel-toggle.focus()` または `activeElement.blur()`)も、修正したページでは `#desktop` に戻る。
3. **未確認**: ブラウザがフォーカスを移す**引き金は未特定**(移動は最後のポインタ上げの 82〜1,535 ms 後、周期内の決まった位相ではない。直前 1.5 s の DOM 変更は状態表示の定期更新だけ)。headless Chrome の挙動と読むが推測。実機の Firefox / 通常の Chrome で起きるか、依頼者の機械で起きるかは 0 件。ランタイムの再入ガード(`ci1-input.diff` v1、`ci1-input-v2.diff`)は本件の原因ではなく、v1 は非局所脱出で入力が止まる欠陥があるため**採らない**(v2 は保管、条件つき)。

## PM の決定と取り込み(2026-11-06 00:30)

取り込み: ci1 の報告 1・2・3・3b、差分、C テストの出力、記録器の行、道具を `docs/investigations/ci1/0260/` に置いた。`ci1-recorder-NOT-FOR-PRODUCT-*.diff` はテスト専用で製品の木に当てない。`ci1-page-focus.diff`(更新版、sha256 先頭 `e000e626e5bff67b`、2 ファイル: `app/src/input/pointer.ts`、`app/test/unit/pointer-dom.test.ts`)を HEAD 91cbd9b0 の `git archive` に `patch --batch -p1 --dry-run`(offset・fuzz なし)で確かめて PM の木に当てた。`-` 行は 1 本(`pointer.ts` の先頭コメントの 1 行を、続きの項目つきに書き換える)を読んだ。FL のクラス名・窓名・アドレスの分岐は無く、新しいオプションも無い。

PM が測った値(この機械、`app/`、`lift/out-pmchk-pf/`): `npm run typecheck` rc=0; `npx vitest run` rc=0 — **66 files passed + 1 skipped、1,291 passed + 7 skipped**(変更前 1,290); `npm run test:worker` rc=0 — 49 passed; `npm run test:e2e` rc=0 — **190/190 checks passed**。

判断:
1. **採る(ページ側の修正)。** 修正した腕の 0/1,440 と無修正の 19/2,760 の差は偶然では説明できない(P≈0.005%)。「ユーザーの操作が無いのにフォーカスが別のコントロールへ移ったら戻す」はページの入力の契約として自然で、`#desktop` のキー入力が失われる欠陥を直す。依頼者の機械でも、ブラウザが同じようにフォーカスを移せば同じ欠陥が起きうる(未確認)ので、実害を直す変更。app だけの変更で、engine の bundle は変わらない。
2. **ランタイムの再入ガード(`ci1-input*.diff`)は入れない。** 実エンジンで観測された不具合を直さない(`NESTED-synth` 0 件)変更は足さない。v1 は `translating` が非局所脱出で戻らない欠陥があり、v2(スレッドごとの印と深さ)は保管する。取り込むなら Windows の入力順序の正しさの改善として別の決定にし、`translate_one` からの全ての脱出経路で減算されることの表を付ける。
3. **副作用の点検**(遅れて開くダイアログが script でボタンへフォーカスを移す形が、1 s の免責の外で奪い返されないか)は ci1 に依頼した。結果は ci1 の最終に入れる。
4. テストツールの欠陥(`waitWindow` の戻り値を無視して Escape が効かなくても次の周期へ進む)は、ci1 が次版で直す。

## 副作用の点検(ci1 の報告 4、2026-11-06 00:55)

`app/src` の `.focus()` は 14 箇所(`autofocus`・`showModal`・`showPopover` は 0、`tabindex` は `#desktop` だけ)。ci1 が全数を表にした(`docs/investigations/ci1/0260/`)。**奪い返される形は 1 つ確認**: ページの Files ボタンを**ポインタ押下なしで**(プログラムの `.click()`、desktop が 1 s を超えてフォーカスを持っている状態)開くと、`files/mount.ts:50` の `#fx-list.focus()` が desktop に奪い返される(実ブラウザで確認)。本物のマウス押下・キーボード(Tab → Enter)では起きない(ボタンが先にフォーカスを取り、押下の時点で免責される)。ファイル選択・保存の代行ダイアログ(`ui/filedlg.ts`)と `#fx-filename` は行き先が INPUT なので影響なし。**支援技術・拡張による『ポインタも Tab も無い』移動は、1 s 以内の押下・Tab が無ければ desktop に戻される**(意図した動作との衝突、スクリーンリーダーの仮想カーソルなど)。判断(PM): 今回は直さない — エンジンのキー入力(Escape など)を失う欠陥のほうが実害が大きく、本物の押下・キーボードの経路は影響を受けない。ci1 の案(`byUser` を『直近 1 s に `#desktop` の外へのポインタ押下か Tab』に単純化)は、支援技術の経路が実際に問題になった時点で再検討する(依頼者の機械・支援技術・Safari / macOS の Firefox は 0 件、未確認)。
