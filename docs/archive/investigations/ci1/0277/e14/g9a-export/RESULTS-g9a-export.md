# G9-a test bundle の Export（Chrome 156、非計時・参考の 1 本）— ci1、2026-11-07 04:53〜04:58（machine date）

**事実（先頭）**: 実エンジン（mt3 の test bundle `fl-engine-test-g9a-28f6f631` = Balance を実モジュールにした物。製品の bundle ではなく、R2 に無い）を実 Chrome 156 で動かした 1 本。page は gpu1 の port 4969（main の app）、assets は `lift/out-gpu2/assets-g9a`。Export の scenario は ci1 の `scn-export.json` に engine log の保存（`saveLog`）を足した `scn-export-g9a.json`。この機械、headless、模擬入力。依頼者の機械は 0 件。開始前: 他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export は 0 本、`vmstat` の最後の 4 サンプルの si/so は 0、`wait-quiet` の swap の入出力 0/0（Mem available 16.9 GB、Swap 使用 3.0 GB）、load 0.52、I/O 圧 0。first idle は測っていない。

| 判定 | 結果 |
|---|---|
| (1) 完走 | **完走**（scenario ok〔no stop〕、310 s。Start から WAV まで約 74 s〔file の時刻 04:57:10 → 04:58:24〕。同じ手順の P + V7 の page + E13 の run は 319 s、約 80 s〔load 5.2→3.0〕） |
| (2) `fle: unsupported` / abort | `fle: unsupported` は **0 行**（engine log 318 行）。`abort kind: (not a runtime message)` の 1 行は、アプリ起動前の `== stop: returned from entry` の標準の要約に含まれる文字列（function entries 0、`a4: init_count=0 of ?`）で abort ではない。E13 の engine log は保存していないので、同じ形かの比較は未実施 |
| (3) WAV | サイズ **22,863,828 B（基準と同じ）**、フレーム数 **5,715,911（基準と同じ）**、ヘッダ 44 byte は同一。**sha256 `94c9366687076a410f48c6fbe1485e8e22f6b528592fcc5ceba2d665d1554090` は基準 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` と違う** |
| (4) g9-diag | **2 行**: `LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`、`GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0` |

## 基準（P + V7 の page + E13 の本番 bundle、同じ手順、sha `381cdb5a…f979`、`out/export/export.wav`）との WAV の差（numpy、再計算ではなく今回の比較）
- 差のあるフレーム: **640,995 / 5,715,911（11.21%）**。最初の差は **25.79 s**（フレーム 1,137,368）、最後は 119.78 s。**0〜20 s、90〜100 s、120〜129 s はビット一致**。
- 10 s 区間別の差の RMS（差のあるフレームの割合）: 20〜30 s 4,350（9.7%）、30〜40 s 6,802（22.9%）、40〜50 s 4,511（15.3%）、50〜60 s 3,928（18.9%）、60〜70 s 3,637（17.8%）、70〜80 s 3,566（18.2%）、80〜90 s 1,618（3.5%）、100〜110 s 5,584（15.9%）、110〜120 s 6,730（23.2%）。
- 全体: 差の RMS 3,987（L 4,189・R 3,773）、信号の RMS 20,413 に対し −14.2 dB。最大差 65,534（|差| > 32,768 の標本が 31,585 個 = 符号が反転した標本を含む）。ピークは両方 32,767。差のあるフレームの |g9a| / |基準| の平均は 0.998（レベルはほぼ同じ）、相関は L 0.842・R 0.873。
- **読み（未確認）**: Balance の実モジュールが呼ばれて出力が変わった（読み込みの g9-diag 2 行と sha の変化が根拠）。ただし変化が**大きい**（−14.2 dB、符号の反転を含む）うえレベルが変わらず相関が下がるので、単純な利得・パンの変化ではなく、波形の中身（遅れ・位相・クリップ付近の挙動など）が変わった可能性がある。**正しい変化かどうかを判定する参照が無い**（Wine の FL は Balance を含む全プラグインを読むので、Balance だけの差は取れない）。遅れか内容の変化かの切り分けは未実施。
- **未確認**: この変化が Balance の GUI/処理の正しさを示すか、Balance が参照プロジェクトのどのトラックに入っているか、E13 の本番で同じ run が 381cdb5a…f979 を再現する条件の再確認（今回の基準は 03:51 の P + V7 page の run の WAV）。

置き場所: `lift/out-git/ci1-results/0277/e14/g9a-export/`（`export.json`、`report.json/md`、`engine-log.log`、画像）。WAV 本体は `lift/out-ci1/kit/out/export-g9a/export.wav`（22.9 MB）。

## 追記（決定 0314d の作業中。alpha1 の解析〔0310 §6、PM が取り込み済み 8c4b72a6〕との突き合わせ）
上の『読み（未確認）』のうち「波形の中身（遅れ・位相・クリップ付近の挙動など）が変わった可能性」は、alpha1 の解析で次のように説明された（私の測定ではなく alpha1 の numpy の実測と FLP の読み。sha256 は alpha1 と PM も `sha256sum` で確認）: 差のあるフレーム 640,995 のうち 626,143 は Balance の自動化 clip 256 個（Insert 1）の内側、14,852 は各 clip の終端から 0〜137 フレーム（最大 3.11 ms）後の尾で、clip の無い区間の差は 0。基準の WAV は 30.83% のフレームに ±32767 の標本があり、符号の反転（最大差 65,534）は 16 ビットの整数化の clip の効果と読める。**よって WAV どうしの rms・最大差・相関は Balance の利得の正しさの尺度にならない**（私の『遅れ・位相』の可能性は取り下げ、利得の自動化 + clip で説明できる）。変化が正しいかは Wine の参照（Balance のみ有効の Export）との照合で判定する（未実施）。
