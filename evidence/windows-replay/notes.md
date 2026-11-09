# notes.md — Issue #1 作業メモ（win-replay 実機測定）

パスは `<HOME>\signature-studio-develop`（= `$REPO`）の形で書く。コンピュータ名・ユーザー名は書いていない。
**測って記録しただけで、値の解釈（A2b を満たすか）はしていない。**

## 0. Issue 番号

本文の `1` は、そのまま Issue 番号 1 として使った（`gh issue view 1` で本文を確認済み。置換漏れかどうかは確認していない）。

## 1. 判断と理由

| # | 状況 | 判断 | 理由 |
|---|---|---|---|
| 1 | Step 2 の `git clone` | **clone しなかった**。作業ディレクトリが既にこの Issue のリポジトリ（`origin` = `https://github.com/signature-studio/signature-studio-develop.git`、`main` = `159a42e2f6376df21bbe8b481af31a962d521f48`）だった | 同じ内容を再 clone する意味が無い。`replay.c`・`cases.tsv`・`compare.py`・`replay-win10.manifest`・`gen_cases.py` は全て存在（Test-Path = True）。作業ツリーは clean だった |
| 2 | Step 2 の `git switch -c worklog-issue-1` | `fatal: a branch named 'worklog-issue-1' already exists` となった。**既存のローカルブランチに `git switch` した** | 調べたところ、そのブランチは `main` と同じコミット（159a42e）を指し、`main` との差は 0/0、リモートには存在しない（`git ls-remote --heads origin` は `main` のみ）。誰かの作業を上書き・破棄する恐れが無いと確認してから使った。ブランチは作り直していない |
| 3 | コンパイラ | VM に既にある **Visual Studio 2022 Community**（17.14.44、`cl` 19.44.35229 x64）を `vcvars64.bat` 経由で使った。w64devkit などは取得していない（`toolchain.md` は無し） | Issue の Step 3 の第 1 候補。`cl` は PATH に無かったので、`vswhere` で場所を探して `vcvars64.bat` を呼んだ。ビルド用の一時 .bat は `%TEMP%` に置いた（リポジトリ外）。環境変数・レジストリは変更していない |
| 4 | 実行 C（`--dpi-aware`、任意） | **実行した** | Issue は「拡大が 100% でない場合は任意」としている。後で分かったが、この VM は約 175% と推測されるので（env.md）、実行して損は無いと判断した |
| 5 | `compare-stdout.txt` の文字コード | 最初の `Tee-Object` の出力は UTF-16 LE（BOM 付き）だった。**`PYTHONUTF8=1` を付け、`cmd /c` でリダイレクトして UTF-8 で作り直した**。`compare-report.md` も同じコマンドで作り直した（Python が書くので UTF-8。内容は同じ） | Windows PowerShell 5.1 の `Tee-Object` は既定で UTF-16。そのままだと Issue コメントに貼れない。`compare.py` は書き換えていない |
| 6 | 管理者権限 | **昇格は一度も要求していない**（`IsInRole(Administrator)` = False のまま） | 追加の指示で「昇格ダイアログを操作できる人がいない」と言われた。元から管理者は不要という Issue の方針とも合う。UAC・SmartScreen・ウイルス対策のダイアログは、**一度も出なかった**（画面は見られないが、全コマンドが止まらず完了した） |

## 2. 実行したコマンド（要点。全て読み取り／ビルド／実行のみ）

1. `git rev-parse --show-toplevel; git branch --show-current; git rev-parse HEAD; git status --short; git remote -v`（認証情報の部分は `<masked>` に置換する式を付けた。実際には URL に認証情報は無かった）
2. `Get-Command cl, gcc, clang, python, py, gh` → `python`（3.14.10）と `gh` のみ PATH にあった。`cl` は PATH に無し
3. `vswhere.exe -latest -products * -property installationPath` → `C:\Program Files\Microsoft Visual Studio\2022\Community`
4. `gh auth status`（アカウント名は `<masked>` に置換して表示。トークンは `gho_***` と伏せられていた。**中身は見ていないし、書かない**）
5. `git branch -a -vv` / `git rev-list --left-right --count main...worklog-issue-1` / `git ls-remote --heads origin`（判断 2 のため）
6. `git switch worklog-issue-1`
7. Step 1 の環境記録コマンド一式 → `env.md`
8. ビルド（`%TEMP%\winreplay-build.bat`）:
   ```
   call "...\VC\Auxiliary\Build\vcvars64.bat" >nul
   cd /d <REPO>\tools\win-replay
   cl /nologo /O2 /W3 /MT replay.c /Foout\ /Feout\replay.exe /link /MANIFEST:NO
   ```
   → 警告・エラー無し、`CL_EXIT=0`。`build.log` は 44 バイト（`replay.c` と `CL_EXIT=0` の 2 行のみ）
