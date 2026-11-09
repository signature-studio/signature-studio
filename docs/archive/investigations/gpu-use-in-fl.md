# FL Studio 自身は GPU を使って描いているか(Windows 版と macOS 版の静的な読み取り)

作成 2026-11-05(PM)。**静的な読み取りだけ**(バイナリは実行していない)。依頼者の質問2件への答えの根拠:
(1)『Serum 2 を 1 枚挿した FL の System Informer に nvgpucomp64.dll / nvwgf2umx.dll / d2d1.dll があり、プラグイン追加時の半透明オーバーレイ・サムネイルの変形ブラー・アニメを CPU でやっているわけがない』
(2)『どのみち GPU を使うなら、DirectX 前提の Windows 版より macOS 版の方が良かったのでは』。

## 1. Windows 版エンジン(`binaries/win64/FLEngine_x64.dll` build 5653、同梱の `ILTools_x64.dll`)

| 項目 | 値 | 方法 |
|---|---|---|
| import DLL | 24 本 + delay-load 17 本。グラフィックス関連は `gdi32`(127 API)、`user32`、`msimg32`、`uxtheme`、`dwmapi`、`opengl32`(**2 API だけ**: `wglUseFontBitmapsW`、`wglGetProcAddress`)。**`d2d1`・`d3d9/10/11/12`・`dxgi`・`dcomp`・`dwrite` の import は 0** | pefile(import / delay import) |
| 文字列 | ASCII / UTF-16 とも `d3d11.dll`・`d3d12.dll`・`d3d9.dll`・`d2d1.dll`・`dxgi.dll`・`dcomp.dll`・`dwrite.dll`・`d3dcompiler`・`D3D11CreateDevice`・`D2D1CreateFactory`・`CreateDXGIFactory`・`wglCreateContext` が **0 件**。つまり LoadLibrary で後から読む形でもデバイスを作れない(名前が無い) | 文字列検索 |
| シェーダー | `DXBC`・`#version`・`gl_FragColor`・`gl_Position`・HLSL の `ps_5_0` 等・`D3DCompile` が FLEngine と ILTools に **0 件**(GPU に渡すシェーダーの実体が無い) | バイト列検索 |
| 描画に使う OS の API | GDI 127 API、`AlphaBlend`(delay、msimg32)、`GradientFill`、`BufferedPaint`(uxtheme)、WIC の `WICConvertBitmapSource`、`UpdateLayeredWindow` / `SetLayeredWindowAttributes` / `DwmEnableBlurBehindWindow`(UTF-16 の名前文字列。Delphi VCL の透明窓の経路)、`DwmIsCompositionEnabled`、`DwmExtendFrameIntoClientArea` | pefile、文字列検索 |
| ライブラリ | Blend2D(CPU のラスタライザ)、FreeType、QuickFontCache(FreeType 経由。`d2d1` / `dwrite` の import なし、`quickfontcache-lift-feasibility.md`) | 既存の調査 |
| WebView2 | `Winapi.WebView2` の `ICoreWebView2*` 型が 123 か所。`WebView2Loader.dll` を LoadLibrary する(`f_be3520`)。**何を載せているかは未確認**(決定 0126 は『Browser 窓』と読む) | 文字列、決定 0126 |

**読み:** FL のエンジン自身は GPU の描画 API(Direct2D / Direct3D / DXGI / OpenGL のコンテキスト / シェーダー)を持たない。窓の中身は GDI と Blend2D が CPU で書く。GPU が関わるのは (a) Windows の DWM が窓のビットマップを合成する所(窓の移動・重なり・半透明窓・影)、(b) プラグイン自身の GUI、(c) WebView2 の中身(Chromium の GPU プロセス)。
**依頼者の System Informer の `d2d1.dll` / `nvwgf2umx.dll` / `nvgpucomp64.dll` の出所は、ここからは特定できない。** 候補は Serum 2(後述)と WebView2、OS 側の DLL(多くの OS コンポーネントが d2d1 を読む)。FL の import には無いので、FL のエンジンの描画が使っている根拠にはならない(出所の特定には、読み込みの呼び出し元か、Serum 2 を挿さない FL のモジュール一覧が要る)。

## 2. プラグイン(Serum 2)

`docs/investigations/vst3-hosting.md` の実測: Serum 2 は `d3d11`・`d2d1`・`DWrite`・`dwmapi` を import し、`D3D11CreateDevice` の文字列がある(Direct2D + D3D11 の GUI)。決定 0165 の実行では `createView`(編集画面)を呼んでおらず、`InitDll` で `D2D1CreateFactory` / `DWriteCreateFactory` が呼ばれる(失敗応答で続行)。Serum 2 を挿した FL のプロセスに d2d1 と NVIDIA のドライバ DLL があるのは、この経路で説明できる(**説明できるだけで、確認はしていない**)。

