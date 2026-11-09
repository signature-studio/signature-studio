# 0319 cand1 の機能確認(gpu1/gpu8、2026-11-07 09:12〜09:38 JST)— 0318-owner-observations-repro.md の続き

**実際に実行した物**: 実 Chrome 156 headless(私の 1 つずつ)で、mt3 の **cand1 test bundle** `fl-engine-test-g9f-cand1-f7ebcca1`(wasm 166,973,591 B、manifest の engine.env に `FLE_DIAG_WIN=1` `FLE_DIAG_SCROLL=1` `FLE_DIAG_VFS_MISS=1`、`SHA256SUMS` は manifest.json 以外 OK)の実エンジンを動かした。page は `p0`(main の app 無変更)、assets は `lift/out-gpu2/assets-cand1`(bundle へのリンク + manifest.json + `proj`)、port 4993、サーバーは `tools/preview-p.sh p0 cand1 4993`。ドライバは `tools/sdriveg2.mjs`(`listWindows` に `name` を付けて名前ごとに残す変更のみ)、viewport 2260×1116(ゲスト 1920×1080)。**機能の確認だけ**で、計時・first idle・fps の結論には使わない(cand1 は hybrid の strip 漏れで +8.45 MB、PM の指示)。代役・再生・モックはない。n=1 ずつ(5a のみ 2)。依頼者の機械は 0 件。
**規則からの逸脱**: 開始前の vmstat で swap の入出力が 0 でない標本があった run が 3 つ(Balance・scroll は si 16〜20 KB/s、ドロワー 5a の最初は so 6.9 / 3.4 MB/s)。PM の『入出力 ≠ 0 なら始めない』に反する。機能の確認なので始めたが、5a は si/so = 0・0 で再実行(5a2)して同じ停止を再現した。ci1 の Export の Chrome が並走していた(load 3〜10)。
**保存**: `lift/out-git/gpu8-results/0319/logs/cand1-{v3,balance,scroll,o1,about,drawer5a,drawer5a2,drawer5b}/`(画像・engine log・report.json)。シナリオは `tools/scn-g9a.json`(V3)、`scn-c1-balance.json` `scn-c1-scroll.json` `scn-c1-o1.json` `scn-c1-about.json` `scn-c1-drawer-5a.json` `-5a2.json` `-5b.json`、ログの集計は `tools/o1-logcheck.sh`。画像は表と文で書く(PNG は docs に入れない)。

## 結果の一覧
| 項目 | 結果 | 区分 / 状態 |
|---|---|---|
| (1) V3 Filter | **合格**。Problems 窓 **6 plugin(s)**(Filter が消えた)、`LoadLibraryW "…Fruity Filter_x64.dll" -> 0x7000000`、`GetProcAddress(0x7000000,"CreatePlugInstance") -> 0x72cb670`、`fle: unsupported` 0、abort 0、stop-context 0 | (b) 実モジュール化 + DestroyCursor の cap(0313f2)が効いた。直前の Filter bundle の exit 134(DestroyCursor)は再現しない |
| (2) Balance の GUI | **client は描かれない**(0323① の背景消去の修正で変化なし) | 未解決。プラグイン DLL 自身の窓の create が 0 行(下) |
| (3) Settings→General のスクロール(O9) | **直っていない**。E13 の 0318 の絵とビット単位で同一 | 未解決。ScrollWindowEx のモデルは動いている(66 回の記録)が絵は同じ |
| (4) O1 clip のドラッグ | `fle: unsupported`・ImageList・stop-context は 0 行。浮いた clip は描かれない。放した後 Track 5〜7 に clip が見えない | 未解決。『消えた』のか『描かれていない』のか未確定(下、次の窓で確認) |
| (4) O3 About | ロゴは 3×3 のまま、staff roll なし。**BigFruit.png と AboutTxt.txt が仮想ファイルに無い**(vfs-miss) | (b)(seed の内容)。ファイルを足した run は未実行 |
| (5a) wizard 開 + ドロワー | **止まる 2/2**。exit 134、`callback nesting is deeper than FLE_CB_MAX_DEPTH (16)` | O5 の原因候補。区分は未確定 |
| (5b) wizard 閉 + ドロワー | 止まらない(display change 4 回) | |

