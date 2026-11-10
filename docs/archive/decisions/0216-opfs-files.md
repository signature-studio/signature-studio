# 0216: ブラウザの OPFS(Origin Private File System)を管理するエクスプローラー風の GUI をページに足した(段階 1 = ページだけ)— 隔離 Chrome 156 の実 OPFS で 60/60、Firefox 158(geckodriver・headless)で 59/59(うち 2 項目は模擬)。zip は依頼者の `serina1-serum2.zip` を 49 ファイル・62,747,002 B・全 SHA-256 一致で展開でき、**決定 0178/0193 の『CP932 + #$3099 の復元』ではこの zip の日本語名は復元できず、zip に入っている Info-ZIP Unicode Path 拡張フィールド(0x7075)だけが FLP の名前と一致する**ことが分かった。仮の決定

状態: **仮の決定(段階 1)**。依頼は PM(番号 0216、担当 opfs1、2026-11-04 03:50〜、総予算 5 時間、段階 1 の report は 60 分以内)。段階 2(エンジンの『開く/保存』を JS のエクスプローラーで)と段階 3(OPFS をゲストの実体に)はこの文書の §8 に設計メモだけを置き、**未着手**。作業は HEAD **e7af37b** の `git archive`(`lift/out-opfs1/src/`)。HEAD は 7c0e9cc まで `app/` への変更が無く、さらに d32e2f6 まで進んだ(`app/src/perf/*` などが変わった)。`opfs1-s1.diff`(19 ファイル、`-` の行 0)は **d32e2f6 の `git archive` に `patch -p1 --dry-run` で clean に当たる**(`app/src/main.ts` の hunk が 1 行ずれる〔offset 1〕だけ。`logs/patch-dry-run-s1.txt`)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや資料を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝`S-opfs1-…` として §7 と `tables/s-choices.tsv` に代替案つきで書いた。速度の数字は、この機械(16 コア、SwiftShader、他の担当が同時に走り load 40〜56)の壁時計で、依頼者の機械の値ではない。

## 冒頭の三点

1. **実行した本物と範囲(実測)**:
   - **隔離 Chrome 156.0.8078.47(headless、この機械)の実 OPFS**(モック無し)で、本番ビルドを `vite preview`(CSP `style-src 'self'` つき)で配信し、`app/test/e2e/files-opfs.mjs` が 60 項目を実行して **60/60**。地面の真実(OPFS の中身・SHA-256)は**ページ内でブラウザ自身の API**(`navigator.storage.getDirectory()` と `crypto.subtle`)で読み、テスト対象のコードを通さない。zip の中身の正解は Python の `zipfile` で読んだ SHA-256 と、FLP 内の UTF-16LE 検索。ネットログ(`logs/e2e-chrome-final.txt` の `INFO browser:` の行): ページの TCP 接続は 127.0.0.1 だけ(`non_loopback_tcp_connects=0`)。ブラウザ自身の背景通信の試み(accounts.google.com・update.googleapis.com の名前解決 9 件が `--host-resolver-rules` で失敗、UDP 接続 8 件・送信バイト 0)は、素の HEAD の既存 e2e のネットログにも同じ形で出る。ファイルの名前や内容がページから送られる経路は無い(このコードに `fetch`/XHR/`sendBeacon` は無い: `grep` 0 件)。
   - **Firefox 158.0(`/usr/bin/firefox` = snap、`-headless`)を `/snap/bin/geckodriver` 0.37.1 の WebDriver(classic、HTTP)で動かせた**(`firefox --version` は mount namespace のエラーを出さなかった。BiDi/Marionette の直接操作は試していない)。同じシナリオが **59/59**(Chrome より 1 項目少ないのは、OS からの本物のドロップを WebDriver で作れないため。下の模擬を見よ)。**依頼者の Firefox 159.0 / Windows 10 ではない**(Linux、158、headless)。
   - **実エンジンのページ**(既定の入口、パラメータは `?beacon=0` のみ、SHIPH の `lift/out-websrv/assets`)で、エンジンが最初の idle(emscripten-fruity-v1、wasm メモリ 977 MB)に達した後に Files 窓を開閉し、`files-with-engine.mjs` **6/6**(エンジンは止まらず idle を 3 回以上繰り返す、題名は 'FL Studio 2026' のまま、窓にタイプしたキーはエンジンの入力にならない〔ただしテストの前後とも `inputEvents` が 0 で、この項目は弱い〕)。起動 44 秒は load 52 の壁時計で、P1 の値ではない。
   - ライブラリの比較表(`tables/lib-comparison.tsv`)は、サイズを esbuild で**実際にバンドルして**測り(`tables/lib-bundle-sizes.txt`)、ライセンスは `npm view` と package.json で読み、MDN browser-compat-data(`raw.githubusercontent.com/mdn/browser-compat-data`、2026-11-04 に取得)で Chrome/Firefox の対応を読んだ。UI ライブラリのうち**ページで動かして試したものは 0**(README・npm メタデータ・ファイルサイズだけ。js-fileexplorer は clone してサイズを測った)。
   - **模擬(テストの入力。人の操作ではない)**: (a) ファイル選択: Chrome は CDP `DOM.setFileInputFiles`(ブラウザの選択窓の代わり。**フォルダ入力にはディレクトリのパスを渡し、Chrome が展開して `webkitRelativePath` が付く**)、Firefox は WebDriver の Element Send Keys。(b) **Firefox のフォルダ入力は模擬**: WebDriver はフォルダを選べず、送ったファイルに `webkitRelativePath` が付かないので、`webkitRelativePath` つきの `File` を入力の `files` に差し込んで `change` を起こした(ページのハンドラは本物、選択窓は無い)。(c) ドロップ: Chrome は CDP `Input.dispatchDragEvent`(**ファイルのパスにフォルダを含めて**、OS からのドロップに近い。フォルダの構造とバイトが一致)。両ブラウザで、`DataTransfer` に `File` を入れた `DragEvent` を発火する**模擬のドロップ**(zip を含む)も行った。**Firefox での本物のフォルダのドロップ(`webkitGetAsEntry` の走査)は未確認**。(d) クリックの多くは `element.click()` / `dispatchEvent`(信頼されたイベントではない。このページのコードは信頼を要求しない。ダウンロードは Chrome の `Page.setDownloadBehavior`、Firefox の prefs で保存先を指定)。(e) `navigator.storage.persist()` は Chrome が『許可しない』と答え、**Firefox は許可の確認を出す仕様のため headless では答えが得られない**(ページは『ブラウザに尋ねている』と表示する)。
