# ci1 → PM 報告6a: alpha5（df0512b0 = 4f054ecd）の「FillRect hDC is not a DC」で止まらない確認（短報・先出し）

## 先頭の事実（実エンジン。代役・再生なし）
- df0512b0 と同一バイトの局所再ビルド（c8: wasm sha256 `4ca5ba4eeca5…`、CI 実行 42013617447 と一致）を Chrome 156 と Firefox 158 で実行。シーン: 歓迎ウィザード表示 4 s と 16 s、F10 の設定窓、メイン窓の最小化ボタン、最小化後に空のデスクトップをクリック。
- **どの時点も エンジンは engine-idle のまま、停止なし。エンジンログ 310 行・ページログに `is not a DC|hDC` は 0 件**（Chrome / Firefox とも、5 つの時点すべて）。
- **ただし、TEdgeBrowser（lid 108 の窓）はこの経路では一度も窓表に現れていない**（ウィザード表示中の窓表は `TFruityLoopsMainForm` と `TWelcomeWizard` の 2 件だけ、4 s と 16 s の両方）。つまり alpha5 が直した経路（TEdgeBrowser への `RedrawWindow(RDW_ERASENOW)`）は、このヘッドレス・オフラインの環境では**発火させられていない**。「止まらない」は『この経路では停止が起きなかった』であり、alpha5 の修正そのものの実エンジンでの確認ではない（未確認）。
- メイン窓の最小化ボタン（1863,20）を押すと、デスクトップは黒になり、ブラウザから復元する手段がページにない（Alt+Enter・クリックでは戻らない）。最小化→復元の往復は試験できない（製品の隙間: 最小化後に戻せない）。
- 到達の数え方: TEdgeBrowser の窓が窓表に出る条件（オンラインの歓迎ページなど）を作れない限り、この項目は『未確認』のまま。

## 数
| 項目 | Chrome | Firefox |
|---|---|---|
| 第一アイドル（このシーン） | 8.2 s（負荷 15〜25 の下） | 7.8 s |
| 停止 | なし | なし |
| `is not a DC|hDC`（エンジン 310 行 + ページ） | 0 | 0 |
| ウィザード中の窓表 | MainForm + WelcomeWizard（TEdgeBrowser なし） | 同左（Chrome の窓表のみ記録） |

成果物: `df0512b0/{a5-chrome,a5-firefox,a5w-chrome}-report.json`、`scn-a5w.json`。残り（11 シーン・File メニュー×100・フェード・picker・Firefox 再読込・beacon）の報告6本体は 0260 の合間に出す（Chrome の 11 シーン・File メニュー 100/100・フェード 3 本は ok、詳細は本報告で）。

## 自己申告の逸脱
- この報告の Firefox の窓表は記録していない（Chrome のみ）。「同左」は未確認なので上表では Chrome の値のみ確かな値。
