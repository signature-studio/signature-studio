# 0193: エンジンの File > Open / Export(Win32 の共通ダイアログ)を、ブラウザの実ページで**利用者が選んだローカルのファイル**に橋渡しした — 実エンジンが選択されたフォルダ(FLP + 音声)を開き(題名 'Sampler.flp - FL Studio 2026'・Sampler 窓に波形)、Ctrl+R の書き出しが作った WAV(353,904 B、sha256 f5c6a08b…)がページの Download でネイティブ(決定 0188)とバイト一致で取り出せた。ただし出荷構成(B2D_SWAP)では書き出しの Start でエンジンが止まる。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0193、担当 fpick1、2026-11-03 20:44〜、予算 4 時間)。バンドルのビルドと実エンジンのテストは HEAD **8c10f48** の `git archive` した木(`lift/out-fpick1/src/`)で行い、HEAD が **f2dded6**(決定 0189、ship1)に進んだので、その `git archive` に載せ直した木(`lift/out-fpick1/src2/`)で最終のテスト(全体・a4・vitest・e2e・Worker・diff の dry-run)をやり直した。`fpick1.diff` は f2dded6 の `git archive` に `patch -p1 --dry-run` が当たる(`logs/patch-dry-run.txt`、20 ファイル clean)。runtime は 8c10f48 → f2dded6 で無変更なので、バンドルは f2dded6 の runtime と同じ入力。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝名前つき(`S-fpick1-…`)で §7 と `tables/s-choices.tsv` に代替案つきで書いた。**実機 Windows・依頼者の機械・実際のファイル選択ダイアログとの照合は 0 件**。速度の数字は、この機械(16 コア、SwiftShader、他の担当が同時に走り load 5〜29)の壁時計で、依頼者の機械の値ではない。

## 冒頭の三点(正直さ)

1. **実行した本物と範囲(実測)**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → wasm にした物。出荷候補 SHIPH〔ship1、決定 0189〕と**同じ F1xH 構成の「ハイブリッド無し」版**。生成 C は決定 0171/0178 の行〔ship1 の `ship-imports.txt`〕+ **決定 0188 の層 4 の行〔`exp1-imports.txt`〕**で再生成。runtime を `-DFLE_FILEDLG_BROWSER` でコンパイルし、`fle_filedlg_lib.js` をリンクした。`fle-lift` 無変更)を、隔離した Chrome 156.0.8078.47(headless、この機械)の実ページ(自分の `vite preview`、ポートは毎回別。PM の 5173 と `lift/out-websrv` は読み取りだけ)で動かした。起動 8.1 s で idle(SHIPH は 8.2 s。同じ機械・負荷の下の 1 回ずつ)。**模擬(テストの入力。利用者の操作ではない)**: (a) エンジンのメニュー File (18,20)・Open... (48,85)・Ctrl+R・Start (1135,832)・ウィザードの No thanks/Enter のクリックとキー(CDP `Input.dispatchMouseEvent`/`dispatchKeyEvent`。ブラウザ自身が作る trusted イベントで、人の操作の代わり)、(b) **ファイルの選択**: ページの `<input type=file>` への trusted クリック → Chrome のファイル選択窓を `Page.setInterceptFileChooserDialog` で横取り(`Page.fileChooserOpened`)→ `DOM.setFileInputFiles` でテストのファイルを与えた。**Playwright は未インストール**で、これは Playwright の `filechooser` イベントと `setInputFiles` が内部で使う CDP の 2 メソッドと同じ。(c) 保存名は箱の既定(エンジンの提案)をそのまま確定。**入力ファイル**: `Sampler.flp`(81,831 B、sha256 251076a8…)と `SHP_Sawing Synth.wav`(sha256 918e76a0…)は、決定 0178 の native テストが使った物と同じ sha(`vfs-seed-dlg1-g8.tsv`)。代役・モック・画像の再生・事前生成は無い。画像は実エンジンの面を Chrome が撮った物。
2. **結論**:
   (a) **ページの箱 → エンジン**: File > Open をクリックすると、エンジンは JSPI で `IFileOpenDialog::Show` に停止し、ページがエンジンへ転送したマウスアップの **6.3 ms 後**に要求がページに届く(`window.__app.fileDlg.requests[].msSinceInput`。テストのスクリプトがクリックを始めてからは 120〜125 ms)。ページは「これはページでありエンジンではない。あなたが選ぶ。アップロードしない」と書いた箱を出し、利用者が箱の `<input type=file>`(または「プロジェクトのフォルダ」の `<input webkitdirectory>`)をクリックして選ぶ。選ばれた `File` は Worker に渡され、**Worker のライブラリが `File.stream()` を 1 チャンクずつエンジンの malloc ブロックへ直接読み**(JS 側に 2 つ目のコピーを作らない)、C が `fle_vfs_put_file` でそのブロックを仮想ボリュームの `C:\Users\user\Downloads\<名前>` に**所有権ごと移して**、`Show()` がそのパスを返す。
   (b) **Chrome 156 の transient activation(実測)**: `input.click()` は活性化が無いと黙って無効(chooser 0)、`showOpenFilePicker()` は SecurityError 'Must be handling a user gesture'、信頼されたクリックの後 `navigator.userActivation.isActive` は **5.0 s** で false(4,751 ms は true・5,000 ms は false)、Worker の往復を 0.3/2/4 s 挟んだ `input.click()` は開き 6.5 s では開かない。→ 箱の中の input を利用者がクリックする設計にした(その時点で必ず活性化がある)。エンジンの要求がクリック(マウスアップ)から数 ms で届いたので自動で開く方式も間に合うが、5 s を超えると必ず失敗するため採らない。
   (c) **単体の FLP**: `Sampler.flp` 単体(81,831 B)を選ばせると、エンジンは FLP を読み(テンポ 120.000、Sampler チャンネル)、**エンジン自身の窓「Problems loading the project」**(TMissingPluginsForm)を出し、**その窓の Ok の後でエンジンが `user32!DispatchMessageA` の行の無い import thunk で止まる**(`fle: unsupported instruction at 0x4249b0`。呼び元 0x86a1fb、閉じたフォームの窓宛の投稿メッセージ。§5 の F2)。FLP が `%FLStudioFactoryData%\Data\Patches\Packs\Shapes\Misc\SHP_Sawing Synth.wav`(FL の工場データの音声)を参照し、出荷 seed の仮想ボリュームに無いため(読み)。**FLP と音声を入れたフォルダ**(2 ファイル、90,167 B)を選ばせると、**1.78 s 後に主窓の題名が 'Sampler.flp - FL Studio 2026'**、Sampler 窓に `File SHP_Sawing Synth.wav` と波形(`tables/shots/open-folder-after.png`)。エンジンが FLP と同じフォルダの同名ファイルを探す動作は決定 0178 の記録どおり。**ネイティブ dlg1 の面との画素比較は、ネイティブの面が保存されていないため**(§3.2)**できなかった**(代わりに決定 0188 の半分の大きさのネイティブ画像と粗く比べ、平均差 2.8/255・97.7 % の画素が差 24 以内で同じ配置・内容、画素一致は言えない。§3.2)。題名・Sampler 窓の内容・テンポは決定 0178 の記述と一致する(読み)。
   (d) **Export**: Ctrl+R → ページの Save 箱(エンジンの提案 `Sampler.wav`、要求はキーの 16.4 ms 後)→ 名前を確定 → エンジンが自分の窓 `TWAVRenderForm` を出し、Start → **0.9〜1.6 s で WAV が完成**。ページが「Files the engine wrote」に出し、**Download ボタンでブラウザが保存したファイルは 353,904 B、sha256 `f5c6a08b1dede642f91026e26939ca99cbea9e20f21bef0c2742c2d5dc3f6d8a` = ネイティブ(決定 0188 の表、全 64 桁一致)とバイト一致**。ただし**出荷構成のままでは Start でエンジンが止まる**(`compOp 0x11` = `BL_COMP_OP_OVERLAY`、仮想関数番号 `0x35` = `fillGeometry`、幾何は PATH。§4): `fle: b2d1 (decision 0160): upstream no-JIT pipeline returned BL_ERROR_NOT_IMPLEMENTED ... compOp, BLContextVirt slot 0x11 0x35`(Blend2D の置換〔B2D_SWAP〕の厳格停止、決定 0167)で SIGABRT(exit 134)。上の WAV は、テスト用の manifest に `B2D2_STRICT=0`(アダプターが環境変数で受ける。実行時に停止を切る。出荷バンドルの既定は STRICT のまま)を足して取った(§4、PM の判断点)。
   (e) **大きいファイル**: 250 MiB(262,144,000 B)のファイルが **0.73 s** でエンジンのメモリに読み込まれた(wasm メモリ 977,469,440 → 1,216,479,232 B。load 20 台の機械)。FLP ではないのでエンジンが自分の Error 窓を出し、エンジンは生きている。
   (f) **プライバシー**: ファイルは Worker のメモリにだけ置かれる。ビーコン(POST /__perf)28 件のどれにもファイル名・パス・`.flp`/`.wav`・`Downloads`/`Documents` の語は無い(`grep -c` = 0)。エンジンの stderr の行は件数だけでファイル名を出さない。ネットログの非ループバック TCP = 0。
   (g) **既存の動作は不変**: 既定の生成 C(FLEngine `fe-lift lift ... --pdata`)sha256 **cd9107ce…、215,811,678 B 不変**。全体テスト **173 results / 2,232 passed / 0 failed**(HEAD f2dded6 + 本決定。基準 172/2,226 + `wall_fpick1` の 6 件 + 1 result)、SKIPPED-lines 1。a4 FRUITY 無効 942/942・895/895・5,310・1,009,731 不変(§6)。app(HEAD f2dded6 + 本決定): vitest **1065 passed + 7 skipped**(HEAD の素の 1,011 + 本決定 54)、typecheck rc=0、Worker **43/43**、e2e **961/961**(基準 952 + スタンドインの確認 3 target × 3 場面 = 9)。変異検査: C 側 14/14、TS 側 14/14 を検出。
