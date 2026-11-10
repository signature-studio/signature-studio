<!--
  投稿用の下書き（GitHub Issue の本文としてそのまま貼れる）。投稿前に PM が確認すること。
  題名案: [win-vm] A2b: 実機 Windows で Windows API の応答を測る（tools/win-replay）
  ラベル案: windows-vm, measurement
  本文中の <ISSUE> は、投稿後に付く Issue 番号に PM が置換する（コードブロック内の `$ISSUE = '<ISSUE>'` を含む）。それ以外の山括弧（<版>、<保存した名前>、<masked> など）は、エージェントが埋める説明用で、置換不要。
-->

# 実機 Windows で Windows API の応答を測る（A2b の実機確認）

あなたは Hyper-V 上の Windows VM で動く OpenCode のエージェントです。この Issue は**あなた宛ての指示書**です。上から順に実行し、結果をリポジトリの `worklog/<ISSUE>/` にコミットして、この Issue にコメントで報告してください。途中で質問はできません（返事を待つ人はいません）。迷ったら「禁止事項」に反しない範囲で自分で判断し、**判断とその理由を `notes.md` に書いてください**。

## 1. 目的（背景）

- このプロジェクトは FLEngine_x64.dll を x86-64 → C → wasm に変換してブラウザで動かします。Windows API の呼び出しには、プロジェクト自前の「応答関数」が答えます。
- 完了条件 A2b は「実行で踏まれる応答が全て仕様確認済み」（決定 0046 §3）です。完走した実行で呼ばれた応答 171 種・13,572 回のうち、**実機で確認できていない創作・仮の値**が多数あります（`docs/investigations/a2b-audit/SUMMARY.md`）。
- 応答の正しさは、**実機の Windows で同じ API を呼んで値を得る**ことでしか確認できないものがあります。実機の Windows を使えるのは、この VM だけです。
- そこで、実行の記録（`docs/investigations/a4-six-module-trace/`）から作った「再生ツール」`tools/win-replay/` を、この VM で走らせてください。ツールは **Windows の標準 API だけを読み取り専用で呼ぶ**独立した小さなプログラムです（呼ぶ API の一覧は `tools/win-replay/README.md`）。結果は `compare.py` でプロジェクトの runtime の応答と突き合わせます。
- **あなたの仕事は測って記録することです。** 結果の解釈（A2b を満たすかどうか）はあなたの仕事ではありません。値を見て「仕様を決めた」と書かないでください。

## 2. 前提

- Windows 10 または 11（x64）の VM。管理者権限は**不要**（管理者として実行しないこと）。
- 必要なもの: `git`、PowerShell（5.1 でも 7 でも可）、**C コンパイラ**（`cl` または `gcc`。Step 3）。Python 3 は `compare.py` 用で、無ければ Step 6 を省略してよい。
- **FL Studio をインストールしない。** FL Studio のファイル（exe・DLL・インストーラ・フォント・設定・ライセンス）を探さず、持ち込まず、リポジトリにも入れない。再生ツールは FL Studio のコードを含まず、必要ともしない。
- VM の状態は**あるがままで測る**。ロケールや表示言語、DPI、解像度、タイムゾーンを「en-US に合わせる」などの変更はしない（機種・VM 依存の値は記録して参考にするだけで、変えて良い値ではない）。
- 最初に **OS のバージョン・ビルド番号・ロケール・DPI・モニタ構成を記録**する（Step 1。`env.md`）。コンピュータ名とユーザー名は書かない。
- 所要時間の目安: コンパイラが有れば 30〜60 分（ツール自体の実行は数秒〜数十秒の見込み＝**読み**、未確認）。

## 3. 手順

コマンドは PowerShell の例です。エラーが出たら、コマンドと出力を `notes.md` に書き、直せるものは直して続けてください。全体を通して、**画面に出たコマンドと出力は `notes.md` に残す**（長いものは要点と、保存したファイル名）。

### Step 0. 変数と作業ディレクトリ

```powershell
$ISSUE = '<ISSUE>'
$ErrorActionPreference = 'Continue'
Set-Location $HOME            # 書き込める場所なら他でもよい。パスは notes.md に「<HOME>\...」の形で書く
```

### Step 1. 環境の記録（`env.md`）

次のコマンドの出力を `env.md` に貼ります（出力の中にコンピュータ名・ユーザー名・ドメイン名・IP・シリアル番号があれば `<masked>` に置換する）。何も変更しない読み取りだけのコマンドです。

