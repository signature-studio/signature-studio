# 0318(仮): 依頼者の観察 O1〜O9 のブラウザでの再現と原因の切り分け(gpu1/gpu8、2026-11-07 07:00〜)

状態: **仮**(実測 + 読み)。作業者 gpu1(成果物名 gpu8)。依頼: PM(`docs/feedback/2026-11-07-owner-observations.md` の O1〜O9)。**再現と観測のみ。実装・build はしていない。** 依頼者の機械の値は 0 件(依頼者の定性的な観察は我々の測定ではない)。時間は参考値(他の job が走る窓で、swap が満杯近い)。

## 冒頭の三点
1. **実エンジンで何を実行し何を観測したか(実測)**: 実 Chrome 156 headless と実 Firefox 158(DISPLAY=:77、llvmpipe)で、**製品の E13 bundle**(`fl-engine-ci-0363b050`、wasm 無変更、協調)と**main の page**(p0、display method D)を、依頼者の観察に近い条件(Monitor を畳む、project を開く、Settings → General/About、playlist のドラッグ)で動かし、page の状態(`window.__app`)、engine log、画面の画素、canvas・peek(エンジンの compositor)・dump(窓表から描き直し)の sha を取った。代役・再生・モックなし。依頼者の機械の値は 0 件。
2. **結論**: (a) **O9(General のスクロールで描画が壊れる)は Chrome と Firefox の両方で再現し、壊れた絵は page の描画ではなくエンジンの窓の面そのもの**(canvas・peek・dump の sha が一致)= 契約 (b) の側(ScrollWindowEx の画素モデル、または FL の再描画の更新矩形)。(b) **O3(About)**: 歯車は描かれ動く。ロゴの浮遊に当たる layered 窓が **4×3(Chrome)/3×3(Firefox)** で 1 枚あるだけで、ロゴとスタッフロールは出ない(engine log の増分は 0 行)。(c) **O5(Monitor を畳むとエンジンが止まる)は再現しなかった**: Chrome(5 条件)と Firefox で、畳むと page が guest 画面の大きさをエンジンへ送り、エンジンは追従して 30〜60 s engine-idle のまま、メモリも増えない。(d) 別件の発見: 診断経路の『非 peek の dump の 2 回目(画面の大きさ変更をまたぐ)』でエンジンが落ちる(2/2)。(e) 画面のぼやけ(O7)の機構の候補: dpr 1.75 で guest 画面が CSS px のまま(物理 px と一致しない)。(f) O1: playlist の既存の clip のドラッグは始まり、離すと移るが、**ドラッグ中にカーソルに付いて動く clip の絵が描かれない**(layered 窓も出ない)。engine は死ななかった。
3. **未取得**: 起動のフェードの present 時刻列(O2。`--page-init` の driver と sampler を用意、未実行)、コード進行ツール・Sampler の落ち(O13)、画面のぼやけの物理 px の測定(O7)、1 拍目(O13 の 8)の armCapture、Filter の V3(テスト bundle が参照プロジェクトを開く途中で止まる)。

## 1. 条件
- 共通: page = p0(`app-dist-p0` = main d0447902 の app、display method D)、port 4967、assets = `assets-e13`(ci1 の E13 bundle `fl-engine-ci-0363b050`)。driver = `tools/sdriveg2.mjs`(`--dpr` を足した、`pixel` の fallback を足した)、`tools/run-fb.sh`(Chrome)、`tools/run-ff.sh`(Firefox)。各 run の前に `tools/pre-run-note.sh`(free・vmstat の si/so)を `logs/memnote.txt` に残した。
- 参照プロジェクト `serina1-serum2.flp`。Problems 窓は `clickIn`(Ok の +370, +315)で閉じる。

## 2. O5: Monitor(右のドロワー)を畳む
- 仕組み(コードの読み): `panel-toggle` の click → `setPanel(false)` → body に `panel-off` → stage が広がる → `applyDisplay()` と `ResizeObserver` → `roomFollower.push(measureRoom())`(room が静まってから) → `app.screen.asked` を更新して engine へ『画面の大きさ』を送る(`engineFollows`)。エンジンは `FLE_MONITOR_W/H` の変更に従う(ログに `asked.env`)。
- 観測(`logs/fb1b`・`fb1c`・`fb1d`・`fb5a`・`fb5b`・`ff-o5`):

| 条件 | 畳む前 → 後の guest 画面 | 畳んだ後 | WebAssembly.Memory |
|---|---|---|---|
| Chrome 2260×1116 dpr 1、project なし | 1920×1080 → 2260×1080 | 30 s engine-idle、key を受ける、再度開くと戻る | — |
| Chrome 2260×1116、project 再生中 | 同上 | 30 s engine-idle(idles 増加) | — |
| Chrome 3400×1500 | 3060×1464 → 3400×1464 | 30 s engine-idle | — |
| Chrome 3840×2160 dpr 1 | 3500×2124 → 3840×2124 | 60 s engine-idle | 1,027 → 1,124 MB(+97 MB、その後一定) |
| Chrome 2194×1234 **dpr 1.75** | 1854×1198 → 2194×1198(CSS px) | 60 s engine-idle | 1,009 MB のまま |
| Firefox 158 2260×1030 | 1920×994 → 2260×994 | 30 s engine-idle、key を受ける | 1,015 MB のまま |