9. `out\replay.exe --list-whitelist | Select-Object -Last 1` → `# total=174 traced=153 helper=21`（想定どおり）
10. 実行 A: `out\replay.exe --cases cases.tsv --out observed.tsv --tag plain` → 終了コード 0（7 秒）
11. 実行 A': `... --out observed-repeat.tsv --tag plain-repeat` → 終了コード 0（3 秒）
12. 実行 B: `replay.exe` を `out\replay-m10.exe` にコピーし、`replay-win10.manifest` を `out\replay-m10.exe.manifest` として置いて `... --out observed-manifest.tsv --tag win10-manifest` → 終了コード 0（3 秒）
13. 実行 C: `out\replay.exe ... --out observed-dpiaware.tsv --tag dpi-aware --dpi-aware` → 終了コード 0
14. Step 5 の確認コマンド、Step 6 の `compare.py`（下記）、Step 7 のハッシュ、Step 8 のコミット
15. 解析のための読み取りだけのコマンド: A と A'／B／C の行ごとの差分の数え上げ、`skip` 行の一覧、ログ・出力ファイルに名前（ユーザー名・コンピュータ名）やメール・IP が混ざっていないかの走査、`Get-HotFix`、`SESSIONNAME` の確認、GDI の DPI の読み取り

`--no-mask` は使っていない。`replay.c`・`cases.tsv`・`compare.py`・`replay-win10.manifest` は書き換えていない（`git status` で `tools/` に変更が出ていないことを確認済み。`out\` は `tools/win-replay/.gitignore` の `out/` で除外されている）。

## 3. エラー・つまずき

- 実行 C の最初の試行で、作業ディレクトリを間違えて（`worklog\1` の下で相対パス `tools\win-replay` を解決しようとして）`Resolve-Path` が失敗した。`replay.exe` は起動されておらず、ファイルも作られていない。絶対パスで実行し直した（上の 13）。同じ呼び出しの `compare.py` の相対パス失敗は無かった。
- Step 7 のハッシュ計算で 2 つ問題があった（どちらも Issue のコードのままでは動かない）。
  1. この VM の Windows PowerShell 5.1 では `Get-FileHash` が「コマンドレットとして認識されない」エラーになった（`Get-Command Get-FileHash` も空）。原因は調べていない（モジュールの読み込みの問題と思われるが未確認）。代わりに .NET の `System.Security.Cryptography.SHA256` で計算した。結果は同じ SHA-256 のはず（`Get-FileHash` で照合はできていない）。
  2. Issue の Step 7 のコード `@($_.FullName, "worklog/$ISSUE/" + $_.Name)` は、PowerShell では `,` が `+` より先に評価されるため、要素が 3 つの配列になり、2 番目の要素（表示名）が `worklog/1/` だけになる。最初に作った `artifacts-sha256.txt` はファイル名の欄が欠けていた（`worklog/1/` だけが並んだ）。表示名を `( ... )` で囲んで作り直した。Issue のコードの不具合であり、測定結果には関係しない。
- `compare-stdout.txt` が UTF-16 になった件は判断 5 のとおり。
- シェルの日本語出力が文字化けした（コンソールのコードページの問題）。`env.md` の日本語の値は、化けたバイト列から復元した（env.md の冒頭に注記）。`observed*.tsv` と `compare-*` は影響を受けていない（`compare-stdout.txt` の `×` が正しく読めることを確認）。

## 4. 実行結果の事実

### 行数と status（実行 A / A' / B / C 共通）

| | cases | rows | ok | skip:policy | nomodule | noexport | crash | hang-or-killed | 終了コード |
|---|---|---|---|---|---|---|---|---|---|
| A (plain) | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| A' (repeat) | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| B (win10-manifest) | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |
| C (dpi-aware) | 2144 | 2144 | 2127 | 17 | 0 | 0 | 0 | 0 | 0 |

- クラッシュ・`hang-or-killed` は無かったので、`--resume` は使っていない。クラッシュしたケース番号は無し。
- `skip:policy` の 17 件は 4 実行で同じ: C01076 `GetModuleHandleW`、C01081 `GetModuleHandleA`、C01123 `FreeLibrary`、C01429 / C01431 / C01433 / C01435 / C01437 / C01439 / C01463 / C01465 / C01467 / C01471 / C01475 / C01479 / C01481 `RegCloseKey`（13 件）、C02121 `DeleteObject`。設計どおりの拒否。
- 環境行（`# RtlGetVersion 10.0.26300 platform=2 sp=0.0 suite=0x100 product=1`、`ACP 932`、`OEMCP 932`、`compiler MSVC 1944`）は `env.md` の OS（10.0.26300）・コードページ（932）と一致する。Issue が予想した `ok` ≥ 約 2,000 件にも合う（2127）。