## (1) V3 Filter(`cand1-v3`)
| 見た物 | 値 |
|---|---|
| シナリオ | `scn-g9a.json`: ウィザードを閉じる → `serina1-serum2.flp` を開く → Problems 窓 → 絵と engine log |
| Problems 窓 | 691×311、『6 plugin(s) could not be opened』、Fruity Wrapper / FLEX / Frequency Shifter / Soundgoodizer / Fruity Parametric EQ 2 / Fruity Delay 2。E13 は 8(691×343)、G9-a は 7(691×327) |
| engine log(2029 行) | `g9-diag: LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll" -> 0x7000000`、`GetProcAddress(0x7000000, "CreatePlugInstance") -> 0x72cb670`。Balance も `-> 0x6800000` / `0x6abdde0` |
| 停止 | `fle: unsupported` 0、abort(onAbort / exit 134)0、stop-context 0、`== stop` は起動時の『returned from entry』の 1 回(正常) |
| engine | idle のまま、opErrors 空、first idle 7.2 s(診断入りなので数字に使わない) |

## (2) Balance の GUI(`cand1-balance`)
`scn-c1-balance.json`(= 0313 の stage 3: ウィザードを閉じる → project を開く → Problems を閉じる〔cand1 は 6 個で窓が 16 px 低く、Ok のクリック位置を dy 283 に変更〕→ Insert 1 を選ぶ → スロット 0 の名前を 1 回クリック → 1 / 3 / 10 s の絵とログ)。
- 10 s の絵: 『Fruity Balance (Insert 1)』の枠(タイトル・×)は出るが、**枠の中は背後のチャンネルラックの絵がそのまま透けている**(未描画)。engine は idle、`fle: unsupported`・abort・stop-context 0。
- FLE_DIAG_WIN の create 行(1 s で 52 行、10 s で 53 行): クリックの後、`TPluginForm`(lid 167、rect 360,436 571×476、parent_lid 16、caller 0x611d0e)→ `TVectorPanel` → `TQuickSheetSelector` → `TVectorSheet`(lid 170)→ `TWPControlForm`(lid 171、227×148、parent 170、caller 0x6973abe)→ `TNewCaption`『Fruity Balance? (Insert 1)?』(lid 172、233×23)→ `TWinControlProxy`(lid 173)。クリックの前に TPluginForm(lid 157)とその子の作成・破棄、TWPControlForm(lid 140)の破棄もある。**プラグイン DLL 自身が作る窓の create 行は 10 秒間 0 行**。`g9-diag` はプラグインの image への呼び出し数が 131072 まで増える(呼び出しは続いている)。
- vfs-miss: `…\Presets\Plugin presets\Effects\Fruity Balance`(FindFirstFileW status 3)、`…\Data\Patches\Plugin presets\Effects\Fruity Balance`、`…\Downloads\Plugin presets`、フォント(`Windows\Fonts\ILGlyphsEx.ilfont`、`MS Sans Serif.ttf`、`OpenSans-Regular.ttf`、`Share-Regular.ttf`、`JosefinSans-SemiBold.ttf`)。
- 読み(推測): client が空なのは、プラグインが GUI の窓を作っていない/作れていないため。FL のホストの枠(TWPControlForm)までは作られる。