3. **未確認・未着手**: 依頼者の機械・実際の OS のファイル選択ダイアログ(0 件。テストは CDP が選択窓を横取りして答えた)。Firefox/Safari(JSPI が無いので対象外)。**ZIP を選ぶ経路**(エンジンの内蔵展開は DelZip64.dll〔未 lift〕を要り、ページ側展開は設計だけ。§3.3)。**Serum 2 を含む参照プロジェクトを開くこと**(プラグインが無い。本決定の範囲外)。ネイティブ dlg1 との面の画素比較。出荷物(ハイブリッド SHIPH)に本決定の変更を入れたバンドル(ハイブリッド版の再生成は未実施。§7 の S-fpick1-no-hybrid)。Save の確認ダイアログ(エンジンの Buy FL Studio 試用版のダイアログ)は、このテストの Export では**出なかった**。複数の書き出し(2 回目のダイアログ)・MP3/OGG/M4A(未 lift)。モバイルのブラウザ。

## 1. 設計メモ(まず。実装前に書いた比較を、実測で直した形で残す)

### 1.1 (a) 同期的なゲストの `Show()` とページの選択窓をどう橋渡しするか

エンジンは Worker の中で、`IFileOpenDialog::Show` を window proc の中から同期的に呼ぶ。選択肢:

| 案 | 内容 | 長所 | 短所 | 判断 |
|---|---|---|---|---|
| A. **JSPI で Show() を一時停止** | `Show` が async import(`WebAssembly.Suspending`)を呼び、Worker のイベントループがページと話す。ページの答えで Promise が解け Show が戻る | **エンジンは既に idle で同じ仕組み**(`fle_idle_wait`、決定 0136)を使っており、Show→window proc→メッセージループ→main が全部 wasm なので suspendable(実測: 動いた)。SharedArrayBuffer も Atomics.wait も要らない。ページのメインスレッドを止めない | Chrome 系の JSPI が前提(エンジンが既に前提) | **採用** |
| B. Atomics.wait でページの答えを待つ | Worker を SAB 上で同期ブロック | JSPI 不要 | Worker のイベントループが止まり、ページから Worker への postMessage(`File` の受け渡し)を読めない。ページが SAB に書く経路を別に作る必要。COOP/COEP は配信済み(`app/shared/headers.ts`、実測 crossOriginIsolated = true)なので技術的には可能だが、File は SAB に載らない(全バイトのコピーが要る) | 不採用 |
| C. Show を『キャンセル』で返し、非同期に開き直す | 最初の Show は cancel、ページで選んだ後に WM_COPYDATA(決定 0144)でエンジンに開かせる | エンジンの Show を止めない | 『エンジンが自分のダイアログの結果として開く』という本来の経路(OnFileOk、GetResult、Save の GetResult→書き込み)を通らない。Save は結果のパスが Show の戻りで要るので成り立たない | 不採用 |

**ユーザー操作(transient activation)の制約(実測、`tables/activation.json`、`tools/activation-probe.mjs`、Chrome 156.0.8078.47 headless、クリックは CDP の trusted イベント、選択窓は `Page.setInterceptFileChooserDialog` で横取り)**:

| 実験 | 結果 |
|---|---|
| E1: 活性化なしで `input.click()` | 呼びは戻る(例外なし)が chooser は開かない(0 件)。コンソールに何も出ない(黙って無効) |
| E2: 活性化なしで `showOpenFilePicker()` | `SecurityError: Must be handling a user gesture to show a file picker.` |
| E3: trusted クリックのハンドラ内で `showOpenFilePicker()` | 開く(横取りされて AbortError。chooser 1 件) |
| E4: trusted pointerup の後の `navigator.userActivation.isActive` | 4,751 ms まで true、**5,000 ms で false**(250 ms 刻み、1 回) |
| E6: trusted クリック → Worker が 0.3 / 2 / 4 / 6.5 s 後に返事 → ページが `input.click()` | 0.3 / 2 / 4 s は開く(chooser 1)、**6.5 s は開かない**(chooser 0)。`input.click()` は活性化を消費する(呼びの直後 `isActive` = false) |
| E7: 同じ流れで `showOpenFilePicker()` | 0.3 s は開く、6.5 s は SecurityError |
| E8: 7 秒何もしなかった後、ページの input そのものへの trusted クリック | 開く(selectSingle) |

→ **エンジンの要求が届くまでの時間は、実測で、ページがエンジンへ最後の入力を転送してから 6.3 ms(File > Open のマウスアップ)・16.4 ms(Save の Ctrl+R。`requests[].msSinceInput`、`tables/engine-requests.json`)、テストのスクリプトがクリックを始めてからは 120〜125 ms**なので、要求の到着時点では活性化が残っており(`activationActive: true`)、自動で選択窓を開く方式も**この機械では**間に合う。しかし 5 s を超える(エンジンが重い計算の最中・遅い機械・ネットワーク越しの Worker の起動など)と必ず失敗し、失敗は黙って起きる。→ **常にページの箱を出し、箱の中の `<input type=file>` を利用者が自分でクリックする**(その時点で活性化は必ずある。E8)。`showOpenFilePicker` は使わない(フォルダの記憶や書き込みハンドルが要る Save では案 §1.4 を参照)。箱は『これはページでありエンジンではない』と明記し、ファイルがアップロードされないことを書く。

