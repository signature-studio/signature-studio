## 8. lift の忠実性の方針(`docs/policies/lift-fidelity.md`、2026-11-04 に追加)との照合 — 自己点検

この方針は作業の後に追加された。差分を点検表(同 §2)で見直し、足した物をすべて分類した。**判断は PM。** 分類は私の読み。

| 足した物 | 点検表の問い | 分類 |
|---|---|---|
| `fle_clip.c`(Win32 クリップボード 8 API、OLE クリップボード 3 API) | 外部 API の契約の実装か → はい | (b) API の拡充。ただし合成形式・遅延描画・形式の公開の細部は出典なしの選択(§5 S-ia1-clip-*)で PROVISIONAL |
| `fle_timerq.c`(CreateTimerQueueTimer / Change / Delete)、`fle_thread.c` の `fle_thread_spawn` | 同上 | (b)。タイマーごとに協調スレッド 1 本、は実装の選択(S-ia1-tq-1) |
| `fle_dwm.c`(DwmIsCompositionEnabled = TRUE、DwmExtendFrameIntoClientArea = 状態のみ) | 同上 | (b)。TRUE は Learn(Win8 互換の manifest なら TRUE)と FL64.exe の manifest に基づく環境の答え。**状態のみ**の部分は契約の一部(枠の拡張の効果)が未実装 = (b) の不足が残る |
| `fle_shell.c`(PathCleanupSpec)、`fle_input.c`(SetKeyboardState)、`fle_locale.c`(CompareStringW の数字/文字) | 同上 | (b) |
| `fle_win.c`: システムキャレット、ShowOwnedPopups、LogicalToPhysicalPoint / PhysicalToLogicalPoint(恒等)、DefWindowProcW の 3 メッセージ、WH_CALLWNDPROC と CallNextHookEx、SetWindowRgn | 同上 | (b)。**不足が残る物**: キャレットは描かない(契約は描画と点滅)、SetWindowRgn は状態のみ(描画とヒットテストを切り抜かない)。恒等の座標変換は DPI の選択(S-ia1-dpi)に依存 |
| `fle_seh.c` の診断(遅延ロードの dll・関数名、登録の連鎖) | FL の観測(診断)だけか → はい | 製品の経路には入らない(停止の文に付くだけ。通常の経路では実行されない)が、点検表に厳密に従うならビルドの opt-in へ切り離す候補 |
| `fruity_g3.c` / `fruity.c` の `probe`・`importlog`・`setenv`(`-DFLE_G3_FORKPROBE`、native 専用)、`a4-multi.sh` / `gen_two.sh` / `build-engine-bundle.sh` の `EXTRA_IMPORTS` | FL の観測 / ビルドの opt-in | 製品の経路から切り離し済み(native 専用か、環境変数で渡したときだけ) |
| `ia1-imports*.txt`(行ファイル) | opt-in | 既定の生成 C は不変。**既定の入口に入れるかは未決(PM)**(MERGE-NOTES) |

- **FL のクラス名・窓の種類・位置・文言で分岐する処理: 0 件**(ランタイムの追加行を grep した。クラス名の文字列は無い)。ページ(`app/`)の変更は無い(差分の `app/` は `build-engine-bundle.sh` だけ)。窓の前面化・閉じる・配置・Z 順には介入していない。
- **余計なロジック(FL の代行・補正・上書き)は、この差分に入れていない。** 探索の途中で使った『soft』ビルド(行の無い import が名前を出して 0 を返す)は検証に使わず、差分にも入れていない。
- 記録の 17 件の壁は、すべて (b) の不足(行の無い API、契約の一部だけの実装、遅延ロードで引かれる export の欠落)だった。(a) の余計な処理が原因の壁は、この範囲では見つからなかった。
- **PROVISIONAL の一覧**は §5(`tables/s-choices.tsv`)。**期限は付けていない**(PM が付ける)。契約に照らして実装し直す候補の順: (1) キャレットの描画と点滅、(2) SetWindowRgn の切り抜き、(3) DwmExtendFrameIntoClientArea の効果、(4) クリップボードの形式、(5) W16 / W17 / U1(ShellExecuteW、GetPrivateProfileStringW、IFileOpenDialog の COM)。
- 性能(変換と通信の削減)には関与していない。ここでの速度の数値は主張しない。