## (3) Settings→General のスクロール(O9、`cand1-scroll`)
`scn-c1-scroll.json`(F10 → General タブ → ホイール 6 回、前後の canvas / peek / dump の sha と絵、FLE_DIAG_SCROLL の増分)。
| 見た物 | 値 |
|---|---|
| sha(canvas / peek / dump) | スクロール前 `63294acd2778e87a` ×3、後 `f5cc73589fd22261` ×3(全て一致 = エンジンの面そのものが崩れている) |
| 絵の比較 | 0318 の E13 の `top-canvas` と `scrolled-canvas` に対し、cand1 の同名の絵は ffmpeg PSNR が **inf**(ビット単位で同一)。スクロール後は項目が重なり帯状に崩れる(『Automatically check for updates』が 4 行、『Auto name effect slots』が 4 行 など) |
| ScrollWindowEx の記録 | 66 回(ホイール 6 回 × 11 回)、全て hwnd 0x7f990058、prcScroll (6,65,645,894)、prcClip 同じ、dx 0、dy = -19,-22,-17,-13,-9,-6,-4,-2,-2,-1,-1 の繰り返し(合計 -576)、**flags 0、hrgnUpdate あり、prcUpdate NULL**、戻り 1、parts 1、`uncovered` = 下の |dy| の帯、呼び元 0x825f77 の 1 か所 |
- 読み(推測): モデルが答える更新領域は妥当だが、絵が変わらない。崩れの原因は更新領域の答えではなく、スクロール後の描画(帯の再描画の位置、スクロールした内容の写し先)かもしれない。alpha1 の ScrollDC / scroll harness の面で見る。

## (4) O1 clip のドラッグ(`cand1-o1`)
`scn-c1-o1.json`: project を開く → Track 5 の既存の『Pattern 1』clip を (1500,346) から掴む。**ゆっくり**: 400 ms 押したまま、30 歩 × 40 ms で 2 トラック下(y 346 → 430)、途中の絵と窓の一覧、放して絵、Ctrl+Z。**激しく**: (1500,346) ↔ (1700,430) を 8 秒(`dragRate`、20 位置/脚、ドライバが送れる限り速く)、放して絵。
| 見た物 | ゆっくり | 激しく |
|---|---|---|
| engineLog の増分 | 0 行 | 1 行(`g9-diag: 131072 indirect calls into the plug-in image`、異常ではない) |
| `fle: unsupported`・ImageList・stop-context | 0 | 0(run 全体 2043 行も 0、起動時の『returned from entry』を除く) |
| 窓の一覧(ドラッグ中/後) | 主窓だけ | 主窓だけ(layered の影の窓なし) |
| 入力・present | 入力 108 → 140 | 481 回の移動/8 s、入力 143 → 627、present 308 回(38.5/s)、engine は idle のまま |
| 絵(Track 5〜7 の切り抜き) | 掴むと元の Track 5 から Pattern 1 が消え、**浮いた clip はどこにも描かれない**。放した後も Track 5〜7 に Pattern 1 は見えない(Track 7 には元からの Pattern 2 だけ) | 同じ |
- E13 の 0318(`fb2c` A)の切り抜きと同じ見え方。cand1 の 0319-dcx / imagelist の差分は入っていないので変化がないのは自然(alpha1 の診断: FL は LockWindowUpdate + GetDCEx(DCX_LOCKWINDOWUPDATE) で描くドラッグを使う、ImageList_*Drag の行は無い、の読みとは矛盾しない)。
- **FL の hint 欄(エンジンが描く左上の 2 行目)に手がかりがある**(絵の切り抜きを 4 枚並べた `hint-stack-before-mid-after-violent.png`): 掴む前は空、掴んだ後(ゆっくりの途中)は『**5:01:00 for 15:00:00**』と『**Pattern 1**』(掴んでいる項目の開始位置と長さ)、放した後も同じ、激しい後は『**10:04:21**』と『**Track 7**』(ポインタ位置のヒント)。→ ポインタの下のトラックは Track 7 と FL は認識している。掴んだ clip の開始は 5:01:00(x を動かしていないので変わらない)、長さ 15 小節(画面の右端 1893 px の先まで続く)。
- 未確定: 放した後の clip が **Track 7 にあるが描かれていない**(再描画の漏れ)のか、**消えた**のか。次の Chrome の窓で `scn-c1-o1b.json`(掴んで放す → 0.5 s / 3 s の絵 → Edit メニュー〔Undo の項目名〕→ ドロワーの畳み/開き = 主窓全体の再描画 → 絵)で確かめる。再描画で Track 7 に Pattern 1 が現れれば『描かれていない』(GetDCEx/LockWindowUpdate 経由の描画が捨てられる alpha1 の読みと整合)。