```powershell
$os = Get-CimInstance Win32_OperatingSystem
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
"OS: {0} | Version {1} | Build {2}.{3} | {4} | DisplayVersion {5}" -f $os.Caption, $os.Version, $cv.CurrentBuild, $cv.UBR, $os.OSArchitecture, $cv.DisplayVersion
"MUILanguages: " + ($os.MUILanguages -join ', ')
Get-WinSystemLocale | Format-List Name, LCID, DisplayName
Get-Culture | Format-List Name, LCID
Get-UICulture | Format-List Name, LCID
"ANSI code page: " + [System.Globalization.CultureInfo]::CurrentCulture.TextInfo.ANSICodePage
"OEM code page : " + [System.Globalization.CultureInfo]::CurrentCulture.TextInfo.OEMCodePage
"Win32 TimeZone: " + (Get-TimeZone).Id
(Get-WinUserLanguageList | ForEach-Object { $_.LanguageTag }) -join ', '
(Get-CimInstance Win32_ComputerSystem | Select-Object Manufacturer, Model | Format-List | Out-String)
Get-CimInstance Win32_VideoController | Format-List Name, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentBitsPerPixel, CurrentRefreshRate
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Screen]::AllScreens | ForEach-Object { "{0} primary={1} bounds={2} working={3} bpp={4}" -f $_.DeviceName, $_.Primary, $_.Bounds, $_.WorkingArea, $_.BitsPerPixel }
Get-ItemProperty 'HKCU:\Control Panel\Desktop' -Name LogPixels -ErrorAction SilentlyContinue | Select-Object LogPixels
Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -Name AppliedDPI -ErrorAction SilentlyContinue | Select-Object AppliedDPI
$PSVersionTable.PSVersion.ToString()
```

加えて、次を**あなたが見て**書いてください（コマンドでは分からない）: VM への接続方法（Hyper-V の基本セッションか、拡張セッション／RDP か）、表示のスケール（設定アプリの「拡大/縮小」の値）、モニタの台数、Windows が最新の更新を適用済みかどうか（分かる範囲）。

### Step 2. リポジトリの取得とブランチ

```powershell
git clone git@github.com:signature-studio/signature-studio-develop.git
Set-Location signature-studio-develop
git rev-parse HEAD                    # notes.md に記録
$REPO = (Get-Location).Path
$RP = Join-Path $REPO 'tools\win-replay'
$WL = Join-Path $REPO "worklog\$ISSUE"
Test-Path (Join-Path $RP 'replay.c'), (Join-Path $RP 'cases.tsv'), (Join-Path $RP 'compare.py')    # すべて True のはず
New-Item -ItemType Directory -Force $WL | Out-Null
git switch -c "worklog-issue-$ISSUE"      # git switch が無い古い git では: git checkout -b "worklog-issue-$ISSUE"
```

- clone に SSH が使えなければ、この VM に**既に設定されている**認証で使える URL（HTTPS など）に替えてよい。新しい認証情報を作らない・探さない。clone できなければ、その旨を `notes.md` に書き、**ここで終了**する（Issue にコメントできるなら 1 行だけ書く）。
- `tools\win-replay\` が無い場合も同様に終了し、`git log -1` の出力を報告する。

### Step 3. ビルド

まず、使えるコンパイラを確認します。

```powershell
Get-Command cl, gcc, clang -ErrorAction SilentlyContinue | Format-Table Name, Source, Version -AutoSize
```

- **`cl` がある**（Visual Studio / Build Tools の「x64 Native Tools」端末、または `vcvars64.bat` を呼んだ後）:
  ```powershell
  Push-Location $RP
  New-Item -ItemType Directory -Force out | Out-Null
  cl /nologo /O2 /W3 /MT replay.c /Foout\ /Feout\replay.exe /link /MANIFEST:NO 2>&1 | Tee-Object (Join-Path $WL 'build.log')
  Pop-Location
  ```
  `cl` が PATH に無いが Visual Studio が入っている場合は、`"${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath` で場所を探し、その下の `VC\Auxiliary\Build\vcvars64.bat` を呼んでから同じコマンドを実行する。
- **`gcc`（mingw-w64）がある**（x64 向けであること。`gcc -dumpmachine` が `x86_64-w64-mingw32` など）:
  ```powershell
  Push-Location $RP
  New-Item -ItemType Directory -Force out | Out-Null
  gcc -O2 -Wall -o out\replay.exe replay.c 2>&1 | Tee-Object (Join-Path $WL 'build.log')
  Pop-Location
  ```