## 3. macOS 版(`flstudio_mac_26.1.7.5419.dmg`、`oss-survey/tables/mac-flstudio-static.tsv` と本日の追加確認)

- エンジン `FLEngine_x64.dylib` は universal(x86_64 31.6 MB + arm64 32.3 MB)。コンパイラは FPC 3.2.3(Windows 版は Delphi)。system import は 337(CoreGraphics 51、CoreFoundation 47、Foundation 17、CoreMIDI 16、AppKit 12、AudioToolbox 10、CoreText 9、WebKit 5、QuartzCore 1。**Metal の import は 0**)。
- UI 層 `MacVclCocoa_x64.framework`(MacVCL): `Metal`・`MetalKit`・`OpenGL`・`Quartz`・`Accelerate` をリンクし、`D1BackendMetal` クラスと `MVUseMetal` を持つ。**`default.metallib` は 6,530 B で関数は 2 つだけ(`vertexShader`、`samplingShader`)**。引数の型名は `AAPLVertex`・`viewportSize`・`colorTexture` で、Apple の『テクスチャを貼った四角形を描く』サンプルと同じ形。ブラー・ぼかし・合成のためのシェーダーは無い(`CIFilter`・`NSVisualEffect`・`CIGaussianBlur` の文字列も 0)。画像処理は `Accelerate` の vImage(`vImageBufferFill_ARGB8888`、`vImagePermuteChannels_ARGB8888` = CPU のベクトル演算)。
- **読み:** macOS 版も、CPU で描いたビットマップを Metal のテクスチャとして四角形に貼って出す構成(= 本プロジェクトの『CPU の絵を GPU のテクスチャで提示する』と同じ)で、描画そのものを GPU にはしていない。テクスチャの中身の出所(CPU のビットマップかどうか)は `replaceRegion` の文字列が無く、未確認。

## 4. 比較(Windows 版と macOS 版を lift の入力として)

| 観点 | Windows(現行) | macOS |
|---|---|---|
| FL 自身の GPU 描画 | 無い(上記) | ほぼ無い(テクスチャの四角形 2 シェーダー) |
| 窓の合成 | DWM(GPU) | Core Animation(GPU) |
| system import | 738 名 / 24 DLL | 337 |
| lift の入力の総量 | 74.6 MB(x86_64。IPP の `dsp_ippv2` 33.8 MB を含む) | arm64 スライスなら 41.8 MB(IPP が arm64 は 0.41 MB)。x86_64 スライスなら同じ |
| 命令 | x86-64(lifter 済み) | arm64 は AArch64 の lifter を新規に作る。x86_64 スライスは現行の lifter で入るが、OS 層は別 |
| OS 層 | Win32(Learn の契約を根拠に約 57k 行を実装済み、Wine 10.0 を UNOFFICIAL の補助) | Objective-C ランタイム(objc_msgSend のクラス/セレクタ表)と AppKit / CoreGraphics / CoreText / CoreAudio の模倣、Mach-O / dyld。境界(エンジンと MacVclCocoa / OsxFL)は未確認 |
| 版 | 5653(参照プロジェクトは 5654 で作成) | 5419(古い) |
| プラグイン | Windows の VST3(参照プロジェクトの 20 インスタンス) | 別バイナリ(依頼者の `ダウンロード/` に dmg があるものもある。20 インスタンス分の有無は未確認) |

**判断(PM):** GPU を理由に macOS 版へ移る根拠は、上の静的な読みでは出ない(どちらも CPU 描画 + OS の GPU 合成)。移ると Win32 向けの実装の積み上げ(約 57k 行と契約の表)が使えず、AArch64 lifter か Cocoa の模倣が要る一方、利点は入力の総量(−33 MB)と import 数に限られる。したがって現行(Windows 版)を続ける。GPU の使い道は『FL の絵を GPU で描き直す』ではなく『ブラウザ側の合成・提示を GPU にする』(決定 0240、gpu1)。再検討の条件: Windows 版で契約を決められない壁が続く、または IPP の大きさ(33.8 MB)が S1 の線を決定的に越える場合。

## 5. 未確認

- 依頼者の System Informer の DLL の出所(Serum 2 か WebView2 か OS か)。
- プラグイン追加時の半透明オーバーレイ・サムネイル・ブラー・アニメが、FL のどの描画経路(Blend2D / `AlphaBlend` / 透明窓 / WebView2)か。エンジンを実際に動かして API 呼び出しを数える調査が要る(この文書は静的な読みだけ)。
- macOS 版は何も実行していない。