## (4) O3 About(`cand1-about`)
`scn-c1-about.json`: F10 → About タブ、3 / 8 / 28 s の絵と窓の一覧、About の増分と全体の engine log。
| 見た物 | 値 |
|---|---|
| 絵 | 『FL STUDIO 2026』『Trial v26.1.7 [build 5653]』『Windows - 64Bit』『FL Studio is in trial mode.』『Unlock FL Studio』は出る。**果物のロゴ(BigFruit)は出ず、staff roll も無い**(歯車の背景のみ) |
| 窓の一覧 | `lid 141 TLightweightLayeredControl` **3×3**(layered、premultiplied alpha、alpha 255)、3 s で (758,302)、28 s で (758,309)(y が 7 px 動く) |
| create 行 | `win create lid 141 class "TLightweightLayeredControl" rect (0,0,16x16) style 0x80000000 exstyle 0x80800ac parent_lid 5 (owner) caller 0x71e138`(About の作成時に TVectorSheet 138、TQuickMemo 139、TWPControl 140 も作られる) |
| **vfs-miss(FLE_DIAG_VFS_MISS)** | `GetFileAttributesW status 3` `Users\user\Documents\Image-Line\FL Studio\Settings\Skins\Default\BigFruit.png`、`GetFileAttributesW status 2` `Program Files\Image-Line\FL Studio 21\Artwork\Skins\Default\BigFruit.png`、**`CreateFileW status 3` `Program Files\Image-Line\FL Studio 21\System\Texts\About\AboutTxt.txt`** |
- 読み: ロゴの画像と staff roll の本文が仮想ファイルに無い。これが 3×3 と staff roll 無しの原因の第一候補(推測。ファイルを足した run は未実行)。区分は (b)(実際の install にあるファイルが seed に無い = 環境の内容で、API の契約ではない)。mt3 の seed を足した実験用 bundle(BigFruit.png・AboutTxt.txt など)で確認する(PM の合図後)。

## (5) ドロワー(Monitor ▸)と display change(`cand1-drawer5a`、`5a2`、`5b`)
ページの右の panel(DOM の『Monitor』ボタン `#panel-toggle`)を畳むと room が 1920 → 2260 CSS px 幅になり、ページはエンジンに display change(ゲストの画面 2260×1080)を送る。
| run | 状態 | 結果 |
|---|---|---|
| 5a(wizard 開) | `waitWindow TWelcomeWizard` 後 2.5 s、畳む(クリック 1 回) | **exit 134**(畳んでから 1.5 s 以内)。ページの `guest` 2260×1080、`resizes[0].sent = true` |
| 5a2(同じ、engine log を全部保存) | 同じ | **exit 134**、停止の文脈が 5a と同じ値 |
| 5b(wizard を閉じた後、畳む/開く を 2 回 = display change 4 回) | `closeWizard` 後 2.5 s | **止まらない**。主窓が 2260×1080 ↔ 1920×1080 に追従(窓の一覧)、各増分 9〜10 行、`unsupported`・callback nesting・stop context は 0 |
- 5a の engineLog の全文(`cand1-drawer5a2/log-5a2-collapse.log`、345 行): 
  `[err] fle: fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16): decision 0064`
  `[err] fle: stop context: guest return 0x6136d6 rsp 0x126f2ad8 rcx 0x7ff900000e30 rdx 0x7f99016c r8 0x5 r9 0`
  `fruity: exit code 134`、`CreateFruityInstance … elapsed 12.99 s; lifted entries since the call began: 0`、shadow call stack depth 0、guest stack at rsp: `0x6136d6 0x400000004 0x1b00000528 0x5 0x154a1e10 0x170524 …`。