2. **結論**: (a) OPFS を管理する窓(ツリー+一覧+パンくず、新規フォルダ、リネーム、削除〔確認つき〕、コピー/切り取り/貼り付け〔フォルダごと、衝突は 1 回だけ尋ねる〕、ファイルのアップロード〔選択とドロップ〕、フォルダのアップロード〔`<input webkitdirectory>` とドロップ〕、zip のアップロードと展開、ダウンロード〔単一ファイル、フォルダは zip〕、容量と永続化の表示)を `app/src/files/` に置いた。ヘッダーの『Files』ボタンで開くオーバーレイで、ページのレイアウトの枠は変えない。(b) 自作の UI(`app/src/files/` の TS が 1,387 行・CSS 39 行。`wc -l`)にした理由と比較は §2。依存は `fflate`(MIT、zip の書き出し)と `file-selector`(MIT、ドロップの走査)の 2 つ、ページの JS は gzip で **+19 KB**。(c) **zip の名前**: 依頼者の zip の日本語名 2 個は Shift-JIS 風のバイト列に `#$3099` が混ざる**欠けた**名前で、`TextDecoder('shift_jis')` + `#$XXXX` の置換では**本来の名前にならない**(『メモリアルロビー』や『質』が欠けている)。zip にはその 2 個に **0x7075(Info-ZIP Unicode Path)拡張フィールド**があり、その UTF-8 名が FLP の保持する名前と一致する。全 48 個の音声名を FLP から UTF-16LE で検索して 48/48 見つかった。→ 名前は『UTF-8 フラグ → 0x7075(CRC が合うとき)→ ASCII → CP932』の順で読み、**正規化(NFC)はしない**(FLP は濁点 U+3099 を別の文字で持つ)。(d) Chrome・Firefox のどちらでも、OPFS の読み書き・`createWritable`・`FileSystemHandle.move`・`removeEntry`・`DecompressionStream('deflate-raw')`・`navigator.storage.estimate/persisted` が使えた。(e) 3,000 ファイルのフォルダは最初の表示を名前だけで行い(Chrome 308 ms、Firefox 75 ms)、サイズと日付は後から入る(§5)。
3. **未確認・未着手**: **依頼者の Firefox 159 / Windows 10 での動作(0 件)**。OS の本物のファイル選択ダイアログ・本物の人の操作(0 件)。Firefox の本物のフォルダ入力と本物のフォルダのドロップ。Firefox の `persist()` の許可ダイアログの応答。プライベートウィンドウ。OPFS が他のタブ・エンジンに開かれているときの `NoModificationAllowedError` の画面表示(コードはあるが、テストは 0 件)。数万ファイルのフォルダ(仮想スクロールは未実装: §5)。1.5 GiB を超える『フォルダを zip でダウンロード』(拒否するだけ)。**段階 2・3 は未着手**(§8)。