- **畳む操作だけでは再現しない**(Chrome と Firefox、6 条件)。依頼者の停止は、project の中身・FL の別の窓・操作の順など、この scenario に無い条件に依存する(未特定)。
- **別件の発見(診断経路)**: 非 peek の dump(`deskDumpBase64(false)` = 全窓を窓表から描き直す)を、**畳む前に 1 回呼び、畳んで 15 s 後にもう 1 回呼ぶと、エンジンが落ちる(2/2)**: `RuntimeError: memory access out of bounds`(fb1-drawer)、`Aborted(Assertion failed: at || own != __pthread_self()->tid && "pthread mutex deadlock detected", pthread_mutex_timedlock.c,95)`(fb5d)。畳んだ後の最初の dump(fb5c)や peek(fb5a・fb5b)は落ちない。依頼者が使う操作ではない(page の通常経路は dump を使わない)ので、依頼者の停止の原因とは言えない。区分: 診断経路の不具合(a/b の別は未調査)。

## 3. O9: Settings → General のスクロール
- 手順: F10(Settings)→ General タブ(772,138)→ 中央で wheel を 6 回(各 120)。Settings 窓は `TMIDIForm` 668×900。
- 結果: **Chrome(`logs/fb3b`・`fb3c`)と Firefox(`logs/ff-o9`)で同じ壊れ方を再現**: 行が重なり、縞、文字が横に潰れる。1.5 s 待っても直らない。
- **切り分け(決定的)**: 壊れた後の canvas(page が持つ画面)・peek(エンジンの compositor の出力)・dump(全窓を窓表から描き直した絵)の sha-256 が**一致**(Chrome `f5cc7358…`、Firefox `880e83de…`)。= 壊れているのは page の描画(dirty 矩形の取りこぼし等、区分 a)ではなく、**エンジンの窓の面の内容そのもの**。
- engine log の増分は 0 行(API の trace は無い)。契約の側の候補: `lift/runtime/fle_wall_ia2w.c` の `ScrollWindowEx`(root 窓の面の内側で bits を move する画素モデル、SW_SCROLLCHILDREN は stop、無効領域は 1 つの bounding rect)と、その後の FL の再描画の更新矩形。**読みは alpha1 の担当**。

## 4. O3: Settings → About
- Chrome(`logs/fb3b`)と Firefox(`logs/ff-o9`): About タブは描かれる(『FL STUDIO 2026』、Trial v26.1.7 [build 5653]、背景の歯車)。3 s と 8 s の間に 13,693 画素変化 = 歯車は回っている。
- **ロゴの浮遊に当たる layered 窓(`TLightweightLayeredControl`、premultiplied alpha 255)が 4×3(Chrome、位置 (755,307))/3×3(Firefox、(755,264))で 1 枚だけ**。スタッフロールは絵に無い。engine log の増分は 0 行(unsupported/abort なし)。
- 読み(未確認): ロゴの画像の復号(WIC/GDI+ 系の API)か資材が失敗して小さな既定の面が残った。FL が待っているのか資源が無いのかは、log に出ないので区別できない。

## 5. O1: Playlist のドラッグ
- **pattern 一覧からのドラッグ**(Chrome、`logs/fb2`): pattern 一覧の Pattern 3 を押して playlist の (1500,300) へ 30 step・40 ms 間隔では**ドロップされず Pattern 3 が選択されただけ**。8 s の激しい往復(dragRate、60 moves/s): **エンジンは死なない**。present **9.6/s**、dirty 矩形 **198/s**(1 present あたり約 20 矩形)、idle の回数 7.4/s(静止時 26.8/s)。
- **playlist 上の既存の clip を掴むドラッグ**(Chrome、`logs/fb2c`・`fb2d`、(C2)): 4 つの別の clip(Pattern 1、schnozz atmos pad、Synthion Downlifter 2、Synthion Clap Impact)を、4 種の gesture(押してすぐ動かす / 400 ms 押してから / 最初に 60 px 跳ぶ / 150 ms 押して 60 ms 刻み)で 40〜84 px 動かした。**どの gesture でもドラッグは始まり、離すと clip が新しい位置へ移る**。エンジンは engine-idle のまま、abort なし。
- **ドラッグ中の絵**(半分の step の時点、窓の一覧つき): clip は**ドラッグ前と同じ位置のまま(schnozz)、または元の位置から消えるだけ(Pattern 1)**で、**カーソルに付いて動く clip の絵は描かれない**。window 一覧は主窓 1 枚のまま(layered の ghost 窓が無い)。`logs/fb2d/crop-B-stack.png`(上から ドラッグ前・ドラッグ中・離した後): 前と中が同じで、後で clip が上へ移っている。= 依頼者の『慣性/重力のアニメと配置先の骨格』に当たる『ドラッグ中の見え方』が**出ていない**ことを再現。present は 1 ドラッグ(約 1.5 s)で約 25 回、矩形約 24。
- 読み(未検証): ドラッグ中の描画が layered 窓(`TLightweightLayeredControl`)か screen DC か、タイマー駆動の描画で、画面に出ていない。O3 のロゴ(layered 窓が 4×3)と同じ型かもしれない。engine log に何も出ないので、API は特定できない。