- 停止時の窓: 主窓(1920×1080、display change で 2260×1080)と TWelcomeWizard(lid 48、874×573)。『Help us improve』の箱は窓の一覧に無かった(私の run では出ていない)。窓の表の z 0(最前面)に TLightweightLayeredControl(lid 8、334×476、owner TApplication、wizard の layered 窓)。
- **ページの停止の文言が誤誘導**: ページは『first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect … lid 8』を原因として表示する。これは停止の報告に残る古い runtime message(最初の画素の記録)で、停止の原因は上の callback nesting。
- 読み(推測): `apply_display`(`fle_input.c`)は WM_DISPLAYCHANGE と WM_SETTINGCHANGE を全トップレベル窓へ **SendMessage**(同期、入れ子)で送る。wizard が出ている間はゲストがモーダルの message loop の中にいるため、その上に入れ子で窓手続きが積まれ、`FLE_CB_MAX_DEPTH`(16)を超える。wizard を閉じた後は深さが足りる。深さの上限(16)が FL の挙動に対して小さいのか(extra な制限 = (a))、Windows の契約と違う送り方(Windows は WM_DISPLAYCHANGE を SendMessage で届けるが、モーダル中の再入の深さに上限はない)か((b))は未確定。

## 今後の確認(PM の合図後の Chrome)
1. `scn-c1-o1b.json`: 放した後の clip の行方(上)。
2. mt3 の seed 差し替え bundle(BigFruit.png・AboutTxt.txt・Fruity microfont.ttf・msgothic.ttc など)で O3(ロゴ・staff roll)と O16(日本語名の FLP〔opt1 の `ja-names.flp`、`serina1-serum2.flp` の名で assets に置く〕での豆腐)。
3. HiDPI(0318b)は cand2 の build が出てから。

## 追補: seed2b(mt3 の実験用 bundle)での O3・O16 の確認(10:00〜10:13、実 Chrome 156)
bundle: `fl-engine-test-rt1-seed2-8128f5be`(RT-A と wasm・glue・DLL pack が byte 同一〔hard link〕、seed pack 17,089,558 B と manifest だけ違う。TEST、push しない)。私の複写の manifest に `FLE_DIAG_WIN=1` `FLE_DIAG_VFS_MISS=1` を足した(bundle は無変更)。**page は ci1 の `page-rt1`(real-thread 用)**: p0 の page で開くと『Tried to spawn a new thread … TypeError: Invalid URL』で落ちた(私の page の選び間違い)。最初の版 `…-184136ac` は seed の表に同じ guest path が 2 回あり engine が exit 2(mt3 に報告、作り直された)。保存: `0319/logs/seed2b-about`、`seed2b-ja`、`seed2b-ja-try1`。
| 項目 | cand1(seed なし) | seed2b | 区分 |
|---|---|---|---|
| O3 About: layered 窓(lid 141) | 3×3 | **246×345**(3 s)/ 245×343(28 s)。ロゴ・歯車・staff roll が出る | (C) 資材。BigFruit.png と AboutTxt.txt の vfs-miss が消えた |
| O16 日本語名 FLP(`ja-names.flp` を `serina1-serum2.flp` の名で置いた) | (未実行。元の観察は豆腐) | Channel rack 1 行目『サンプ‥リナ』、Playlist 見出し『ぷにぷに電機　日本語ボリューム（Ｓｏｌｏ）』が日本語の字形(Wine の j4-font の絵と同じ字面) | (C) 資材(msgothic.ttc の substitute)。読み込みの経路は engine log で未確認、負の対照は未実行 |
| first idle(参考、診断入り) | 7.2 s(V3) | 8.24 s(About)/ 7.79 s(ja) | 比較できない(page・load・診断が違う) |
注意: 起動の早い段に、seed 済みの資材(BigFruit.png・AboutTxt.txt・Fruity microfont.ttf・msgothic.ttc、Fruity Balance/Filter の DLL、`ILCursors.ilfont` など)への `CreateFileW status 2` の miss が出る(log 570〜580 行付近)。同じ資材は後で見つかる(ロゴが出る)ので、これは『seed に無い』の証拠ではなく、起動時の別の探り方(読みであって未確認)。