## 1. 何を足したか(差分 `opfs1-s1.diff`)

`app/src/files/`(新規):

| ファイル | 役割 |
|---|---|
| `paths.ts` | パスと名前の規則(エンジンの仮想ボリュームの規則〔`engine/filedlg.ts` の `isValidComponent`/`sanitizeComponent`〕を import して二重に持たない)。大文字小文字だけ違う名前の衝突、`..`・ドライブ文字・絶対パスの拒否、`name (2).ext` の採番 |
| `zip-read.ts` | 中央ディレクトリの読み(Zip64、0x7075、名前の復号)、CRC-32、エントリのバイト列をストリームで返す(stored はそのまま、deflate は `DecompressionStream('deflate-raw')`)。zip 全体をメモリに持たない |
| `opfs.ts` | `FileSystemDirectoryHandle` の上の小さな非同期ファイルシステム(`list`〔名前だけ/サイズつき〕、`write`〔ストリーム〕、`move`〔`FileSystemHandle.move`、失敗したらコピー+削除〕、`copy`、`remove`〔再帰〕、`du`、大文字小文字を無視した検索)。ルートは引数なので、段階 3 でゲストの部分木をルートにできる |
| `ops.ts` | アップロード(`putFiles`)、zip の展開(`extractZip`: エントリごとに OPFS ファイルへストリームし、CRC を検査し、不一致は削除して報告)、フォルダの zip 化(`folderToZip`: fflate の `Zip`)、`saveBlob`、進捗・中止・衝突の 1 回だけの質問 |
| `explorer.ts` | 窓の DOM(ツリー・一覧・パンくず・ツールバー・進捗・容量・モーダルの質問)、キー操作(↑↓ Enter Backspace F2 Del Ctrl+A/C/X/V Esc)、ドロップ |
| `mount.ts`, `files.css` | ヘッダーのボタンとオーバーレイ(z-index 2147482000: エンジンの窓〔1001〜〕の上、fpick1 の箱〔2147483000〕の下) |

`app/src/main.ts` に import 1 行と 2 行の追記だけ(`window.__files` はテスト用の取っ手。ファイル名は載せない)。`index.html`・`style.css` は無変更。

## 2. 既製のライブラリの調査と選択(表: `tables/lib-comparison.tsv`)

