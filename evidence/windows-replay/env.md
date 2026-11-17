# env.md — 実機 Windows（Hyper-V VM）の環境記録（Issue #1）

記録日時: 2026-10-30 08:08 (+09:00)。コンピュータ名・ユーザー名・ドメイン名・IP・シリアルは書いていない。
注記: この環境のシェルは日本語出力を文字化けして返した（コンソールのコードページ差）。下の日本語の値（`日本語 (日本)`、`64 ビット`）は、文字化けした出力のバイト列から復元した。数値・ID の値は出力のまま。

## Step 1 コマンドの出力

```
OS: Microsoft Windows 11 Pro | Version 10.0.26300 | Build 26300.9711 | 64 ビット | DisplayVersion 26H2
MUILanguages: ja-JP

Name        : ja-JP        (Get-WinSystemLocale)
LCID        : 1041
DisplayName : 日本語 (日本)

Name : ja-JP               (Get-Culture)
LCID : 1041

Name : ja-JP               (Get-UICulture)
LCID : 1041

ANSI code page: 932
OEM code page : 932
Win32 TimeZone: Tokyo Standard Time
ユーザー言語リスト: ja

Manufacturer : Microsoft Corporation
Model        : Virtual Machine

Win32_VideoController #1
  Name                        : Microsoft Hyper-V ビデオ
  CurrentHorizontalResolution : 1024
  CurrentVerticalResolution   : 768
  CurrentBitsPerPixel         : 32
  CurrentRefreshRate          : 1
Win32_VideoController #2
  Name                        : Microsoft Remote Display Adapter
  CurrentHorizontalResolution : 3840
  CurrentVerticalResolution   : 2160
  CurrentBitsPerPixel         : 32
  CurrentRefreshRate          : 32

System.Windows.Forms.Screen.AllScreens:
\\.\DISPLAY1 primary=True bounds={X=0,Y=0,Width=2194,Height=1234} working={X=0,Y=0,Width=2194,Height=1186} bpp=32

HKCU:\Control Panel\Desktop  LogPixels      : (値なし。出力に何も出なかった)
HKCU:\Control Panel\Desktop\WindowMetrics  AppliedDPI : 96

PowerShell: 5.1.26100.9711
```

## 追加で確認したこと（読み取りのみ）

- 管理者権限: なし（`IsInRole(Administrator)` = False）。昇格は一度も要求していない。
- `[System.Drawing.Graphics]::FromHwnd(0)` の DpiX/DpiY: 96 / 96（DPI 非対応プロセスから見た値。実効スケールではない。下の「表示のスケール」を参照）。
- Windows の最新の更新: 最新のホットフィックスは KB5127647 / KB5127648 / KB5128942（いずれも 2026/10/29 にインストール）。「最新の更新を全て適用済みか」は Windows Update を見ていないので不明。
- 導入済みの道具: Visual Studio 2022 Community 17.14.44、`cl` 19.44.35229 (x64)、Python 3.14.10、GitHub CLI。いずれも、この作業の前から VM にあった。インストールはしていない。

## あなた（エージェント）が見て書く項目について

私はデスクトップの画面を見ていない（コマンド出力だけで作業した）。そのため次は**推測**で、確認していない。

| 項目 | 記録 |
|---|---|
| VM であること | Win32_ComputerSystem の Model = `Virtual Machine`（Hyper-V）。確実 |
| 接続方法 | **推測**: リモートセッション。`SESSIONNAME` が `<ハッシュ>#0` の形で、`Microsoft Remote Display Adapter`（3840x2160）が存在する。Hyper-V の拡張セッションか RDP と思われるが、どちらかは区別できていない |
| 表示のスケール | 設定アプリは見ていない。**約 175% と思われる（推測）**。根拠: DPI 非対応の実行 A では `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)` = 2194x1234（0x892/0x4d2）、`--dpi-aware` の実行 C では 3840x2160（0xf00/0x870）で、比は 3840/2194 = 1.75。`AppliedDPI`=96 と GDI の DpiX/DpiY=96 は DPI 非対応プロセスから見た値（または古い値）で、スケールの根拠にならない。`LogPixels` の値は無し。最初に「100% と思われる」と書いたのは誤りだったので訂正した |
| モニタ台数 | 1（`Screen.AllScreens` が 1 件。DISPLAY1、2194x1234、プライマリ） |
| 解像度の食い違い | `Win32_VideoController` は 3840x2160（RDP アダプタ）と 1024x768（Hyper-V ビデオ）を返し、`Screen.AllScreens` は 2194x1234。どれが「現在の画面」かは分からない。記録だけして、変更はしていない |