## 5b. Firefox 158 での再現((C1)、`logs/ff-o5`・`ff-o9`)
- O5: 畳んでも止まらない(30 s engine-idle、メモリ 1,015 MB のまま、key を受ける)。O9: Chrome と**同じ壊れ方を再現**(canvas・peek・dump の sha 一致 `880e83de…`)。O3: 歯車は動く、layered 窓は 3×3 でロゴ・スタッフロールは出ない。Firefox だけに出る差は、この 3 つでは無かった。

## 6. 画面のぼやけ(O7)の先取りの事実
- dpr 1.75 の viewport(2194×1234)で、guest 画面は **CSS px のまま**(1854×1198、`canvas-info` の『scale 1.0000, device pixel ratio 1.75』)。canvas の物理 px は guest と一致せず、ブラウザが 1.75 倍に引き伸ばす = ぼやけの機構の候補。ゲストには実 DPI も物理画面の大きさも見せていない(ゲストは 96 PPI)。設計メモは別途。

## 7. Filter の V3(テスト bundle `fl-engine-test-g9f-7b2e3014`)
- 2/2 本で、参照プロジェクトを開く途中に `fle: DestroyCursor: hIcon was not issued by this runtime's GDI table … the page defines no result for it` でエンジンが止まる(exit 134)。Balance の `LoadLibraryW → 0x6800000`・`GetProcAddress → 0x6abdde0` の後。Filter の LoadLibraryW の行は出ない。Problems 窓の件数は未取得。区分: 外部 API の契約の不足 (b)(DestroyCursor の戻り値の定義)。run = `logs/rt-g9f-1`・`rt-g9f-2`。

## 8. 区分(全体の仕組みの側)
| 項目 | (a) 余計な処理 / (b) 契約の不足 | 直す仕組み(提案、実装は PM の許可後) |
|---|---|---|
| O9 | (b)(エンジンの面が壊れている) | ScrollWindowEx 等の画素モデルの契約の網羅(SW_SCROLLCHILDREN・visible region・update region) |
| O3 | (b)(読み) | 画像の復号系 API(WIC/GDI+)の契約の棚卸し |
| Filter の stop | (b) | DestroyCursor を契約文書どおりに定義(import census の一部) |
| O7 | (b)/設計 | 実 DPI と物理画面の大きさをゲストに見せ、物理 px で描く(GetDeviceCaps/GetSystemMetrics) |
| O5 | 未再現 | — |

## 9. 自己申告
1. 07:29 に PM へ送った info の見出しの時刻を、実際の時刻(07:24)と違えて書いた。以後は `date` で自動で入れる。
2. 先の info で『peek で落ちる』と書いたが、落ちたのは**非 peek の dump の 2 回目**だった(§2で訂正済み、PM に訂正を送った)。
3. /tmp/opencode に私の作業物(variants、メッセージの本文)を置いていた。PM の指示で `~/work/gpu8/` へ移した(`tools/mkvariants.py` の base の場所も更新)。
4. 先の info(0318 の続報)を `date` と backtick を含む未クォートの heredoc で作り、shell が backtick の中身を実行して文面の一部が抜けた(『command not found』が出た)。訂正の info を送った。以後、メッセージの本文は引用符つきの heredoc で作る。


## PM の判断と取り込み(2026-11-07 08:10、docs のみ。製品の差分なし)

取り込み: gpu1 の 0318(依頼者の観察 O1・O3・O5・O9 の実 Chrome 156 / Firefox 158 での再現、非計時、E13)と HiDPI の設計メモ(`docs/investigations/gpu8/0318-hidpi-design-memo.md`)。要点(gpu1 の観測): O9(Settings→General のスクロールで描画が崩れる)は Chrome・Firefox の両方で再現し、canvas・peek・dump の sha が一致 = **崩れているのはエンジンの面そのもの**(ブラウザに依らない runtime の決定的な挙動)。O5(Monitor を畳む)は Chrome 4 条件・Firefox 1 条件で止まらない(別に、畳んだ後の 2 回目の dump が落ちる診断経路の不具合)。O1 は clip のドラッグは始まり離すと移るが、ドラッグ中にカーソルに付く clip の絵が描かれない。O3 は About の layered 窓が 4×3(Chrome)/3×3(Firefox)で小さいまま。判断は `docs/feedback/2026-11-07-owner-observations.md` に記した。依頼者の機械は 0 件。