### 実行間の差（Issue の差分コマンドと、ケース単位の数）

- A と A' の違い: Issue の `Compare-Object` の結果は **184**（差分の行数。1 ケースの違いで 2 行出る）。ケース数では **92 ケース**。内訳は、ヒープ・仮想メモリ・GDI ハンドル等が返すポインタ／ハンドル値（`HeapAlloc` ×12、`RegOpenKeyExW` ×5、`CreateCompatibleDC` ×5、`SelectObject` ×5、`LocalAlloc` ×4、`VirtualAlloc` ×4、`CharNextW` ×4、`SysAllocStringLen` ×4、…）と、時刻・ID（`GetTickCount`、`GetCurrentProcessId`、`GetCurrentThreadId`、`GetLocalTime`、`GetSystemTimeAsFileTime`、`QueryPerformanceCounter`）。
- A と B の違い: **108 ケース**。A と A' の違い（ポインタ・時刻など）に加えて、`GetModuleFileNameW` ×9（実行ファイル名が `replay.exe` と `replay-m10.exe` で違うため、戻り値の長さが 0x47 と 0x4b になる）と、バージョン系の API（下記）。
- A と C の違い: **154 ケース**。`GetSystemMetrics` で、DPI 非対応の A は画面 2194x1234（0x892 / 0x4d2）、`--dpi-aware` の C は 3840x2160（0xf00 / 0x870）。アイコンやスクロールバー等の寸法も A と C で違う（例: C01517 は 0x20 と 0x30、C01526 は 0x11 と 0x1e）。`GetDeviceCaps` の最初の 8 件は A と C で同じだった。

### バージョン系 API（A と B の違い。記録のみ）

- `GetVersion`（C00900）: A = 0x23f00206、B = 0x66bc000a。
- `GetVersionExW`（C00901）の major/minor: A = 6 / 2、B = 10 / 0。build は A = 0x23f0（9200）、B = 0x66bc（26300）。
- `RtlGetVersion` は A・B ともに 10.0.26300。

### `compare.py`（A と B を入力）

- `verdicts: match=352, mismatch=161, reference=55, real-only=1559, skipped=17`、`not run: 0`
- 上位 20 は `compare-stdout.txt` にそのまま保存した。`compare-report.md` は 31,539 バイト。
- `compare.py` に `observed-repeat.tsv` や `observed-dpiaware.tsv` は渡していない（Issue の手順どおり）。

### 出力ファイルの機密確認