| 役 | 候補(版・ライセンス) | gzip(実測) | 判断 |
|---|---|---|---|
| zip の書き出し | **fflate** 0.8.3 MIT | 4,046(読みのみ)/ 8,412(書き+読み) | **採用**(`Zip`・`ZipDeflate`・`ZipPassThrough`)。UTF-8 名なら言語エンコーディングフラグを立てる(Python の `zipfile` で確認) |
| zip の読み | fflate / @zip.js/zip.js 2.24.0 BSD-3 / jszip 3.10.2 MIT-or-GPL / unzipit 2.1.1 MIT | 4,046 / 50,413〜71,340 / 30,251 / 4,783 | **使わなかった**。fflate・unzipit・jszip は名前を内部で復号して生のバイト列と拡張フィールドを出さず、依頼者の zip の名前に 0x7075 が要る(§3)。zip.js は 0x7075 を扱うが 12〜17 倍大きい。代わりに自前の約 130 行の中央ディレクトリ読み+ブラウザの `DecompressionStream` |
| ドロップされたフォルダの走査 | **file-selector** 5.0.2 MIT(react-dropzone の部品) | 1,758 | **採用**(`fromEvent`)。`webkitGetAsEntry`(Firefox 50+)を使い、Chrome は `getAsFileSystemHandle` |
| フォルダの選択 | browser-fs-access 0.38.0 Apache-2.0 | 2,316 | 使わなかった(`<input webkitdirectory>` は 3 行。Firefox に `showDirectoryPicker` は無い〔MDN BCD〕) |
| OPFS の薄い包み | opfs-tools 0.8.1(npm のライセンス欄が空)、opfs-worker 2.3.0 MIT | 5,700 / 45,800 | 段階 1 では不要。段階 3(同期アクセスハンドル)で参考にする |
| ファイルマネージャーの UI | js-fileexplorer 1.0.0(MIT or LGPL、依存なし、UMD)、opfs-tools-explorer 1.1.6(MIT、React+MUI)、@e280/quay(WIP、Shoelace)、@mickaelvieira/opfs-explorer(GitHub Packages、ライセンス不明)、neiki-file-manager・live-codes/file-tree(MIT、README のみ)、SVAR(React/Svelte)、Flmngr(サーバー) | js-fileexplorer は未圧縮 240 KB の JS(gzip 44 KB)+CSS+画像 | **自作(TS 1,387 行)**。React/MUI を要するもの・サーバーを要するものは除外。js-fileexplorer は依存が無いが、アップロードが XHR 前提(`init_upload`/`prepare_xhr`)で、OPFS 用の変換層の大きさが読めず、段階 2 の『選択モード』(ファイル種別フィルタ、名前入力、上書き確認)を足す場所が無い。**試して比べたわけではない**(README とファイルの読みだけ) |

CSP が `style-src 'self'`(インライン style 不可)なので、CSS を自前のファイルにした(本番ビルドの `vite preview` で **CSP 違反 0 件**: ページ内の `securitypolicyviolation` の記録が空、Chrome・Firefox とも)。

## 3. zip の名前(実測。決定 0178/0193 の記述の訂正)

依頼者の `~/ダウンロード/serina1-serum2.zip`(49 エントリ、deflate 45・stored 4、フラグ 0x800 なし、暗号化・Zip64・データ記述子なし)の中央ディレクトリを読んだ(Python の `struct`)。

- 日本語名の 2 エントリ(index 38・39)のヘッダー名は、`[fadr.com] Vocals - ` の後が Shift-JIS の `クリスマスセリナ` + ASCII の `#$3099` + `ー` + `　` + `高画` + `\x8e#$3099` + `る#$3099るあー#$3099 - 115bpm…`。**『メモリアルロビー』『質』『ぶ』『かいぶ』が欠けている**(writer が自分のコード変換で文字を落としている。読み)。`TextDecoder('shift_jis')` で復号し `#$3099` を U+3099 に戻しても、`[fadr.com] Vocals - クリスマスセリナ゙ー　高画\ufffd゙る゙るあー゙ - 115bpm…` になり、**本来の名前と一致しない**(単体テスト `files-zip-read.test.ts` が『違う』ことを固定している)。
- 同じ 2 エントリに **拡張フィールド 0x7075(Info-ZIP Unicode Path、版 1、CRC-32 = ヘッダー名の CRC-32 と一致)**があり、その UTF-8 名は `[fadr.com] Vocals - クリスマスセリナ　メモリアルロビー　高画質 - ぶるぶるあーかいぶ - 115bpm - Cmin (consolidated).wav`(と `_2.wav`)。濁点は**別の文字 U+3099**(ヒ+゛、ふ+゛。NFC にすると変わる)。
- **オラクル**: 抽出した `.flp`(1,322,619 B)の中を、47 個の ASCII 名と 0x7075 の 2 個の名前の UTF-16LE で検索し、**48/48 が見つかった**(`files-opfs.mjs` が Chrome・Firefox の両方でブラウザ内で実行。Python でも同じ結果)。つまり FLP が参照する名前 = 0x7075 の名前。
- → 実装: 名前は UTF-8 フラグ → 0x7075(CRC が合うときだけ)→ ASCII → CP932(+ `#$XXXX`)の順。**正規化はしない**。0x7075 の無い zip で CP932 しか無い場合は、writer が欠けさせた名前がそのまま出る(エンジンは音声を見つけられず『見つからないサンプル』の警告になる。読み)。決定 0193 §1.3 の P3 案『CP932 でデコード → `#$3099` を U+3099 に → NFC。デコードできない所は FLP が持つ名前の一覧から突き合わせる』の前半は不要(0x7075 があれば足りる)で、NFC は誤り、『FLP の名前との突き合わせ』は 0x7075 が無いときの代替案として残る(未実装)。