**エンジンが止まっている間**: Worker は Show の中で止まり、ページからの入力は Worker の IdleDriver に溜まる(engine は idle ではないので注入されない)。再開後に最初の idle が来るまでの仮想時計の遅れは、決定 0170 の `S-clk-maxlag` の上限で切られる(キャップの警告が出る)。**実測**: 要求から答えまでの待ち時間は利用者次第(テストでは 0.1〜数秒)で、その間の wall-clock は仮想時計に入らない。待ち時間を性能の数字(P2 の入力遅延)に混ぜないこと(`busyMs` に Show の待ちが入る。§5 の F5)。

### 1.2 (b) 選んだファイルを仮想ファイルシステムのどこに置くか、大きいファイル

- **パス**: `C:\Users\user\Downloads\<サニタイズした名前>`(単体)、フォルダ選択は `C:\Users\user\Downloads\<フォルダ名>\<相対パス>`(S-fpick1-open-folder。ファイル選択は名前しか教えない、フォルダも OS のパスは教えない。決定 0178 §8 の『Downloads』を踏襲し、サンプルは FLP と同じフォルダの同名ファイルが見つかる〔0178 の実測〕)。名前は Windows の規則(`< > : " | * ? / \`・制御文字・末尾の空白とドット・`CON` などの装置名・255 UTF-16 単位・全体 259)に合わせてページが整え(`sanitizeComponent`)、C 側(`fle_vfs.c put_name_ok`/`put_walk`)がもう一度検査して、通らなければ**停止せずキャンセル**(利用者の入力はゲストの誤りではない。S-fpick1-bad-answer)。
- **メモリ**: `File.stream()` の各チャンクを、JS 側のライブラリが `_malloc(size)` で取ったエンジンのヒープへ直接書く(**JS 側に全体のコピーを作らない**。ArrayBuffer の 2 重化なし)。C は `fle_vfs_put_file` でそのブロックを**所有権ごと**ノードに採用する(コピー 0)。上限は仮想ボリュームの `MAX_FILE_BYTES` = **256 MiB**(決定 0018)に合わせ、全体 384 MiB・512 ファイル(S-fpick1-total。エンジンの wasm メモリは最大 2 GiB で約 1 GB を使う)。**実測(`runs/big1`)**: 250 MiB(262,144,000 B)の乱数ファイル(FLP ではない)を選ばせると、箱が閉じるまで **0.73 s**(Reading の表示は 0.1 s 刻みで 57 → 104 → 146 → 179 → 223 MiB)、wasm メモリ 977,469,440 → 1,216,479,232 B(+239 MB)、エンジンは自分の Error 窓を出して生きている。この時は `B2D2_STRICT=0` の manifest(§4)なので Error 窓の文字が描かれない。数百 MB の FLP を実際に読ませるテストは未実施。
- **エンジンの `ReadFile`** は VFS のノードからエンジンのバッファへもう一度コピーする(エンジン自身の動作)ので、実際のピークはファイルの大きさの約 2 倍(読み)。

### 1.3 (c) プロジェクト(FLP + 別の音声)を開く案、参照プロジェクトの静的な読み

**静的な読み(`~/ダウンロード/serina1-serum2.zip` の FLP、Python で読んだだけ。エンジンは動かしていない)**:
- zip は 49 エントリ(FLP 1 + 音声 48)、圧縮は deflate 45 / stored 4、暗号化 0、Zip64 0、最大のメンバー 6,766,929 B、zip 内の合計 53,482,387 B、展開後 62,747,002 B。
- FLP は音声を **48 個すべて絶対パスの 1 つのフォルダ**で持つ: `C:\Users\cabbage\AppData\Local\Temp\Image-Line\76728-{9BABC52F-…}\{6952ECA0-…}\<名前>`(UTF-16 で 48 個、基底名 48 個がすべて zip の根にある)。その機械・その時刻にしか無いパス。エンジンは最初に絶対パスを探し、無ければ FLP のフォルダの同名ファイルを探す(決定 0178 の実測)ので、**FLP と 48 個の音声を同じフォルダに置けば解決する**(絶対パスをそのまま作る必要はない)。
- **zip のエントリ名は UTF-8 フラグ(0x800)なし、日本語名 2 個は Shift-JIS 系のバイト列に `#$3099` という ASCII の列(濁点 U+3099 の代わりの記法)が混ざり、標準の `shift_jis` ではデコードできない**。FLP 側の名前は正しい UTF-16(『クリスマスセリナ\u3000メモリアルロビー\u3000高画質 - ぶるぶるあーかいぶ - 115bpm - Cmin (consolidated).wav』と `_2.wav`)。→ ページ側で zip を展開する案は、**名前の復元が要る**(CP932 でデコード → `#$3099` を U+3099 に → NFC。デコードできない所は FLP が持つ名前の一覧から突き合わせる)。名前の復元に失敗した音声は『見つからないサンプル』の警告になる(エンジンの動作)。

| 案 | 内容 | 費用(読み) | 判断 |
|---|---|---|---|
| P1. 1 ファイル(FLP だけ) | 本決定の実装 | 完了 | 音声が別にあるプロジェクトは『Problems loading the project』(§4.3) |
| P2. **フォルダ選択**(`<input webkitdirectory>`、本決定で実装) | FLP と音声を同じフォルダ(`Downloads\<フォルダ名>\…`)に置く | 完了(§4.2 で実測)。依頼者が zip を事前に展開する手間が要る | 採用 |
| P3. **ページ側で zip を展開**(`DecompressionStream('deflate-raw')` + 中央ディレクトリの自前の読み。45 deflate + 4 stored、Zip64 なし) | zip を選ばせ、ページが展開して P2 と同じ配置にする。名前の復元つき | 約 150〜250 行 + テスト。メモリ: zip(53 MB)+展開 62.7 MB。名前の復元は参照プロジェクト 1 個の実測しか無い | 次の担当への仕様。実装しない |
| P4. エンジンの内蔵展開 | `.zip` を選び、エンジンが `DelZip64.dll` で展開 | DelZip64.dll(362,808 B、MSVC、export `DZ_Exec` ほか)の lift・模型 | 費用大。P3 のほうが小さい |
| P5. File System Access の `showDirectoryPicker` | フォルダ全体の読み取りハンドルを持ち、エンジンが読む時に遅延読み込み | ReadFile を JSPI で JS に出す大きな変更 | 今回は不要 |

### 1.4 (d) Save / 書き出しの WAV をブラウザの保存に渡す

エンジンの Save ダイアログは**ファイルがまだ無い**時点で結果のパスを返し、エンジンはその後に書く(書き出し窓の Start でレンダリング)。→ **ページの Save 箱はファイル名を決めさせ**(エンジンの提案・既定拡張子を反映)、`Show()` は `<エンジンが指定したフォルダ>\<名前>`(無ければ `C:\Users\user\Documents`)の仮想パスを返し、C が**フォルダを作って**(`FOS_PATHMUSTEXIST` を満たす)そのパスを追跡する。エンジンが書き終えたこと(開いているハンドルが無い・サイズと最終書き込み時刻が 2 回の poll で同じ)を、Worker が**エンジンの idle ごとに** `fle_filedlg_saved_poll()`(runtime の関数。KEEPALIVE で wasm の export)で調べ、完了したファイルを**コピーして**ページへ `saved-file` として送る(ページは『Files the engine wrote』に出す)。**ダウンロードは利用者のボタン**(`Blob` の URL と `<a download>`。利用者のクリックなので活性化の問題がない)。

| 案 | 判断 |
|---|---|
| `showSaveFilePicker`(Chromium のみ。書き込みハンドルを先に取り、エンジンの書き込みを流す) | 活性化が要り(§1.1)、WAV はレンダリングが 1 回のクリックの処理で完結する(決定 0188)ので、先にハンドルを取る利点が小さい。不採用。大きい書き出しや MP3 などの将来の案 |
| **Blob のダウンロード**(採用) | 全ブラウザ。保存先の選択はブラウザの設定に従う |

**実測(`runs/final-folder`)**: Ctrl+R(キー)→ 16.4 ms 後に Save 箱(エンジンの提案 `Sampler.wav`)→ `TWAVRenderForm` → Start → 0.9〜1.6 s で `saved-file` → Download(CDP の `Page.setDownloadBehavior` で保存先を指定)→ ブラウザが保存したファイル **353,904 B、sha256 f5c6a08b…f6d8a**(ネイティブの値と全桁一致)。保存先のパスはエンジンが SetFolder で指定したプロジェクトのフォルダ(`C:\Users\user\Downloads\Sampler-project\Sampler.wav`)。

### 1.5 (e) セキュリティ(ファイルをサーバーに送らない)

- ファイルの経路は `File`(ページ)→ Worker(`postMessage` で `File` を構造化複製。コピーでなく参照)→ `File.stream()` → エンジンの wasm ヒープ。**どこにも fetch/XHR/WebSocket/sendBeacon を使わない**(CSP は `connect-src 'self'`、`app/shared/headers.ts`。ページは外部の origin を持たない)。
- 計測ビーコン(`POST /__perf`、決定 0152)の中身は `app/src/perf/beacon.ts` が組み立てる固定の項目(ブラウザ情報・時刻・統計)で、ファイル名・パス・エンジンのログの行を含まない。**実測**: 28 件のビーコン(Open と Save を行った実行を含む)に `Sampler`/`Downloads`/`Documents`/`SHP_`/`.flp`/`.wav` の語は 0 件(`logs/privacy-grep.txt`)。ネットログ: 非ループバックの TCP 接続 0(UDP の接続はブラウザ自身の外向き確認で、決定 0150 の e2e と同じ)。
- エンジンの stderr に出す行は『選択はページの利用者が行った(N 件)。ファイルはこのタブのメモリにあり、アップロードされない』で、ファイル名を出さない(`FLE_FILEDLG_TRACE=1` を付けたときだけ従来どおりパスを印字する)。
- `app.fileDlg.requests` には**エンジン自身の文字列**(種類・題・提案名・フォルダ)が残る。利用者のファイル名がここに現れる場合がある(Save の提案名・フォルダはエンジンが開いたプロジェクトの名前に由来する)。ページのメモリにだけある診断用で、ビーコンやログには載らない。

## 2. 実装(opt-in。既存の既定動作は変えない)

| 場所 | 内容 |
|---|---|
| `lift/runtime/fle_filedlg.c`(+178 行、`-` 0) | `#ifdef FLE_FILEDLG_BROWSER` の中だけ: `browser_picker`(`Show()` の既定の選択者。**ネイティブのハーネスの環境変数 `FLE_FILEDLG_PICK*`/`SAVE*` が設定されていればそちらが勝つ**)、要求の JSON(save・options・filterIndex・title・okLabel・fileName・defaultExt・folder・filters)、答えの解釈、`fle_filedlg_saved_poll()`(KEEPALIVE の export)。フラグが無ければ従来の `picked = picker != NULL ? picker(...) : env_picker(...)` の 1 行がそのまま残る(`#else`) |
| `lift/runtime/fle_vfs.c/.h`(+95/+7、`-` 0) | `fle_vfs_put_file`(パスを検査し、ディレクトリを作り、ホストが `malloc` したブロックを**所有権ごと**ノードに採用。256 MiB 超・既存のディレクトリ・開いているハンドル・ファイルを通るパスは戻り値 4/2/3/5、不正なパスは 1。**ゲストの呼び出しでは届かない**)、`fle_vfs_make_dirs`、`fle_vfs_file_state` |
| `lift/harness/web/filedlg/fle_filedlg_lib.js`(+51/−20) | 決定 0178 の設計スケッチの置換: JSPI の async import `fle_filedlg_host_pick`。`Module.fleFileDlg(id, request)` で Worker に要求を出し、`Module.fleFileDlgAnswer(id, answer)` で答えを受け、`File.stream()` を**エンジンの malloc ブロックへ直接**読み、`OK`/`R`/`E`/`S` の行からなる文字列を C に返す。失敗時は読んだブロックを解放して `CANCEL` |
| `lift/harness/web/filedlg/fle_filedlg_web.c`(削除、−74) | 決定 0178 の下書き(`emcc -c` が通るだけで実行されなかった)。C 側は `fle_filedlg.c` の中に入れたので不要 |
| `app/src/engine/filedlg.ts`(新) | DOM を持たない部分: 要求/答えの型、ゲストパスの規則(C 側と同じ)、`planOpenFile`/`planOpenFolder`/`planSave`、`parseRequest` |
| `app/src/ui/filedlg.ts`(新) | ページの箱(`FileDialogView`)と『Files the engine wrote』(`SavedFilesView`) |
| `app/src/engine/{protocol,fruity,worker,client}.ts`、`app/src/main.ts`、`app/index.html`、`style.css` | 要求/答え/進捗/`saved-file` のメッセージ、`FruityEngine.answerFileDlg`/`pollSaved`、Worker のハンドラ(idle ごとに poll)、箱の配線、エンジンが終わった時の箱の撤去。**スタンドイン(`surface-table-v1`)は要求を出さず、`fileDlg` が無いホストはエンジンの要求を即キャンセルで答える**(エンジンを待たせない)。箱は `z-index: 2147483000`(エンジンの窓の canvas は 1001 から上の値を取るため、これより下だと選択ボタンが canvas の下に隠れた〔実測の失敗〕) |
| テスト | `lift/crates/fe-lift/tests/wall_fpick1.rs`(6 件)、`app/test/unit/filedlg.test.ts`(54 件)、`app/test/e2e/run.mjs`(スタンインの確認 1 行)、`app/test/e2e/file-dialog-engine.mjs`(実エンジンの Open/Save。スイートには入れない)、`app/test/mutation/run.mjs`(+14 件) |

**ビルド(`app/scripts/build-engine-bundle.sh` は変更しない。MERGE-NOTES に手順)**: `RT_EXTRA_CFLAGS=-DFLE_FILEDLG_BROWSER`(`build_nmod_wasm.sh` が runtime のオブジェクトだけに渡す既存の変数)、`WEBLINK_EXTRA="--js-library <abs>/lift/harness/web/filedlg/fle_filedlg_lib.js"`(`stage_weblink` が `LINK_EXTRA` に渡す既存の変数)、`EXTRA_IMPORTS = ship-imports.txt + exp1-imports.txt の連結`(層 1〜4。層 4 は FLEngine と全モジュールの生成 C を再生成する)。**Open だけなら層 4 は不要**(Save の行が無ければ `CoCreateInstance(CLSID_FileSaveDialog)` は従来どおり `REGDB_E_CLASSNOTREG` で、Ctrl+R は何も開かない)。

**ビルドの実測(この機械、他の担当と並行、load 5〜25)**: gen 42 s、wasm の runtime 9 s・units 492 s(`JOBS=3`)、weblink 2 s、wasm-opt -Oz 262 s。`engine.b735730beb5c.wasm` 171,403,104 B(SHIPH 171,302,606 B。ハイブリッド無し + 層 4)、glue 186,726 B、seed pack・dll pack は SHIPH と同じ sha(`0f4b547e…`・`50206405…`)。`fle_filedlg_host_pick` は glue に含まれる。bundle は `COMPRESS=none`(brotli q11 の既定を避けた)。

## 3. 実測(実エンジン、隔離 Chrome 156、この機械)

### 3.1 基準線: 出荷候補 SHIPH のまま File > Open(ブリッジ無し)
SHIPH(`lift/out-websrv/assets/fl-engine-ship1-SHIPH`、読み取りのみ)で File > Open をクリック → **ページは一度も尋ねられず**(120 s 待った)、エンジンは何も開かない(題名は 'FL Studio 2026' のまま、`fileDlg.history` 空)。`runs/baseline-shiph/`。

### 3.2 Open(`runs/open2`〜`runs/final-*`、画像は `tables/shots/`)
| 操作(模擬) | 観測 |
|---|---|
| File (18,20) → Open... (48,85) をクリック | **ページがマウスアップを転送した 6.3 ms 後**(スクリプトがクリックを始めてから 120〜125 ms)にページが尋ねられる(`requests[0].activationActive = true`)。箱に『This is the page, not the engine ...』と OPEN、エンジンの種類 6 つ(FL Studio song file など。▶ が選択中)、file input、フォルダ input、Cancel |
| file input を trusted クリック → chooser(selectSingle)を横取り | `Page.fileChooserOpened` 1 件 |
| `Sampler.flp` 単体(81,831 B)を選択 | 約 3.9 s 後にエンジン自身の窓 **'Problems loading the project'**(`TMissingPluginsForm`)。エンジンの Sampler 窓は `File (none)`、チャンネルラックに Sampler、テンポ 120.000。Ok を押すと**エンジンが `unsupported instruction at 0x4249b0` で止まる**(§5 の F2) |
| フォルダ(`Sampler.flp` + `SHP_Sawing Synth.wav`、2 ファイル 90,167 B)を選択 | **1.78 s 後に主窓の題名が 'Sampler.flp - FL Studio 2026'**、Sampler 窓に `File SHP_Sawing Synth.wav` と波形、エンジンは idle。箱は消え、キーボードはデスクトップに戻る |
| 250 MiB の乱数ファイル(`big.flp`) | 箱が閉じるまで 0.73 s、wasm メモリ +239 MB、エンジンの Error 窓(§1.2) |

主窓の面の sha256 は実行ごとに違う(同じ操作の 4 実行で `009ee273…`・`650f19bd…`・`7fc89d0a…`・`a21900f7…`)。差は上部の小さなメーター(x≈1038・y 6〜18 の 13 画素など)で、`tools/pngdiff.mjs` で `open3` と `save4` の主窓の画像を比べると**差は 13 画素だけ**。**ネイティブ dlg1 の面(sha `3c444e7e…`)との画素比較はできなかった**: 決定 0178 の `shots/` は git に無く(空)、`lift/out-dlg1/runs/*/dump` は空で、ネイティブの面が残っていない。再生成するには layer 4 の native -O0 ビルドとハーネスの再実行が要る(今回は行わない)。

**粗い比較(画素一致の検定ではない)**: 決定 0188 のネイティブの面(`lift/out-exp1/tables/shots/main-after-export-sampler.png`、同じ手順 = Sampler.flp を開いて Ctrl+R→Start の後の主窓、**半分の大きさ**の 965×546 でしか保存されていない)と、ブラウザの同じ段階(`runs/final-folder/07-after-render.png`、デスクトップの 1920×1080 の切り出し)を、ブラウザ側を 2×2 の平均で半分にして最良の位置(サンプル 3, 3)で比べた(`tools/halfcmp.mjs`、`logs/halfsize-face-compare.txt`)。平均絶対差 **2.8 / 255**、画素の **87.1 %** が差 8 以内・**97.7 %** が差 24 以内(差が大きい画素は文字の行に集中。縮小方式の違いと読む)。→ **同じ配置・同じ内容の面を描いている**(Sampler 窓、波形、ミキサー、チャンネルラック、テンポ 120.000)が、画素単位の一致は言えない。題名・Sampler 窓の内容は 0178 の記述(題名 'Sampler.flp - FL Studio 2026'、波形)と一致する(読み)。

### 3.3 Export / Save(`runs/final-folder`)
Ctrl+R(キー)→ 16.4 ms 後にページの Save 箱('Save as'、エンジンの提案 `Sampler.wav`)→ 確定 → エンジンの `TWAVRenderForm`('Rendering to ?Sampler.wav'。`?` は題名の文字の 1 つが ASCII 外/制御文字のため、決定 0188 と同じ)→ Start (1135,832) → 0.9〜1.6 s で `saved-file` → Download → **353,904 B、sha256 `f5c6a08b1dede642f91026e26939ca99cbea9e20f21bef0c2742c2d5dc3f6d8a`**(RIFF/WAVE、非零バイト 303,731)。ネイティブ(決定 0188 の `cpu-brackets-o2.txt`: Sampler.flp → 353904 bytes sha256 f5c6a08b…f6d8a)と**全桁一致**。ディザ(MT19937 の seed)・仮想時計の違いは、このプロジェクトの WAV には影響しなかった(Dithering は既定で切)。**『正しい音』とは書かない**(実機の出力 0 件)。保存先は `C:\Users\user\Downloads\Sampler-project\Sampler.wav`(エンジンが SetFolder で指定したプロジェクトのフォルダ)。Save で出る『Buy FL Studio』の試用版ダイアログは**出なかった**(Export の経路。応答の偽装はしていない)。

## 4. 出荷構成の Export が止まる(重要な発見、PM の判断点)

出荷候補 SHIPH と同じ構成(Blend2D の置換 B2D_SWAP=1、決定 0167/0189)では、**Export の Start でエンジンが SIGABRT(exit 134)する**。ブリッジの不具合ではなく、Blend2D の置換アダプタの名前つき停止(`-DB2D2_STRICT_NOTIMPL=1`、決定 0167)に、エンジンの書き出し窓の描画が当たった。

| 項目 | 値(出典) |
|---|---|
| 停止メッセージ | `fle: b2d1 (decision 0160): upstream no-JIT pipeline returned BL_ERROR_NOT_IMPLEMENTED (a JIT-only composition operator): compOp, BLContextVirt slot 0x11 0x35`(`logs/engine-log-b2d-strict-stop.txt.gz` の 376 行。`lift/harness/b2d1/b2d_swap.cpp` の `die(what, a, b)` は `a b` の順に `%#llx %#llx` で印字し、`DRAW_RET` は `a = p->gs.compOp`、`b = slot` を渡す) |
| (a) `0x11` = **compOp = 17 = `BL_COMP_OP_OVERLAY`**(上流 `blend2d/context.h:278`。決定 0167 が『FL が静的に使う演算は SRC_OVER・SRC_COPY・OVERLAY のみ』と書いた OVERLAY) | 読み(ヘッダの値と印字の順を読んだ。印字の順は compOp、slot) |
| (a) `0x35` = **53 = アダプタが付けた仮想関数の番号で `fillGeometry`**(`b2d_swap.cpp:423` の `DRAW_RET(ri->virt->fillGeometry(...), 53)`)。BLCompOp の値ではない | 読み |
| 幾何の種類 | 停止時のレジスタ `rdx = 0x15` = 21 = `BL_GEOMETRY_TYPE_PATH`、`rcx = 0x1201d360`(コンテキスト)、`r8 = 0x120307a8`(BLPath を指す)= **OVERLAY で PATH を塗る呼び**(決定 0167 §の『swap1 の静的な手がかり: 線形グラデーション + OVERLAY + drawPath』と同じ型の呼びに見える。グラデーションかどうかは未確認) |
| (b) 呼び元 | 直接の戻り番地は Blend2D モジュール(基底 0x4700000)の中: ゲストのスタックの先頭 `0x4759321`(RVA 0x59321)、次の戻り番地 `0x4759a41`。**FLEngine 側の呼び元(読み)**: 出荷バンドルは関数入口を数えないので shadow call stack は空(`depth 0`)。そこで使い捨ての Worker のダンプ(停止後に、エンジンが印字した `rsp=0x13c0d548` からの 8 KiB を読む。`tools/scratch-worker-stack-dump.patch`。**差分には入れない**)で、FLEngine の範囲を指す語のうち **`call` の直後を指す物**だけを選んだ(`tools/stackmap.py`、objdump と .pdata。`tables/b2d-stop-stack-frames.txt` に 77 語)。最も近い FLEngine のフレームは関数 **`0x7f33a0`**(戻り番地 `0x7f3bbe` = `call [rsi+0x338]` の直後、仮想呼び出し)、続いて `0x6c71e0`・`0x8207b0`(`call [rbx+0x1a0]`)。つながって見える鎖は `0x806b70 → 0x8261a0 → 0x8207b0 → 仮想呼び出し`(戻り番地 `0x806b79`・`0x8268de`・`0x82088d`)で、その外側が `0x86a0f0`(`TApplication.ProcessMessage` の形。戻り番地 `0x86a13d`)→ `0x86a250` → `0x863600` → `0x809610` のメッセージ処理。**スタックの古い語が混ざりうる**ので鎖は読みで、関数に名前は付けていない(Delphi の RTTI のクラス名は未確認) |
| (b) 経路 | 停止は **Start のクリックの 0.4 s 後、書き出し窓 `TWAVRenderForm`(lid 174)が現れてから 2.2 s 後**。最後に画が変わった窓は lid 174(停止の 0.1 s 前)。停止時の画面(`tables/shots/b2d-strict-stop.png`)は、書き出し窓が出たまま、エンジンのツールバーに『Abort rendering』、書き出し窓の下部に進捗バーが出た状態。→ **書き出しの進捗表示/窓の再描画の経路**(読み。プレビューか進捗バーかは未確認) |
| (c) `B2D2_STRICT=0` のとき | アダプタは `rc = 5`(`BL_ERROR_NOT_IMPLEMENTED`)をそのままゲストに返す(`RET(rc_)`)。ゲストは戻り値を見ない(決定 0160/0167 の記述)ので、**その PATH の塗りが描かれないだけ**でエンジンは続く。`b2d_swap.cpp` のコメントによれば、置換前の lifted DLL も同じエラーを黙って返す(読み。本決定では再測定していない)。実測: STRICT=0 で書き出しは 0.9〜1.6 s で完了し、WAV は**ネイティブとバイト一致**(音声の経路は Blend2D と無関係)。画面に差が出る領域は未測定(STRICT と STRICT=0 の面の比較はしていない)。STRICT=0 の主窓の面は、書き出し後に書き出し窓が閉じて通常に戻った(`tables/shots/after-export.png`) |

- **テストの設定**: `manifest-nostrict.json` = 出荷 manifest に `engine.env.B2D2_STRICT = "0"` を 1 行足した物(`tools` の説明を参照)。**出荷バンドルの既定は STRICT のまま変えていない**(黙って描画を省くのは忠実性の逸脱。PM の指示どおり)。
- **PM の判断点**: 出荷の Export には Blend2D 側の未実装演算(OVERLAY の PATH 塗り)の実装が要る。対処は別の担当(決定 0167 の『swap1 の自前テンプレート `b2d-fixedpipe-plus-overlay.diff` を上流のビルドに足す』、実 DLL と画素一致を Wine で確認)へ。それまで、出荷構成で利用者が Export を押すと、エンジンは名前つきで停止する。

## 5. 見つかった壁・所見(一覧)

| ID | 所見 | 出典・状態 |
|---|---|---|
| F1 | 出荷構成の Export が OVERLAY の PATH 塗りで停止(§4) | 実測 |
| F2 | **`user32!DispatchMessageA` の行の無い import thunk**(単体 FLP の『Problems loading the project』の Ok の後) | 実測(`logs/engine-log-single-flp-dispatchmessagea.txt.gz`)。下の表 |
| F3 | 単体の `.flp` は FL 工場データの音声(`%FLStudioFactoryData%\Data\Patches\Packs\Shapes\Misc\...`)を参照するので、出荷 seed の仮想ボリュームに無いとエンジンが自分の『Problems loading the project』を出す。FLP と音声を入れたフォルダを選ばせれば開く | 実測。FL の工場データの供給は別の話(seed pack の大きさ) |
| F4 | zip のエントリ名は UTF-8 フラグなし・Shift-JIS 系 + `#$3099`(濁点の ASCII 記法)で標準デコーダが通らない | 静的な読み(§1.3) |
| F5 | Show の待ち時間(利用者が選ぶ間)が、エンジンの『idle ではない時間』(`busyMs`)に入り、再開後の仮想時計の遅れ(S-clk-maxlag の上限で切られる)になる | 実測: テストでは待ちが 0.1〜数秒。性能(P2)の数字に混ぜないこと |
| F6 | `app.fileDlg.requests` に残るエンジンの提案名・フォルダは利用者のファイル/フォルダ名に由来しうる(ページのメモリだけ) | 設計(§1.5) |

### F2: `DispatchMessageA` の壁

| 項目 | 内容 |
|---|---|
| 停止 | `fle: unsupported instruction at 0x4249b0`(`fruity: the stop is in the lifted thunk f_4249b0 ... = jmp [IAT slot] of import user32.dll!DispatchMessageA`)。到達: `TMissingPluginsForm`(lid 175)の Ok をクリックした約 0.2 s 後 |
| 呼び元 | **`0x86a1fb`: `call 0x4249b0`**(`objdump-text.txt`。この thunk を呼ぶ箇所は FLEngine にここだけ)。戻り番地 `0x86a200`(停止時のスタックの先頭)。関数は VCL の `TApplication.ProcessMessage`(Forms)の形(読み): フック → `IsKeyMsg`/`IsHintMsg`/`IsMDIMsg`/`IsDlgMsg` 相当(0x86cb80、0x869f00、0x869d70、0x869dd0、0x869d10)→ `TranslateMessage`(0x425640)→ `r13b`(= `IsWindowUnicode(msg.hwnd)`、0x425090 の結果)が真なら `DispatchMessageW`(0x4249c0)、**偽なら `DispatchMessageA`(0x4249b0)** |
| 停止時の MSG | `rcx = 0x13c138c0`: `hwnd = 0x7f9902bc`(= 0x7f990000 + 4 × 175 = **lid 175 = 今閉じた `TMissingPluginsForm`**)、`message = 0xB021`(WM_APP + 0x3021)、`wParam = lParam = 0`(停止時のダンプの 32 バイトの読み。`message` の名前は未確認) |
| 何が起きたか(読み) | 閉じた(破棄された)フォームの窓に、まだキューに残っていた投稿メッセージ。ランタイムの `IsWindowUnicode` は live でない窓に 0 を返すので、Delphi の分岐が `DispatchMessageA` を選ぶ。実機の Windows でも破棄された窓の `IsWindowUnicode` は 0、`DispatchMessage` は何もせず 0 を返す(読み。Learn の『hwnd が無効なら...』は未確認) |
| 最小の実装案(実装しない) | `user32.dll DispatchMessageA` の行を足し、`fle_imp_DispatchMessageA`: `hwnd` が 0 または live でない窓なら `R[0] = 0` で何も呼ばない。live な窓なら `fle_imp_DispatchMessageW` に渡す(この仮想環境の窓はすべて Unicode なので、Delphi が A を選ぶのは live でない時だけ)。`fle_win.c` の `fle_imp_DispatchMessageW`(1776 行)は `hwnd == 0` で既に 0 を返す。**live でない窓で W が `fle_stop` する点は A では 0 にする**。行は層 5 になる(response table が変わるので全モジュールの再生成) |

## 6. テストと基準(HEAD f2dded6 + 本決定の木、`lift/out-fpick1/src2/`。この機械、load 7〜29)

| 項目 | 結果(実測) | 基準・差 |
|---|---|---|
| ネイティブ `wall_fpick1`(新、6 件) | 6 passed | (1) フラグ有無の `nm`: 無しの目的ファイルは `fle_filedlg_host_pick` を参照せず `fle_filedlg_saved_poll` を持たない・有りは参照し持つ・3 つの VFS 関数は有りだけが使う、(2) ページの答えで選択ファイルと隣のファイルが実際の VFS に載り `Show` が S_OK で `GetResult`/`GetResults` が 1 件・リクエストの JSON(引用符・バックスラッシュ・U+3042 のエスケープ、フィルター 2 つ、`filterIndex`)、(3) 環境変数 `FLE_FILEDLG_PICK` が勝ちページは呼ばれない/未設定ならページが呼ばれキャンセルは 0x800704C7、(4) Save: 先のフォルダが作られ(属性 0x10)、開いている間・閉じた直後(1 回目の poll)は報告されず 2 回目で 1 回だけ(サイズ 5・内容 `hello`・UTF-8 のパス)、書き直すと 2 回の poll の後にまた 1 回、(5) 不正な答え 9 種(CANCEL・空・未知の役割・タブ欠け・`con`・D:・相対・256 MiB 超・Save のフォルダが不正)は**停止せず** 0x800704C7 を返しボリュームに何も載せない、(6) VFS 関数: 置換(大小無視)・空ファイル・ディレクトリ(2)・ファイルを通るパス(5)・256 MiB 超(4)・開いているハンドル(3)・不正な 18 種のパスがすべて 1・255/257 単位の成分 |
| 変異検査(ネイティブ側、`tools/mutate.py`、`logs/mutation-wall-fpick1.txt`) | **14/14 検出** | poll が開いたハンドルを報告する・poll が即報告する・同じ書き込みを繰り返し報告する・環境変数が勝たない・Save のフォルダを作らない・JSON が引用符を逃がさない・未知の役割を受ける・256 MiB の制限なし・開いたファイルを置換する・装置名を通す・末尾ドットを通す・ファイルを通って歩く・隣のファイルも結果にする・フラグ無しの目的ファイルがページを参照する |
| 全体テスト(`harness/test.sh full --jobs 2 --fresh`) | **results=173 passed=2232 failed=0 ignored=0 SKIPPED-lines=1** | 基準 172 / 2,226 / 0 / SKIPPED 1。差は `wall_fpick1` の 1 result・6 件。SKIPPED の 1 行は `wall_fr9t`(基準と同じ) |
| a4 FRUITY 無効(`a4-fruity.sh off`) | init_count **942/942**・run_rank **895/895**・functions **5,310**・entries **1,009,731**、harness exit 0 | 基準と一致(`logs/a4off.txt`、gen 17 s・build 145 s) |
| FLEngine の既定の生成 C | sha256 **cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50**、215,811,678 B | 基準と一致(`logs/felift-default.sha`)。`fe-lift` は無変更 |
| app: typecheck | 4 つとも rc=0 | |
| app: vitest | **1065 passed + 7 skipped**(rc 0) | 基準(HEAD f2dded6)+ 本決定 `filedlg.test.ts` 54 件。f2dded6 の素の vitest は 1,011 passed + 7 skipped(1,065 − 54) |
| app: 変異検査(unit 層) | **14/14 検出**(`filedlg-parse-accepts-anything` は同値の変異体〔`typeof` の検査を消しても `r.save` が未定義で null になる〕と分かり、`filedlg-parse-filter-shape-unchecked` に替えた) | |
| app: Worker(wrangler dev --local) | **43/43 passed** | 基準 43 |
| app: e2e(`node test/e2e/run.mjs`、dev・preview・worker の 3 target) | **961/961 checks passed**(rc 0) | 基準 952。差 +9 = スタンインの確認 1 チェック × 3 場面 × 3 target(『スタンインはファイルダイアログの要求を出さず、箱は隠れ、開いた・答えた・保存したの記録が 0』。スタンインの banner の検査はその前に従来どおり) |
| 実エンジン(スイート外、`file-dialog-engine.mjs`) | open+save 12/12(`runs/final-folder`)、cancel 4/4(`runs/cancel1`)(8c10f48 の木の app)。**f2dded6 の木の app でも再実行**: open+save 12/12(WAV の sha256 は同じ)・cancel 4/4(`tables/runs/v2-*-f2dded6.log`)。単体 FLP 6/7(最後の 1 件は §5 F2 の壁で題名が変わらない)、基準線(SHIPH)0/1(ページは尋ねられない) | |

## 7. 出典のない選択(S-fpick1-N、19 件。`tables/s-choices.tsv` に代替案と撤回条件)

`S-fpick1-input-box`(箱の中の input を利用者がクリックする。自動で開かない)・`-open-folder`(Downloads に置く)・`-limit-file`(256 MiB)・`-total`(384 MiB・512 ファイル)・`-name-sanitise`・`-result-flp`(フォルダの結果は上の階層の FLP)・`-extra-dropped`(パスが長いサンプルは落とす)・`-pickfolders`(キャンセル)・`-bad-answer`(停止せずキャンセル)・`-env-wins`(ハーネスの環境変数が先)・`-save-folder`・`-save-overwrite`(黙って上書き)・`-save-done`(完成の判定)・`-saved-copy`(完成したら即コピーして送る)・`-box-z`・`-log-names`(名前を stderr に出さない)・`-no-hybrid`(テストのバンドルはハイブリッド無し)・`-b2d-nonstrict`(Export のテストは `B2D2_STRICT=0`)・`-cdp-chooser`(Playwright が無いので CDP)。

## 8. 規則違反の自己申告・確認できなかったこと

- 存在しない `bash` ツールを **2 回**呼んだ(エラー)。`sleep 1` を **1 回**試して拒否された。
- `/tmp/opencode/fpick1/app` に e2e の作業を置いて tmpfs の割り当て超過(`-122`)で 1 回失敗し、`lift/out-fpick1/e2e-work` に移した(`/tmp/opencode/<名前>/` は 300 MB 以内の規則に対し、27 MB 程度だったが割り当てを超えた)。
- 並行の重い処理が**同時に 3〜4 個**になった(全体テスト `--jobs 2` + e2e + a4 の build 4 並列など)。load は最大 29。壁時計の速度の数字(起動 8.1 s・250 MiB の 0.73 s・Export の 0.9〜1.6 s)はこの load の下の値で、速度の主張ではない。
- 実エンジンのテストで、待ち時間の上限を長く取りすぎて約 20 分(Export が起きなかった 10 分 × 2)と約 10 分(単体 FLP の題名待ち)を無駄にした。結果には影響しない。
- brotli q11 の bundle ステージを途中で PID を指定して止め(`pkill -f` は使っていない)、`COMPRESS=none` でやり直した。
- `ps`/`kill` でプロセスを触ったのは自分のビルド/テストの PID だけ。PM の 5173 と `lift/out-websrv` は読み取りだけ(SHIPH の基準線の実行は、その assets を読み取りで配信した)。
- **確認できなかったこと**: 依頼者の機械・実際の OS のファイル選択ダイアログ・実機 Windows との照合(0 件)。ネイティブ dlg1 との面の画素比較(§3.2)。ハイブリッド SHIPH にこの変更を入れた物。ZIP の選択(§1.3 の P3)。複数ファイルの選択(`FOS_ALLOWMULTISELECT` をエンジンは使わなかった)。Firefox・Safari。Chrome の他の版。モバイル。音の正しさ(WAV はネイティブとバイト一致だが、実機との照合は 0)。MP3/OGG/M4A の書き出し(未 lift)。数百 MB の実 FLP。

## 9. 再現

`lift/out-fpick1/tools/`(`bb.sh` = バンドルのビルド、`fpick-e2e.mjs` = 実エンジンの Open/Save、`activation-probe.mjs` = §1.1 の実験、`mutate.py`、`pngdiff.mjs`、`mkdiff.sh`)。

```
# バンドル(層 1〜4 + ブラウザの橋)。ハイブリッド無し
cd lift/out-fpick1 && NAME=A4 IMP=$PWD/in/extra-imports-l4.txt bash tools/bb.sh gen resplit && NAME=A4 bash tools/bb.sh wasm weblink wasmopt seed && COMPRESS=none NAME=A4 bash tools/bb.sh bundle
# 実エンジンの Open + Export(隔離 Chrome、自分の vite preview)
cd src2/app && npx vite build && cd ../.. && node tools/fpick-e2e.mjs --assets build/A4/bundle --prefix fl-engine-fpick1-A4 --manifest manifest-nostrict.json --out runs/x --file inputs/Sampler-project --mode folder --steps open,save
```

## 10. 置き場(結果の同梱物)

- `fpick1.diff`(f2dded6 に当たる)、`MERGE-NOTES.md`、`docs/0193-browser-file-bridge.md`(本書)、`docs/README-0193-line.txt`。
- `tables/`: `activation.json`(§1.1 の実験の生データ)、`engine-requests.json`(実エンジンが出した Open/Save の要求の全文)、`saved-info.json`、`s-choices.tsv`、`runs/*.log`(各実行の記録。本文の `runs/<名前>` はこれ)、`shots/`(実エンジンの面の画像: `open-box.png`〔箱〕、`open-folder-after.png`〔Open の結果〕、`single-flp-problems-dialog.png`〔単体 FLP〕、`save-box.png`、`after-export.png`、`b2d-strict-stop.png`〔STRICT で止まった面〕)。
- `logs/`: `mutation-wall-fpick1.txt`・`mutation-app-filedlg.txt`、`summary-*`(全体テスト/e2e/Worker/vitest の要約)、`a4off.txt`、`felift-default.*`、`patch-dry-run.txt`、`privacy-grep.txt`、`engine-log-*.txt.gz`(停止した 2 実行のエンジンのログ全文)。
- `tools/`: `bb.sh`(バンドルのビルド)、`fpick-e2e.mjs`(実エンジンの Open/Save/cancel/bigopen)、`activation-probe.mjs`、`mutate.py`、`pngdiff.mjs`、`mkdiff.sh`。

## PM の判断と取り込み（2026-11-03）

- **取り込み範囲:** `fpick1.diff`（20 ファイル）。`-` 行 87 行を全部読んだ。削除は設計スケッチ（`fle_filedlg_web.c` の全削除と `fle_filedlg_lib.js` の置換）だけで、実装済みの既存行を消す変更は無い。0192 を先に取り込んだため、`app/src/style.css` と `app/test/mutation/run.mjs` の 2 か所が衝突した。0192 の内容を残したまま、fpick1 の追加行（CSS 12 行、変異 15 件）を末尾に手で足した（`.rej` は削除。`node --check` 通過）。
- **PM の実測（HEAD 79043d1 + 本差分）:** FLEngine の既定の生成 C は sha256 `cd9107ce…`（215,811,678 B）で不変。全体テスト 173 results / 2,233 passed / 0 failed（取り込み前 172 / 2,227、+ `wall_fpick1` 6）。a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,731（不変）。app: typecheck 通過、vitest 1209 件中 1,201 通過・7 スキップ・**1 失敗**（下記）、Worker 統合 43、e2e 1017/1017（取り込み前 1008）。
- **vitest の 1 件の失敗は環境の原因:** `pack-assets-fruity.test.ts`（`--standin` の名前と説明）が `UNKNOWN: unknown error, write` で失敗した。`/tmp`（tmpfs 3.7 GB）が 80% 埋まっていた（完了済みの作業者が残した `/tmp/hyb.dump` 1.5 GB と古い作業ディレクトリ）。PM が完了済みの作業者の残り物（`/tmp/hyb.dump`、`/tmp/opencode/{app0,app1,app4,web1}`）を消した後、同じテストは 9/9 通過した。本差分のコードの欠陥ではない。
- **採用:** (1) File>Open は、エンジンの `Show()` を JSPI で止め、ページが「これはページであり、エンジンではない。アップロードはしない」と明示した箱を出し、利用者が箱の中の入力欄を押して選ぶ形。自動で開かない（Chrome 156 の一時的なユーザー有効化は 5.0 秒）。(2) 選んだファイルはエンジンのヒープへ直接読み、`fle_vfs_put_file` で `C:\Users\user\Downloads` に置く。
- **限界（そのまま記録）:** メニュー・キー・ファイル選択は、CDP のテスト入力であり、利用者の操作ではない。実機の依頼者ブラウザでの確認は 0 件。ZIP の選択は設計だけ。単独の `Sampler.flp` は、エンジン自身の『Problems loading the project』（FL の工場データ wav が seed に無い）の後、`user32!DispatchMessageA` で名前付きの停止になる（呼び出し元 0x86a1fb、第 5 層の候補。未実装）。フォルダ（`Sampler.flp` + wav、90,167 B）は 1.78 秒で開く。左のブラウザ枠の黒い四角は ia1 以来の未調査の問題。
- **出荷構成の Export は止まる（重要）:** 検証用の manifest `B2D2_STRICT=0` では WAV 353,904 B がネイティブ（決定 0188）と byte 同一。出荷構成（Blend2D swap、STRICT 既定）では、Start で `BL_COMP_OP_OVERLAY` × PATH（`fillGeometry`、`b2d_swap.cpp:423`）の未実装により停止する（exit 134）。**既定は STRICT のまま**（描画の黙った省略は忠実性の逸脱）。b2d3（決定 0195）が対応中。
- **出荷構成への適用は別:** SHIPH を `-DFLE_FILEDLG_BROWSER` 付き、`EXTRA_IMPORTS` = `ship-imports.txt` + `exp1-imports.txt` で作り直す必要がある（既存の `RT_EXTRA_CFLAGS` / `WEBLINK_EXTRA` で足りる見込み。ビルドスクリプトの変更は不要）。作り直しと、混成との同一性（File>Open 後、FLP 読み込み後）の確認は、これからの作業。既定の入口はまだ変えない。
