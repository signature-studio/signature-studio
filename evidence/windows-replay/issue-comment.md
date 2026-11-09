## Issue #1 実機 Windows での win-replay 測定 — 報告

測って記録しただけです。詳細は `worklog/1/notes.md`、環境は `worklog/1/env.md`。

### 1. 環境

| 項目 | 値 |
|---|---|
| OS | Microsoft Windows 11 Pro / Version 10.0.26300 / Build 26300.9711 / 64 ビット / DisplayVersion 26H2 |
| 機種 | Microsoft Corporation / Virtual Machine（Hyper-V） |
| ロケール | システム・書式・表示言語とも ja-JP（LCID 1041）。MUI: ja-JP。ユーザー言語リスト: ja |
| コードページ | ANSI 932 / OEM 932 |
| タイムゾーン | Tokyo Standard Time |
| DPI | `AppliedDPI`=96、GDI の DpiX/DpiY=96（DPI 非対応プロセスから見た値）。**拡大率は約 175% と推測**（`GetSystemMetrics` の画面サイズが、DPI 非対応の実行 A で 2194x1234、`--dpi-aware` の実行 C で 3840x2160）。設定アプリは見ていない |
| モニタ | `Screen.AllScreens` は 1 台（DISPLAY1、2194x1234、primary）。`Win32_VideoController` は Hyper-V ビデオ 1024x768 と Remote Display Adapter 3840x2160 の 2 件 |
| 接続方法 | **推測**: リモートセッション（拡張セッションか RDP か、区別できていない）。画面は見ていない |
| 更新 | 最新のホットフィックスは KB5127647 / KB5127648 / KB5128942（2026/10/29 適用）。全て適用済みかは不明 |
| コンパイラ | Visual Studio 2022 Community 17.14.44、`cl` 19.44.35229 (x64)（VM に既にあったもの。`vcvars64.bat` 経由。新規インストールなし） |
| その他 | Windows PowerShell 5.1、Python 3.14.10（VM に既にあったもの）。管理者権限なし |

### 2. 実行件数

ビルド: 警告・エラーなし（`build.log`）。`--list-whitelist` の最後は `# total=174 traced=153 helper=21`。`replay.c` は無改変。

| 実行 | cases | rows | ok | skip:policy | nomodule | noexport | crash | hang-or-killed | 終了コード |
|---|---|---|---|---|---|---|---|---|---|
| A（plain） | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| A'（repeat） | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| B（win10 manifest） | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| C（`--dpi-aware`、任意） | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |

- クラッシュ・ハングは無し。`--resume` は使っていない。所要は各 3〜7 秒。
- `skip:policy` の 17 件は 4 実行で同じ（`GetModuleHandleW/A`、`FreeLibrary`、`RegCloseKey` ×13、`DeleteObject`）。
- `RtlGetVersion` = 10.0.26300 で `env.md` と一致。
- A と A' の違い: Issue の `Compare-Object` で **184 行**（= 92 ケース）。ヒープ／ハンドル／ポインタ値と、時刻・PID・TID の類。
- 参考: A と B の違いは 108 ケース（上記に加えて `GetModuleFileNameW` ×9 と `GetVersion*` 系）。A と C は 154 ケース（`GetSystemMetrics` の画面・アイコン寸法など）。

### 3. 一致／不一致（`compare.py`、A と B を入力）

`verdicts: match=352, mismatch=161, reference=55, real-only=1559, skipped=17`（`not run: 0`）

`compare.py` に A' と C は渡していない（Issue の手順どおり）。

### 4. 不一致の上位 20（`compare-stdout.txt` そのまま）

```
 1. C02031 GetDIBColorTable x34 last error: runtime=unchanged(0)×34 real=set 6
 2. C02023 StretchDIBits x24 return: runtime=0x20 real=0x0; last error: runtime=unchanged(0)×24 real=set 87
 3. C00630 SetErrorMode x23 return: runtime=0x0/0x8000 real=0x8001
 4. C01364 GetFileAttributesW x20 last error: runtime=unchanged(3)×15,set 3×5 real=set 2
 5. C00901 GetVersionExW x10 output: OSVERSIONINFO.major: runtime=10 real=6; OSVERSIONINFO.minor: runtime=0 real=2
 6. C01460 RegOpenKeyExW x9 return: runtime=0x2 real=0x0
 7. C00900 GetVersion x8 return: runtime=0x4a65000a real=0x23f00206
 8. C01290 TlsAlloc x6 return: runtime=0x0/0x1/0x2/0x3/0x4/0x5 real=0x9
 9. C01468 RegOpenKeyExW x6 return: runtime=0x2 real=0x0
10. C01472 RegOpenKeyExW x6 return: runtime=0x2 real=0x0
11. C01359 GetFileAttributesW x5 return: runtime=0xffffffff real=0x20
12. C01377 GetFileAttributesW x5 last error: runtime=set 2×5 real=set 3
13. C00004 GetLocaleInfoW x4 output: out2: runtime="0" real="2"
14. C01132 GetProcAddress x4 return: runtime=0x0 real=OPAQUE
15. C01366 GetFileAttributesW x4 last error: runtime=unchanged(3)×3,set 3×1 real=set 2
16. C01367 GetFileAttributesW x4 return: runtime=0xffffffff real=0x20
17. C01369 GetFileAttributesW x4 last error: runtime=unchanged(3)×3,set 3×1 real=set 2
18. C01370 GetFileAttributesW x4 last error: runtime=unchanged(3)×3,set 3×1 real=set 2
19. C00905 VerSetConditionMask x3 return: runtime=0x18 real=OPAQUE
20. C00906 VerSetConditionMask x3 return: runtime=0x1b real=OPAQUE
```