## 4. テスト(`app/test/e2e/files-opfs.mjs` + `files-scenario.mjs`、vitest)

`tables/e2e-checks-chrome.tsv`・`e2e-checks-firefox.tsv` に全項目。要点(Chrome 60/60、Firefox 59/59):

- 窓の開閉、空のルート、容量行(`estimate()` と永続化の状態)、重なり(z-index)。
- 新規フォルダ(`a:b` は理由つきで拒否、`Projects` と `projects` は衝突として拒否)、アップロード(2 ファイル: **SHA-256 が元と一致**)、フォルダのアップロード(構造と全ファイルの SHA-256)。
- 名前の衝突(置換 / 両方残す / 飛ばす / 中止の 4 つとも、結果を OPFS で確認)、リネーム(F2)、コピー/貼り付け(同じフォルダ内は採番、別のフォルダで衝突は質問)、フォルダを自分の中に移せない、切り取り+貼り付け(フォルダごと、元は消える)、削除(確認文に件数と大きさ、中止で何も消えない)、Backspace。
- zip: UTF-8 フラグ、フラグ無しの CP932、0x7075、**zip-slip**(`../`、`a/../../`、`C:\`、`//`)は拒否して選んだフォルダの外に何も書かない、絶対・バックスラッシュの名前は相対にし Windows で不正な文字は `_`、**CRC-32 が合わない entry は削除して報告し、他は残す**、暗号化と未対応の圧縮方式は報告して残りを展開、24 個×3 MiB の展開を途中で**中止**(途中のファイルが残らず、残ったファイルはすべて 3 MiB)、**依頼者の zip(49 ファイル、62,747,002 B)**の全ファイルの SHA-256 が Python の読みと一致、FLP の名前と一致。
- ダウンロード(単一ファイル: バイト一致、フォルダ: zip を Python の `testzip` で検査、UTF-8 名・フラグ・バイトが一致)。
- ドロップ(模擬: 2 ファイル+zip → 『展開しますか』の質問 → 展開。Chrome のみ本物に近いドロップ: フォルダ+ファイル)。
- `persist()` の呼び出しと応答の表示(Chrome は『許可されなかった』、Firefox は確認待ち)。
- 3,000 ファイルのフォルダ(§5)。CSP 違反 0 件。

vitest: `files-paths.test.ts`(6)、`files-zip-read.test.ts`(9。うち 1 件は依頼者の zip があるときだけ走る)。**全体: 1,305 passed + 7 skipped**。私の 2 ファイルを除いた同じ木の値は 1,290 passed + 7 skipped(`--exclude 'test/unit/files-*'`)で、PM の基準 1,287 とは 3 件違う(木の HEAD が違うか取り込みの差。**未調査**)。+15 が本差分の分。型検査(4 つの tsconfig)rc=0。

## 5. 性能(実測。この機械、load 40〜56 の壁時計。依頼者の機械の値ではない)

- 依頼者の zip の展開(59.8 MiB を OPFS に書く、CRC 検査つき): Chrome 4,240 ms(load 54)、Firefox 2,239 ms(load 48)。
- 3,000 ファイルのフォルダ(空のファイル): 最初の表示まで、**ツール変更前**(サイズを先に全部読む設計)の Chrome は 3,448 ms。**名前だけで先に表示し、サイズ・日付を 32 個ずつ後から入れる設計**にして Chrome 171〜758 ms(load 54〜56)、最終版 308 ms、Firefox 62〜75 ms。サイズと日付が全部入るまでは、最終版で Chrome 6.7 s、Firefox 2.7 s。**途中の版で Chrome 88〜101 s**かかり、原因を測った: ブラウザの `getFile()` 3,000 回は 570 ms(`work/bench-getfile.mjs`)なのに、3,000 行の DOM を更新するたびの再レイアウトが主(SwiftShader・高負荷)だった。`.fx-row { content-visibility: auto }` の 1 行で 101 s → 6 s。**仮想スクロールではない**ので、数万ファイルのフォルダは未確認。
- ページの JS の大きさ: gzip 45,385 → 64,5xx B(+19 KB)。エンジンの Worker・wasm は不変。

## 6. 発見(依頼者の提案の評価)

- 『OPFS を管理する GUI があれば、FL の「開く」は OPFS からの選択 UI を JS で実装するだけで済む』は、**ページ側の素地としては成り立つ**: 同じ窓を段階 2 で選択モードにすれば、ユーザー操作(transient activation の 5 秒)の制約が無くなる(読み: 窓の中のクリックは、OPFS の読み書きにユーザー操作の活性化を要しない。ブラウザの選択窓を開く `input.click()`/`showOpenFilePicker` を呼ばないため)。ただしそれは**窓から Worker へ渡す部分**(`FileDlgAnswer` の `File` → Worker が `File.stream()` でヒープへ読む、fpick1 の経路)を段階 2 で確かめてからの話で、まだ確かめていない。
- Chrome の CDP は `<input webkitdirectory>` にディレクトリのパスを渡すと展開してくれるが、Firefox の WebDriver は渡せない。Firefox の実機でのフォルダ入力・フォルダのドロップは、依頼者の Firefox で一度開いて確認してもらうのが最短(`Files` → Upload folder / フォルダをドロップ)。
- Firefox の quota は、この機械の fresh profile で『約 371 MiB』と表示された(プロファイルが tmpfs〔3.7 GB〕上にあり、空き領域に比例するため。読み)。依頼者の機械の値は未確認。依頼者の zip(59.8 MiB)は入る。

## 7. 出典のない選択(S- の一覧。代替案つきの全文は `tables/s-choices.tsv`)

S-opfs1-ui-own(UI を自作)、S-opfs1-zip-names(名前の復号の順、正規化なし)、S-opfs1-zip-reader(自前の中央ディレクトリ読み+`DecompressionStream`)、S-opfs1-zip-methods(stored と deflate だけ)、S-opfs1-names(エンジンの仮想ボリュームの名前規則+大文字小文字の衝突)、S-opfs1-extract-folder(zip の名前のフォルダに展開)、S-opfs1-conflict-once(衝突は 1 操作に 1 回だけ質問)、S-opfs1-replace-folder(『置換』はフォルダも消す)、S-opfs1-zip-store-ext(zip 化で無圧縮にする拡張子)、S-opfs1-zip-limit(1.5 GiB 超は拒否)、S-opfs1-quota-check(`estimate()` で事前に拒否)、S-opfs1-stat-lazy(サイズは後から 32 個ずつ)、S-opfs1-mtime(zip の更新日時は適用しない)、S-opfs1-z-index、S-opfs1-no-folder-drop-empty(ドロップした空フォルダは作らない)、S-opfs1-1-ff(テストした Firefox は 158)。

## 8. 段階 2・3 の設計メモ(**未着手。読みと計画**)

- **段階 2**: 現状の橋(決定 0193)は、`Show()` が JSPI で停止 → ページが要求(`FileDlgRequest`: `save`・`options`・`filters`〔[表示名, "*.flp;*.zip"]〕・`fileName`・`defaultExt`・`folder`)を受ける → `FileDialogView` の箱(`<input type=file>` と `<input webkitdirectory>`、保存名の入力)→ 答え(`FileDlgAnswer`: `items` に `File` とゲストのパス)→ Worker が `File.stream()` をエンジンのヒープへ読み `fle_vfs_put_file` で `C:\Users\user\Downloads\…` に置く。→ `FileDialogView` の中身を、この窓の**選択モード**(開く: フィルタ付き、保存: 名前入力+既存の上書き確認、フォルダ選択)に差し替えれば、`FileDlgAnswer` の `file` は OPFS の `getFile()` の `File` でそのまま同じ経路に流せる(コピーはヒープへ 1 回)。FLP を選んだら同じフォルダの兄弟ファイルも `E` として置く(今のフォルダ選択と同じ。上限 256 MiB/384 MiB/512 ファイル)。zip を選んだときは、まずページ側で展開してから中の `.flp` を開く。**エンジンを実際に動かすテストには `-DFLE_FILEDLG_BROWSER` のバンドルが要る**(SHIPH の既定には含まれない。fpick1 のワークツリーのバンドルは残っていない)ので、バンドルの再生成が要るかを段階 2 の最初に確かめる。
- **段階 3**: `createSyncAccessHandle` は Worker 専用で、開くのが非同期。候補: (1) JSPI の中断で開く、(2) ヘルパー Worker + SharedArrayBuffer + `Atomics.wait` の同期橋、(3) 列挙時に先読みして開く。Chrome 102+・Firefox 111+ の両方にあることは MDN BCD で確認済み。実測は未着手。

## 9. 自己申告(規則との関係)

- 『サブエージェントを通して作業する』という AGENTS.md の指示は、私自身がサブエージェントとして起動されており、タスク用のツールを持たないため、自分で実施した。
- sleep を 1 回呼んで拒否された(以後使っていない)。前景の `node test/e2e/files-opfs.mjs` が 120 秒の既定の時間切れで中断され、その孤児の `vite preview` を 1 つ残したので PID で止めた。途中の版のテストが競合(`statsDone` の読み取りの競合)で止まり、PID で止めたことが 2 回ある。
- テストデータは `lift/out-opfs1/testdata/`(snap の Firefox が /tmp を読めないため)。作業用 `node_modules`(457 MB)は終了時に消す。
- 既存の e2e `run.mjs`(`--no-build`、dev・preview・worker の 3 target): **1107/1109**(`logs/e2e-run-mjs.txt`)。失敗 2 件は `default-manifest-standin`(dev)と `default-manifest-none`(worker)の『`engine-state` の data-state が 'not-started' であること』で、実際は 'error'。**素の HEAD(7c0e9cc の `git archive`、私の差分なし)でも同じ検査が落ちる**ことを `--only default-manifest` で確かめた(dev: 1 件、worker: 2 件落ちる。どの検査が落ちるかは実行ごとに違う、読み: 時間依存)。PM の基準 1,109/1,109 はこの機械では再現しなかった。私の差分は原因ではない(`main.ts` の追記は `window.__files` と Files ボタンだけ)。

## PM の判断と取り込み(段階 1、2026-11-04)

取り込み: `opfs1-s1.diff`(19 ファイル。`app/src/files/`〔自作の OPFS エクスプローラー〕、`main.ts` に import 1 行と 2 行、`package.json` に dependencies〔fflate ^0.8.3、file-selector ^5.0.2〕、`package-lock.json`、e2e 6 本、単体テスト 2 本。'-' 行 0)を当てた(main.ts は offset 1 行)。PM の測定: `npm install` 後 typecheck rc=0、vitest **1,331 passed + 7 skipped**(1,316 + 15)、Worker 43、`files-opfs.mjs`(隔離 Chrome 156 の実 OPFS、本番ビルドの vite preview、CSP style-src 'self')**60/60**、既存の e2e **1,109/1,109**。担当が報告した『素の HEAD でも default-manifest-* が 2 件落ちる』は PM の環境では再現しない(crash1・aud3 も同様の報告。それぞれの環境で、FL_ASSETS_DIR 等の環境変数や並行負荷が関係する競合の可能性=未調査)。PM のテストは、`testdata-files/`(75 MB。依頼者の zip と FLP のコピーを含む)と `downloads-chrome/` をリポジトリ直下に書いたので、削除し、`.gitignore` に追加した。**段階 2 以降は、テストのデータ置き場を `lift/out-opfs1/` など git 管理外にする**(デフォルトの `--data` を変更すること)。

判断:

1. **依存の追加(fflate、file-selector。MIT、ページの JS は gzip で約 +19 KB)を了承**。Files ボタンの常時表示も了承(`?files=0` で隠せる 1 行は段階 2 で足す)。
2. **zip の名前の訂正(決定 0178・0193)**: 依頼者の `serina1-serum2.zip` の日本語名 2 個は『CP932 + #$3099 の復元』では復元できず、Info-ZIP の Unicode Path 拡張(0x7075、CRC 一致)の UTF-8 名で 48 個すべての音声名が FLP の UTF-16LE と一致する(48/48)。名前の決定順は『UTF-8 フラグ → 0x7075 → ASCII → CP932』、正規化しない。決定 0178・0193 の該当記述はこの決定が置き換える。エンジン側の DelZip64 が 0x7075 を読むかは、dec1(決定 0206)の結果で比較する。
3. 未確認(担当の報告どおり): 依頼者の Firefox 159 / Windows、本物の OS ダイアログ、Firefox の本物のフォルダ入力・ドロップ、persist() の応答、他タブ/エンジンが開いているファイルへの書き込み失敗、数万ファイル。段階 2(エンジンの『開く/保存』を OPFS の選択モードに)と段階 3(OPFS をゲストの実体に)は未着手。
