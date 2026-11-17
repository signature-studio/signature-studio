# 0313 V3 (gpu1): G9-a test bundle in Chrome — functional check (not a timing run)

状態: **仮**(実測 + 読み)。作業者 gpu1(gpu8)、2026-11-07 04:26〜04:35 JST。実 Chrome 156 headless、実エンジン。代役・再生・モックなし。依頼者の機械は 0 件。first idle・性能は結論に使わない(診断と +7.4 MB が混ざる)。

## 実行
- bundle: mt3 の `fl-engine-test-g9a-28f6f631`(wasm 150,851,575 B、sha256 dde53a6b…、診断 `g9-diag:` 入り)、assets = `assets-g9a`(`tools/mk-assets.sh g9a`)、page = p0(main d0447902 の app)、port 4969。対照 = E13 の `fl-engine-ci-0363b050`(assets-e13、port 4968)、同じ page・同じ scenario(`tools/scn-g9a.json`: ウィザードを閉じる → 参照プロジェクト serina1-serum2.flp を開く → Problems 窓を待つ → 窓の一覧・デスクトップ全体の画像・engine log を保存)。
- 開始前の記録(`logs/memnote.txt`): g9a: load 0.33、MemAvailable 13.5 GB、si/so = 0・0。対照: load 1.26、12.5 GB、si/so = 0・0(alpha1 の TEST build の wasm-opt が同時に走っていた = 計時ではない)。

## 結果(run = `logs/rt-g9a`、`logs/rt-g9a-ctl`)
| | G9-a テスト bundle | E13 の対照 |
|---|---|---|
| first idle まで | 到達(engine-idle、終了せず、opErrors 空) | 同じ |
| Problems 窓 | **7 plugin(s)**(Wrapper / FLEX / Frequency Shifter / Soundgoodizer / Fruity Parametric EQ 2 / Fruity Delay 2 / Fruity Filter)、窓 691×327 | **8 plugin(s)**(上の 7 + Fruity Balance)、窓 691×343 |
| engine log の行数 | 316 | 311 |
| `g9-diag: LoadLibraryW "…\Fruity Balance_x64.dll" -> 0x6800000` | 1 回 | 0 |
| `g9-diag: GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0` | 1 回 | 0 |
| `fle: unsupported instruction at` / onAbort / pruned / not supported | 0 行 | 0 行 |

- 数字と 16 進を除いた engine log の差は、G9-a 側にだけある 3 行(`real module image: …`、`real module table: name=fruity_balance_x64.dll …`、`vfs seed: …\Fruity Balance_x64.dll <- /data/seed/…`)と診断の 2 行 = +5 行だけ。他の [err] 行(act1-net、filedlg、G2/window table)は同じ。
- 対照の run に TQuickPopupMenuWindow(Copy のポップアップ)が 1 つ写っているが、G9-a の判定に関係しない(scenario のクリック位置が窓の大きさで 16 px ずれる影響、読み)。

## 限界
- Balance のプラグイン UI や process は scenario で踏んでいない(壁の本番は未踏)。合格は『LoadLibraryW が 0x6800000 を返し、CreatePlugInstance が 0x6abdde0 で解決され、Problems 窓の件数が 1 減る』まで。
- 各 n=1。first idle・音・性能は未測定。