- **どちらも無い**: インストーラを使わず、ユーザーのディレクトリに展開するだけの携帯版 mingw-w64（w64devkit）を使う。`https://github.com/skeeto/w64devkit/releases` の最新リリースから x64 版（自己展開 7-zip の `w64devkit-x64-<版>.exe`）を `$HOME\winreplay-toolchain` に保存し、`& "$HOME\winreplay-toolchain\<保存した名前>" "-o$HOME\winreplay-toolchain" -y` で展開（**読み**。資産名と展開オプションは未確認）。そして**このシェルの PATH にだけ**追加する（`$env:PATH = "$HOME\winreplay-toolchain\w64devkit\bin;" + $env:PATH`）。システム／ユーザーの環境変数やレジストリは変更しない。保存したファイルの SHA-256 と `gcc --version` を `toolchain.md` に書く。
- どのコンパイラでもビルドできない場合は、エラーの全文を `build.log` と `notes.md` に書いて終了する（Step 8 のコミットは `env.md`・`notes.md`・`build.log` だけで行い、Step 9 でコメントする）。

ビルドが通ったら:

```powershell
Test-Path (Join-Path $RP 'out\replay.exe')           # True
& (Join-Path $RP 'out\replay.exe') --list-whitelist | Select-Object -Last 1     # "# total=174 traced=153 helper=21" のはず
```

`total` が 174 でなければ、`notes.md` に書いてください（ツールを書き換えたり、別の版を使ったりしていないか確認する）。

### Step 4. 実行

実行ごとに、**終了コードを確認**します（0＝完走、3＝呼んだ API がクラッシュ、2＝入出力エラー）。ネットワークは使いません。`replay.exe` の実行中に、ダイアログ（SmartScreen、ウイルス対策ソフト、セキュリティ警告）が出たら、内容を `notes.md` に書き、**ウイルス対策ソフトや SmartScreen の設定は変えない**（許可の操作が必要なら、その 1 回の操作だけで進めてよい。それも `notes.md` に書く）。

```powershell
Set-Location $WL

# A: manifest なし（基準の実行）
& (Join-Path $RP 'out\replay.exe') --cases (Join-Path $RP 'cases.tsv') --out observed.tsv --tag plain *>&1 | Out-File run-plain.log -Encoding ascii
"exit code A: $LASTEXITCODE"

# A': A をもう一度（毎回変わる値を見分けるため）
& (Join-Path $RP 'out\replay.exe') --cases (Join-Path $RP 'cases.tsv') --out observed-repeat.tsv --tag plain-repeat *>&1 | Out-File run-repeat.log -Encoding ascii
"exit code A': $LASTEXITCODE"

# B: Windows 10 互換 manifest を付けた実行（コピーした別名の exe に外部 manifest を置く）
Copy-Item (Join-Path $RP 'out\replay.exe') (Join-Path $RP 'out\replay-m10.exe')
Copy-Item (Join-Path $RP 'replay-win10.manifest') (Join-Path $RP 'out\replay-m10.exe.manifest')
& (Join-Path $RP 'out\replay-m10.exe') --cases (Join-Path $RP 'cases.tsv') --out observed-manifest.tsv --tag win10-manifest *>&1 | Out-File run-manifest.log -Encoding ascii
"exit code B: $LASTEXITCODE"
```

- 終了コードが 3 だった実行は、**同じコマンドに `--resume` を付けて**もう一度実行する（続きから再開し、クラッシュしたケースは `crash:` の行として残る）。何度クラッシュしても、`--resume` を付けて最後まで進める。クラッシュしたケース番号（`observed*.tsv` の `crash:` 行）は `notes.md` に書く。
- 数分たっても終わらない（進行表示が止まる）ときは `Ctrl+C` で止め、`--resume` を付けて再実行する（止まっていたケースは `hang-or-killed` として記録され、次から続く）。
- 実行 A のときだけ、画面の拡大が 100% でない場合は、任意で第 3 の実行も行う: `& (Join-Path $RP 'out\replay.exe') --cases (Join-Path $RP 'cases.tsv') --out observed-dpiaware.tsv --tag dpi-aware --dpi-aware`。
- **`--no-mask` は使わない。** `replay.c`・`cases.tsv`・`compare.py` を書き換えない。