- `observed*.tsv`・`compare-*`・`run-*.log`・`build.log`・`env.md`・`notes.md` を、このセッションのユーザー名・コンピュータ名・ドメイン名で走査した（値は表示していない）。一致は 0 件。メールアドレスと IPv4 らしき文字列も 0 件。
- `Users\` を含む行が各 `observed*.tsv` に 3 行ずつある。内容は `SHGetFolderPathW` の戻り値 `C:\Users\Public\Documents`、`...\Music`、`...\Pictures`（公開フォルダ。個人名は含まない）。`GetModuleFileNameW` の値は、ツールのマスク機能により `<PROFILE>\...` になっている。

## 5. 予想外・気になった点（解釈ではなく観察）

- この VM の画面は、DPI 非対応のプロセスには 2194x1234、DPI 対応には 3840x2160 に見える。OS が返す DPI 値（`AppliedDPI` と GDI）は 96 のまま。env.md の「表示のスケール」を参照。
- `Win32_VideoController` が 2 つ（Hyper-V ビデオ 1024x768 と Remote Display Adapter 3840x2160）を返し、`Screen.AllScreens` は 1 台。接続はリモートセッションと思われるが、確認していない。
- 実行 A と B で `GetVersion`／`GetVersionExW` の値が大きく違う（manifest の有無による。上記）。この違いを、どちらが「正しい」かは判断していない。

## 6. 確認できなかったこと

- 画面（デスクトップ）を見ていない。接続方法（拡張セッションか RDP か）、設定アプリの拡大率、Windows Update の状態は、コマンド出力からの推測または不明（env.md）。
- 実行 A / A' / B / C は各 3〜7 秒で終わった。Issue の見込み（数秒〜数十秒）の範囲内。

## 7. 禁止事項の確認

- FL Studio のファイルは、探さず、入手せず、リポジトリにも入れていない。再生ツールは FL Studio のコードを使わない。
- 資格情報: `gh auth status` を 1 回実行した（アカウント名は伏せ、トークンは出力側で伏せられていた）。トークン・鍵・パスワードは見ていない・書いていない。新しい認証情報は作っていない。`git push` と `gh issue comment` は既存の認証をそのまま使う。
- レジストリ・システム設定・環境変数は変更していない（PATH も。`vcvars64.bat` は一時的な `cmd` の中だけで使った）。インストーラは使っていない。ロケール・DPI・解像度・タイムゾーンは変更していない。レジストリは読み取りだけ。
- ネットワーク: `git push`（`worklog-issue-1`）と Issue コメントだけ。コンパイラのダウンロードはしていない。`git ls-remote`（読み取り）を 1 回使った。
- `replay.c`・`cases.tsv`・`compare.py`・`replay-win10.manifest` は無改変。`--no-mask` は未使用。
- 管理者として実行していない。`main` へは push しない。force push しない。他のブランチ・Issue・PR には触れない。
- **正直に書く点**: 既存の `worklog-issue-1` ブランチを使った（判断 2。`main` と同一で、リモートに無いことを確認済み）。`git clone` はしなかった（判断 1）。`git config` は変更していない（コミットは `-c` で作者を指定する）。

## 8. コミットの構成（`artifacts-sha256.txt` について）

- 1 回目のコミット（`b143e059820be84390b84a3d7e75dab360da2a46`）で、`issue-comment.md` 以外の全てと `artifacts-sha256.txt` をコミットし、push した。
- 2 回目のコミットで `issue-comment.md` を追加する。その際、この `notes.md`（このセクションと下の改行コードの記述を追加した）の SHA-256 が変わるので、**`artifacts-sha256.txt` を作り直して一緒にコミットした**。`artifacts-sha256.txt` は `issue-comment.md` を含まない（コメントの本文が、ハッシュ一覧を引用するため、循環を避けた）。
- ビルドした exe（`out\replay.exe`、`out\replay-m10.exe`）はコミットしない。SHA-256 だけ `artifacts-sha256.txt` に記録した（`out/` は `tools/win-replay/.gitignore` で除外されている）。

### 改行コードの変換（git の `core.autocrlf=true`）

- この VM の git は `core.autocrlf=true`（私が設定したものではなく、既存の設定。`git config` は変更していない）。そのため、作業ツリーで CRLF のファイルは、コミット時に LF に変換された。
- `artifacts-sha256.txt` のハッシュは、**この VM の作業ツリーにあるファイル**のもの。次の 6 ファイルは、コミットされた中身（LF）のハッシュが、記録したハッシュと**一致しない**（改行コードの違いだけ。`git ls-files --eol` と、コミットされた blob のハッシュの照合で確認した）:
  `compare-report.md`、`compare-stdout.txt`、`run-plain.log`、`run-repeat.log`、`run-manifest.log`、`run-dpiaware.log`。
  （`artifacts-sha256.txt` 自身は、ハッシュ一覧に含まれないので照合の対象外。）
- `observed.tsv`、`observed-repeat.tsv`、`observed-manifest.tsv`、`observed-dpiaware.tsv`、`env.md`、`notes.md`、`build.log` は、コミットされた中身のハッシュが記録と一致する（`observed*.tsv` は作業ツリーでも LF だった）。つまり、測定結果の `.tsv` は無加工でコミットされている。
- 対処: 変換を止めるために `git config` や `.gitattributes` を変えるのは、禁止事項（`git config` を変更しない。`worklog/1/` 以外を変えない）に触れるので、していない。上の 6 ファイルを照合するときは、改行を CRLF にそろえてから計算する。