注: 5 番と 7 番の `real` は、manifest なしの実行 A の値（`GetVersionExW` は 6.2 / build 9200、`GetVersion` = 0x23f00206）。manifest ありの実行 B では 10.0 / build 26300（`GetVersion` = 0x66bc000a）。どちらが正しいかは判断していない。

### 5. コミット

- ブランチ: `worklog-issue-1`（push 済み。`main` には触れていない）
- 1 回目のコミット: `b143e059820be84390b84a3d7e75dab360da2a46`（`worklog: Windows API replay results (issue #1)`）
- 2 回目のコミット: `issue-comment.md` の追加と、`notes.md`・`artifacts-sha256.txt` の更新。**このコメント自身が 2 回目のコミットに含まれるので、その SHA はここに書けない**（`origin/worklog-issue-1` の先頭を見てください）。
- `worklog/1/` のファイル（この VM の作業ツリーでのバイト数）:

| ファイル | バイト |
|---|---|
| artifacts-sha256.txt | 1891 |
| build.log | 44 |
| compare-report.md | 31539 |
| compare-stdout.txt | 1675 |
| env.md | 4294 |
| issue-comment.md | （このファイル） |
| notes.md | 17218 |
| observed.tsv | 458146 |
| observed-dpiaware.tsv | 458252 |
| observed-manifest.tsv | 458223 |
| observed-repeat.tsv | 458198 |
| run-dpiaware.log | 665 |
| run-manifest.log | 669 |
| run-plain.log | 665 |
| run-repeat.log | 665 |

- `artifacts-sha256.txt` の先頭 5 行（全 20 行。`issue-comment.md` は含まない）:

```
b4953a4743e6b279f2bb29bb07abc0e617e2b0b6db0b9e0137bea39403da3002  worklog/1/build.log
aa4c040e1ec64f06ec53c264c898db3821fde4cbb38c961215b8927fa3988156  worklog/1/compare-report.md
27b0ba20cbceb25332834334ee341bec6562a29644ce3585a764c0a21801533e  worklog/1/compare-stdout.txt
59ed0ea26e44b47b80ff019cccc18a04a33a90c853b4beac359c06f632af954a  worklog/1/env.md
2ed44583791909b1043c857c33e9cddcc91865ee51766f03030b3600582e0eba  worklog/1/notes.md
```

- ビルドした exe はコミットしていない（SHA-256 は `artifacts-sha256.txt` に記録。`replay.exe` と `replay-m10.exe` は同一内容で `8907e0e1...0443ab7`）。
- **改行コードの注意**: この VM の git は `core.autocrlf=true`（既存の設定）。作業ツリーが CRLF の `compare-report.md`、`compare-stdout.txt`、`run-*.log`（6 ファイル）は、コミット内では LF になり、`artifacts-sha256.txt` のハッシュと一致しない（改行だけの違い。`observed*.tsv` は無加工でハッシュも一致）。詳細は `notes.md` §8。

### 6. 禁止事項の確認

守った。FL Studio のファイルは探しても入手してもいない。資格情報は見ていない（`gh auth status` を 1 回実行。トークンは出力側で伏せられていた）。レジストリ・システム設定・環境変数・ロケール・DPI は変更していない。管理者として実行しておらず、昇格は一度も要求していない。`replay.c`・`cases.tsv`・`compare.py`・`replay-win10.manifest` は無改変、`--no-mask` は未使用。`main` へ push せず、force push もしていない。ユーザー名・コンピュータ名・メール・IP が出力に無いことを走査して確認した（一致 0 件）。

守れたか迷う点を正直に書く:
- `git clone` はしなかった（作業ディレクトリが既にこのリポジトリで、`main` = 159a42e、作業ツリーは clean だった）。
- `worklog-issue-1` というローカルブランチが既にあり、`git switch -c` が失敗した。`main` と同一で、リモートに無いことを確認してから、既存ブランチに `git switch` して使った。
- `Get-Command gh` と `gh auth status` で認証状態を見た。

### 7. 確認できなかったこと

- 画面（デスクトップ）を見ていない。接続方法・設定アプリの拡大率・Windows Update の状態は、コマンド出力からの推測または不明。
- Issue の手順のコードの不具合 2 件（測定結果には影響しない）:
  1. この VM の PowerShell 5.1 で `Get-FileHash` が使えなかった。.NET の SHA256 で代用した（`Get-FileHash` との照合はしていない）。
  2. Step 7 の `@($_.FullName, "worklog/$ISSUE/" + $_.Name)` は、`,` が `+` より先に評価されて、ファイル名の欄が欠けた。括弧を付けて作り直した。
- `compare.py` の出力は、PowerShell の `Tee-Object` では UTF-16 になったので、`PYTHONUTF8=1` で UTF-8 に作り直した（`compare.py` は無改変）。
- 実行 C の最初の試行は、作業ディレクトリを間違えて失敗した（`replay.exe` は起動されず、ファイルも作られていない）。絶対パスでやり直した。
- A と A' の違いは、ポインタ・時刻・ID の類と見たが、全行を 1 件ずつは確認していない。

### 8.

測った値の解釈（A2b を満たすか）はしていない。