### Step 5. 実行結果の確認

```powershell
$nCases = @(Get-Content (Join-Path $RP 'cases.tsv') | Where-Object { $_ -and -not $_.StartsWith('#') -and -not $_.StartsWith('case_id') }).Count
foreach ($f in 'observed.tsv', 'observed-repeat.tsv', 'observed-manifest.tsv') {
    $n = @(Get-Content $f | Where-Object { $_ -and -not $_.StartsWith('#') }).Count
    "{0}: rows={1} cases={2}" -f $f, $n, $nCases
    Get-Content $f | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object { ($_ -split "`t")[2] -replace '^(skip|crash|err).*', '$1' } | Group-Object | Sort-Object Count -Descending | ForEach-Object { "    {0} = {1}" -f $_.Name, $_.Count }
}
Get-Content observed.tsv | Where-Object { $_.StartsWith('#') } | Select-Object -First 12      # 環境行（RtlGetVersion など）が env.md と合うか確認
$a = Get-Content observed.tsv | Where-Object { -not $_.StartsWith('#') }
$b = Get-Content observed-repeat.tsv | Where-Object { -not $_.StartsWith('#') }
"rows that differ between A and A': " + @(Compare-Object $a $b).Count
```

- 行数が `cases` と同じであること、`ok` が大半であることを確認する（**読み**: `ok` はおよそ 2,000 件以上のはず。`nomodule`/`noexport`/`skip` は多くても数十件）。クラッシュや `hang-or-killed` の行があっても、それは**有効な測定結果**（その API が実機で落ちた）なので、そのまま残す。
- `RtlGetVersion` の版とビルド番号が、Step 1 の `env.md` と一致するか確認する。

### Step 6. 突き合わせ（Python 3 がある場合）

```powershell
$py = Get-Command python, py -ErrorAction SilentlyContinue | Select-Object -First 1
if ($py) {
    & $py.Source (Join-Path $RP 'compare.py') --cases (Join-Path $RP 'cases.tsv') --observed observed.tsv --observed-manifest observed-manifest.tsv --report compare-report.md *>&1 | Tee-Object compare-stdout.txt
} else {
    "Python 3 が無いので compare を省略した" | Out-File compare-skipped.txt -Encoding ascii
}
```

- Python を**インストールしてはいけない**（システムの変更になる）。無ければ省略し、`notes.md` に書く（PM が後で実行する）。
- `compare-stdout.txt` の「top mismatches」が、Issue コメントに貼る「不一致の上位 20」です。

### Step 7. ハッシュと整理

```powershell
$items = @()
Get-ChildItem $WL -File | Where-Object { $_.Name -ne 'artifacts-sha256.txt' } | ForEach-Object { $items += , @($_.FullName, "worklog/$ISSUE/" + $_.Name) }
foreach ($rel in 'replay.c', 'cases.tsv', 'compare.py', 'gen_cases.py', 'replay-win10.manifest', 'out\replay.exe', 'out\replay-m10.exe') {
    $p = Join-Path $RP $rel
    if (Test-Path $p) { $items += , @($p, 'tools/win-replay/' + $rel.Replace('\', '/')) }
}
$items | ForEach-Object { '{0}  {1}' -f (Get-FileHash $_[0] -Algorithm SHA256).Hash.ToLower(), $_[1] } | Set-Content (Join-Path $WL 'artifacts-sha256.txt') -Encoding ascii
Get-Content (Join-Path $WL 'artifacts-sha256.txt')
```

コンパイラを自分で取得した場合は、その配布ファイルの SHA-256 も `toolchain.md` に書く。**ビルドした exe（`out\`）はコミットしない**（SHA-256 だけ記録する）。

### Step 8. コミットと push

```powershell
Set-Location $REPO
Remove-Item (Join-Path $WL '*.inflight') -ErrorAction SilentlyContinue      # 再開用の目印。コミットしない
git add "worklog/$ISSUE"
git status --short                   # worklog/<ISSUE>/ 以外が出たら、理由を調べて notes.md に書く（出ていてもコミットしない）
git -c user.name="win-vm-agent" -c user.email="<email-removed>" commit -m "worklog: Windows API replay results (issue #$ISSUE)"
git rev-parse HEAD
git push -u origin "worklog-issue-$ISSUE"
```

- `git config` は変更しない（上のコマンドの `-c` で足りる）。`main` に push しない。force push しない。他のブランチに触れない。
- push できない（権限・認証が無い）場合は、`git bundle create (Join-Path $HOME "worklog-issue-$ISSUE.bundle") "worklog-issue-$ISSUE"` を作って場所を `notes.md` と Issue コメントに書き、**認証情報を探さずに**終了する。

### Step 9. Issue へのコメント

`gh` が**既に認証済み**なら `gh issue comment $ISSUE --body-file (Join-Path $WL 'issue-comment.md')`。そうでなければ、コメント本文をあなたの最終応答にそのまま書く（PM が Issue に貼る）。コメントに書く項目:

1. **環境**: OS（Caption・Version・Build.UBR・DisplayVersion）、ロケール、ANSI/OEM コードページ、DPI（拡大率）、モニタ構成、接続方法、コンパイラ名とバージョン。
2. **実行件数**: 実行 A / A' / B のそれぞれで、`cases`・`rows`・status 別の件数（`ok` / `skip:policy` / `nomodule` / `noexport` / `crash` / `hang-or-killed`）、終了コード。A と A' で違った行数。
3. **一致／不一致の数**: `compare.py` の verdicts 行（match / mismatch / reference / real-only / skipped）。実行しなかった場合はその理由。
4. **不一致の上位 20**: `compare-stdout.txt` の「top mismatches」の表（番号・ケース・API・差）をそのまま貼る。
5. **コミット**: ブランチ名、コミットの SHA、`worklog/<ISSUE>/` のファイル一覧と各サイズ、`artifacts-sha256.txt` の先頭 5 行。
6. **規則の確認**: 「禁止事項」を守ったか（守れなかった点があれば、何をしたかを正直に）。
7. **確認できなかったこと**: 実行できなかった手順、疑わしい結果、`notes.md` に書いた判断のうち重要なもの。
8. 「測った値の解釈（A2b を満たすか）はしていない」の 1 行。

同じ内容を `worklog/<ISSUE>/issue-comment.md` にも保存し、**コミットし直して**（`git add` → `git commit` → `git push`）ください。

## 4. 成功条件

次をすべて満たしたら、この Issue の作業は成功です。満たせない項目があっても、**できたところまでを提出**してください（その場合は「確認できなかったこと」に書く）。

- [ ] `tools/win-replay/replay.c` をそのまま（無改変で）ビルドできた。`--list-whitelist` の最後が `total=174`。
- [ ] 実行 A と B が完走した（終了コード 0。クラッシュ後は `--resume` で最後まで）。`observed.tsv` と `observed-manifest.tsv` の行数が `cases.tsv` の件数と一致。実行 A' も同様。
- [ ] `compare-report.md` を作った（Python が無ければ省略の理由を書いた）。
- [ ] `worklog/<ISSUE>/` に、`env.md`、`observed.tsv`、`observed-repeat.tsv`、`observed-manifest.tsv`、`compare-report.md`、`notes.md`、`artifacts-sha256.txt`、実行ログ（`run-*.log`、`build.log`）をコミットし、ブランチを push した（または bundle を作った）。
- [ ] Issue にコメントした（または最終応答に本文を書いた）。
- [ ] 「禁止事項」に違反していない（違反したら、正直に書いてある）。

## 5. 禁止事項

- **FL Studio のバイナリ・インストーラ・DLL・フォント・設定・ライセンス・抽出物を、入手・インストール・コピー・リポジトリへの追加のいずれもしない。** 探しにも行かない。
- **資格情報に触れない**: トークン、SSH 鍵、パスワード、ブラウザや資格情報マネージャーに保存されたもの、環境変数に入っている秘密。出力にも `notes.md` にも貼らない。`git clone`/`push` は VM に**既に設定されている**認証をそのまま使うだけ。新しく作らない。
- **レジストリとシステム設定を変更しない**: ロケール、表示言語、DPI、解像度、タイムゾーン、ウイルス対策・ファイアウォール・Windows Update・電源、システム／ユーザーの環境変数（PATH を含む。PATH は実行中のシェルの中だけで変える）、サービス、スケジュールタスク。ソフトウェアのインストーラも使わない（Step 3 の携帯版の展開を除く）。
- **ネットワークへ測定結果以外を送らない**: 送ってよいのは、`git push`（`worklog-issue-<ISSUE>` ブランチ）と Issue コメントだけ。ダウンロードしてよいのは、`git clone` と、Step 3 のコンパイラ（必要なときだけ）。ツールにネットワーク機能は無い。
- **`replay.c`・`cases.tsv`・`compare.py`・`replay-win10.manifest` を書き換えない**（測定の同一性のため）。どうしても直す必要があるときは、元のファイルをそのまま残し、直した版を別名で `worklog/<ISSUE>/` に置いて、diff と理由を `notes.md` に書く。
- `--no-mask` を使わない。コンピュータ名・ユーザー名・メールアドレス・IP アドレスを、`env.md`・`notes.md`・コメントのどこにも書かない。
- 管理者として実行しない。`main` に push しない。force push しない。他の Issue ・PR・ブランチを変更しない。
- 禁止事項に触れそうになったら、その操作はしないで `notes.md` に書き、できる範囲で続行する。

## 6. 提出物

### 6.1 Issue コメント

Step 9 の 8 項目。

### 6.2 `worklog/<ISSUE>/` にコミットするもの

**可能な限り、作業メモとアーティファクトに関する情報をコミットする**（捨てずに残すこと）。

| ファイル | 内容 |
|---|---|
| `env.md` | Step 1 の環境の記録（ロケール、DPI、モニタ構成、接続方法、VM であること） |
| `observed.tsv` | 実行 A の出力（基準） |
| `observed-repeat.tsv` | 実行 A' の出力（A の繰り返し） |
| `observed-manifest.tsv` | 実行 B（Windows 10 互換 manifest あり）の出力 |
| `observed-dpiaware.tsv` | （任意）実行 C |
| `compare-report.md`、`compare-stdout.txt` | `compare.py` の出力（実行したとき） |
| `notes.md` | 作業メモ: 実行したコマンド（全部）、エラーとその対処、判断とその理由、予想外の挙動、所要時間、ダイアログ、クラッシュしたケース番号、禁止事項の確認 |
| `artifacts-sha256.txt` | リポジトリ内の入力ファイルと、ビルドした exe、出力ファイルの SHA-256 一覧 |
| `build.log`、`run-plain.log`、`run-repeat.log`、`run-manifest.log` | ビルドと実行の出力 |
| `toolchain.md` | （コンパイラを自分で取得したとき）取得元、ファイル名、SHA-256、`gcc --version` |
| `issue-comment.md` | Issue に書いたコメントの本文 |

出力の `.tsv` は 1 ファイル 1 MB 前後です。これらを**加工しない**（行の削除・並べ替え・整形をしない）。

## 7. Issue 番号の扱い

本文中の `<ISSUE>` は**プレースホルダ**で、この Issue を投稿した後に PM が実際の番号に置換します。置換されずに `<ISSUE>` のまま残っている場合は、この Issue の URL に含まれる番号を使い、`notes.md` にその旨を書いてください。

## 8. うまくいかないとき

| 症状 | 対処 |
|---|---|
| `cl` が見つからない | 「x64 Native Tools」端末で開くか、`vcvars64.bat` を呼ぶ。無ければ Step 3 の他の方法 |
| ビルドで警告 | 警告だけなら続行（`build.log` に残す）。エラーなら全文を残して終了 |
| `replay.exe` がウイルス対策／SmartScreen にブロックされる | 設定は変えない。ブロックの内容を `notes.md` に書く。その 1 回の許可だけで進められるなら進める。進められなければ、そこで終了して報告する |
| 実行がクラッシュ（終了コード 3） | `--resume` を付けて再実行（Step 4） |
| `nomodule` / `noexport` が多い | そのまま残す（実機に DLL／export が無いという測定結果）。数を報告する |
| `skip:policy` がある | 設計どおりの拒否（FL Studio の DLL 名、書き込み系の呼び出しなど）。そのまま残す |
| Python が無い | Step 6 を省略し、`compare-skipped.txt` と `notes.md` に書く |
| push できない | bundle を作る（Step 8） |

## 9. 参考（読まなくても作業できる）

- ツールの説明: `tools/win-replay/README.md`（呼ぶ API の一覧、検査、入出力の形式、限界）。wine での煙テストの記録: `tools/win-replay/SMOKE-TEST.md`（**wine の値は実機の値ではない**）。
- 背景: `docs/investigations/a2b-audit/SUMMARY.md`、`docs/investigations/a4-six-module-trace/trace-report.md`。
- このフロー全体の説明: `docs/windows-delegation/README.md`。
