# wine での煙テストの記録（仕様の確認ではない）

**これは wine の煙テストであり、実機 Windows の値ではない。ここに出る値（OS 版、ロケール、API の戻り値）は wine の実装・このマシンの設定の値で、A2b の「仕様確認」には使えない。** 実機 Windows での実行は**未確認**（まだ誰も実行していない）。MSVC でのビルドも**未確認**。観測ファイル（`observed.tsv`）は wine のものなのでリポジトリに置いていない。

作業日: 2026-10-28。リポジトリ: ブランチ main、HEAD e071fc0（作業木は未コミットの追加のみ）。

## 環境（実測）

- wine 10.0（Ubuntu `10.0~repack-12ubuntu1`）、`x86_64-w64-mingw32-gcc` 13-win32。
- wine のプレフィックスは `/tmp/opencode/winrep/wp`（RAM 上）。**作業中に、プレフィックス作成で一時的に約 1.2 GB を使ってしまい、依頼の上限（200 MB）を超えた**（wineboot が組み込み DLL の実体コピーを作るため）。気付いてすぐ `syswow64`・`winsxs`・コピーされた DLL を消し、組み込み DLL へのシンボリックリンクに置き換えて約 32 MB にした。この間、他の作業者の領域（/tmp の quota）に影響した可能性がある（未確認）。
- wine が答えた OS は `RtlGetVersion = 10.0.19043`、ACP/OEMCP = 932（このマシンのロケールから。日本語ロケールの wine ＝読み）。

## 実測した結果

| 項目 | 結果 |
|---|---|
| ビルド | `x86_64-w64-mingw32-gcc -O2 -Wall -Wextra -Wno-unused-parameter -Wno-cast-function-type replay.c`: 警告・エラーなし。`-std=c99 -pedantic -fsyntax-only` も通る。`clang --target=x86_64-w64-mingw32 -isystem /usr/x86_64-w64-mingw32/include -fsyntax-only -Wall -Wextra` も通る。**MSVC の `cl` は無いので未確認**（`clang --target=x86_64-pc-windows-msvc` は Windows SDK が無く `windows.h` が見つからず検査できなかった） |
| 構造体配置の静的検査 | `layout_check.c`（76 個の sizeof/offsetof の `_Static_assert`）がコンパイルできた。**作る途中で 2 件の誤りを見つけた**: LOGFONTW の 25 バイト目は `lfClipPrecision`（`lfQuality` は 26 バイト目）、`sizeof(CPINFO)` は 20。トレースの decoder（`fleanalyze.py`）は 25 バイト目を `quality` と表示していた（トレース側の誤表示。`compare.py` の `TRACE_ALIAS` で吸収し、`replay.c` は `clip=`/`quality=` を正しい位置から出す） |
| ホワイトリスト | `--list-whitelist`: 174 種（トレースの 153 種＋補助 21 種）。`gen_cases.py --whitelist` で、ケースが使う API はすべてホワイトリストにあり、ホワイトリストにあって使われない API は無いことを確認 |
| 完走 | ケース 2,144 件、約 1〜2 秒（2 回測って 0.9 秒と 2.2 秒）、終了コード 0。`status`: ok 2,126、skip:policy 18（RegCloseKey 14＝wine にそのキーが無くて開けなかった、FreeLibrary 1＝2 回目、GetModuleHandle* 2＝許可リスト外の名前の拒否確認、DeleteObject 1＝ストックオブジェクトの拒否確認）。クラッシュ 0 |
| 同一性 | 同じ入力で 2 回走らせた出力は、ポインタ・ハンドルの値を除いて同じ（`compare.py --observed-manifest` で「差のあるケース無し」） |
| クラッシュからの再開 | 故意に壊した 1 件（`CharLowerBuffW` に不正ポインタ）で `crash:0xc0000005` の行が書かれ、終了コード 3。`--resume` で次のケースから最後まで続けられた |
| マスク | 作業ディレクトリ名をユーザー名と同じにして実行し、`GetModuleFileNameW`/`GetCommandLineW` の出力が `<USER>` に置換され、その名前が出力に残らないことを確認 |
| `--dpi-aware` `--no-mask` | 付けて完走（2,144 行） |
| compare.py | 動く。wine の値との突き合わせ結果は match 371／mismatch 150／reference 47／real-only 1,558／skipped 18。**mismatch は wine と runtime の違いであって、実機の結果ではない**ので記録しない |

## 実施していない／確認できていない

- 実機 Windows での実行、MSVC (`cl`) でのビルド（`/W3` で出る警告も未確認）、Hyper-V の Windows VM での動作、Windows 上のエージェントが指示書どおりに動くか。
- `docs/windows-delegation/issue-0001-api-replay.md` の PowerShell コマンドは、PowerShell 7 の構文解析でエラーが無いことだけ確認した（Windows の cmdlet は実行していない）。
- 実機では wine と違い、レジストリ・ロケール・システム DLL（`avrt`, `powrprof`, `shcore` など）が実在するため、`skip` や `nomodule` の出方は変わる。
