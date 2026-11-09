# 0315c(仮): 実験用の seed(RT-A の bundle の seed だけを差し替えた『fl-engine-test-rt1-seed2』)— About のロゴ・staff roll・日本語フォントの確認用(mt3。PM の instruction 09:5x。**実験であり本番でも push でもない**)

状態: **仮(10:0x に作り直した版 = §4。最初の版 `…-184136ac` は engine が起動しなかった)**。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**、**未確認**。**実エンジンの実行・ブラウザは 0**(gpu1 が確認する)。

## 0. 実際に実行した物(作り直した版。最初の版の経緯は §4)

1. 本物の抽出器 `ci/lib/extract_fl.py`(installer `flstudio_win64_26.1.7.5653.exe`)を、RT-A の木の複写 `tree-rt1s` で実行: 『**9 DLL + 229 seed file written, all SHA-256 verified**』rc 0。
2. 本物の CI の段 `seed` と `bundle`(`GIL=1`、nice 19)を、RT-A の work dir の hard link 複写 `ci-rt1s` で実行: rc=0 09:56:20(seed の段は重複検査つきの pack_seed.py を通った)。wasm・glue・DLL pack は **RT-A とバイト同一**(cmp)。変わったのは seed pack・manifest・SHA256SUMS だけ。
3. 試験版 `fl-engine-test-rt1-seed2-8128f5be`(~/work/mt3w-s1/ci-rt1s/engine/bundle/): SHA256SUMS は通常ファイル(link 数 1)、`sha256sum -c` ok=9・bad=0、manifest の sha256 `5edb4c3c39aa53cc6410f6dd8781146d31dab7dd9a9b49fae58cdc35d0b9aa1b`。README-TEST-BUNDLE.txt 入り。
4. native の実 loader(`vfs_seed.h`)をその seed pack の展開に対して実行: rc=0(§4)。

## 1. 追加した行の表(元の installer のパス・大きさ・SHA-256)

guest の根は `C:\Program Files\Image-Line\FL Studio 21`(以下 `<FL>`)。『gpu1 の vfs-miss』は 0319/logs/cand1-about/log-all.log の `fl-diag: vfs-miss …` の行。

| # | 目的 | guest path | installer の中のパス | 大きさ | SHA-256 | 状態 |
|---|---|---|---|---|---|---|
| 1 | About のロゴ(vfs-miss: GetFileAttributesW status 2) | `<FL>\Artwork\Skins\Default\BigFruit.png` | `Artwork\Skins\Default\BigFruit.png` | 166,508 | `0a8dab6829bde22c64bbea6d5c467d8f89bc14a4b8c3bee93bc6b257f2e6e108` | **installer にある**・追加 |
| 2 | staff roll の本文(CreateFileW status 3) | `<FL>\System\Texts\About\AboutTxt.txt` | `System\Texts\About\AboutTxt.txt` | 9,228 | `2e59358cbd85ac611a8fd2296f12adb6ad6ad98f8a5689818d0c0fa5a8c8813c` | **ある**・追加 |
| 3a | Fruity microfont(installer の位置) | `<FL>\Artwork\Fonts\Fruity microfont.ttf` | `Artwork\Fonts\Fruity microfont.ttf` | 20,072 | `7ecd6ce6c72a505e21d5d7ba60b30e97661e1f563042a0271e5e054fc542e791` | **ある**・追加 |
| 3b | 同(guest が探す所 1: vfs-miss GetFileAttributesW status 2) | `<FL>\Shared\Artwork\Fonts\Fruity microfont.ttf` | **installer にはこの位置のファイルは無い**(3a と同じ bytes) | 20,072 | 同上 | **実験**: 3a の bytes をこの path に置いた(os-fonts の経路: tree の `os-fonts/Fruity_microfont.ttf`〔installer から読んで SHA 検証した物〕を SHA 検査つきで複写) |
| 3c | 同(guest が探す所 2: `Windows\Fonts\Fruity microfont.ttf`、5 回) | `C:\Windows\Fonts\Fruity microfont.ttf` | 同(Windows の Fonts への導入は install tree に無い) | 20,072 | 同上 | **実験**(同じ扱い) |
| 4a | (最初の版が足した行。**取り下げ**) | `<FL>\System\Internet\Default news.rss` | `System\Internet\Default news.rss` | 434,047 | `d4bc1d95…` | **RT-A の seed に既にある**(vfs-seed-idle1.tsv の 237 行目)。最初の版は同じ guest path をもう一度足し、loader が拒否した(§4) |
| 4b | 同の `Versions.xml`(vfs-miss: GetFileAttributesW status 2 `System\Internet\Versions.xml`) | — | **installer の install tree に無い**(`versions.xml` に一致するエントリ 0) | — | — | **無い**(推測で作らない・代役を入れない)。FL がネットから取る物と読むが、確認は無い |
| 5 | (最初の版が足した行。**取り下げ**) | `<FL>\System\Config\Typing to piano\Layout Piano.scr` | `System\Config\Typing to piano\Layout Piano.scr` | 236 | `e4ac91ab…` | **RT-A の seed に既にある**(204 行目)。同上 |
| 6 | 日本語フォント | `C:\Windows\Fonts\msgothic.ttc` | **installer に無い**(OS のフォント) | 2,123,288 | `daa1948fcd6cddc9549249d79ef075f6001c3693bf2a61ad8717679d7fb80b2f` | opt1 の `~/work/opt1-wine/fonts-pkg/msgothic.ttc`(FreeType 2.13.2 で確認済み、OFL.txt・COPYRIGHT.txt 同梱の組)。**もう一つの `scratch/fonts-det-1/msgothic.ttc` は同じ大きさ 2,123,288 B で SHA-256 が違う**(`1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69`)ので使っていない。os-fonts の前例どおり tree の `lift/harness/seeds/os-fonts/` に置いて seed に入れた |

- 3b・3c は『同じ bytes を別の path に置く』実験である(installer のファイルだが、その path で配られた物ではない)。結果が良くても『FL がその path を見る』ことの確認で、製品の seed にどの path で入れるかは別の判断(Windows の契約: フォントは `Windows\Fonts` から列挙される物、`Shared\Artwork\Fonts` は QuickFontCache の探索の候補の読み)。
- 追加していない(同じ vfs-miss にあり installer にもある小物): `Artwork\Skins\Default\{Title.png, ScopeGradient.bmp, StartSnd.wv}`、`System\Internet\About\*.url` 5 本、`System\Config\Typing to piano\` の他の 6 本、`System\Internet\{FL Studio online.url, error-offline.html}`。依頼の外。

## 2. seed pack の大きさ(実測。作り直した版 `fl-engine-test-rt1-seed2-8128f5be`)

| | RT-A | seed2(作り直し) | 増分 | (最初の版 …-184136ac、起動せず) |
|---|---|---|---|---|
| seed pack(raw) | 14,748,394 B | **17,089,558 B** | **+2,341,164 B** | 17,524,658 B |
| seed pack(brotli、BROTLI_QUALITY=1) | 8,240,342 B | **9,826,130 B** | +1,585,788 B | 10,153,231 B |
| seed list の行数 | 243 | 249(+6 行) | | 251 |

追加した file の中身: msgothic.ttc 2,123,288 + BigFruit 166,508 + AboutTxt 9,228 + microfont 20,072(3 path に置いたが bytes は 1 つにまとまる読み)。起動の取得量(seed の br)は +約 1.6 MB。manifest の sha256 `5edb4c3c39aa53cc6410f6dd8781146d31dab7dd9a9b49fae58cdc35d0b9aa1b`、SHA256SUMS ok=9。wasm・glue・DLL pack は RT-A と cmp で同一(hard link)。

## 3. gpu1 の確認の観点(読み)

- About のロゴ: 起動の miss の BigFruit.png が消え、About 窓の 3×3 の絵(O3)がロゴになるか。staff roll: `AboutTxt.txt` を読むか(vfs-miss の CreateFileW status 3 が消えるか)。
- 日本語: opt1 の `variants/ja-names.flp`(日本語のチャンネル名)で豆腐が消えるか。msgothic.ttc を FL の GDI のフォント選択が拾うかは**未確認**(face 名 `MS Gothic` / `MS PGothic` / `ＭＳ ゴシック` の対応は runtime のフォント表の領分。拾わなければ『資材はあるが選択の契約が足りない』と切り分けられる)。
- 診断の env(FLE_DIAG_VFS_MISS=1)は RT-A の manifest に無い。vfs-miss の消え方を見るには manifest の engine.env に足す(cand1 の bundle にはある)。

## 4. 最初の版が起動しなかった理由と、作り直し(mt3。PM の instruction 09:5x)

**起きたこと(gpu1 の実 Chrome 156 の実測)**: `fl-engine-test-rt1-seed2-184136ac` は engine が 3 秒で exit 2。engine の log の最後: `vfs-seed: /data/seed/vfs-seed-idle1.tsv:249: the guest path C:\Program Files\Image-Line\FL Studio 21\System\Internet\Default news.rss is listed twice (line 237)`。

**原因(実測)**: 私が足した 8 行のうち 2 行(`Default news.rss`、`Layout Piano.scr`)は、RT-A の seed list(= cand1 と同一の vfs-seed-idle1.tsv)に**既に同じ guest path の行がある**(237・204 行目。host path は空白入りの `…/Default news.rss`、私は下線入りの `…/Default_news.rss` を書いた)。loader(`lift/harness/vfs_seed.h` の `vs_parse`)は同じ guest path を大小文字を無視して 2 回書くと list 全体を拒否する。
**見落とした理由**: 私は gpu1 の vfs-miss の行(`CreateFileW status 2 …Default news.rss`)を見て『seed に無いから足す』と判断し、**足す前に RT-A の list にその guest path が既にあるかを調べなかった**。さらに実行した物は host の道具(extract_fl.py の SHA 検査、pack の作成、SHA256SUMS)だけで、**loader を通していなかった**ので、再 build の前に気づく手段が無かった。
**付随して分かったこと(読み。追っていない=未確認)**: cand1 の log(gpu8-results/0319/logs/cand1-about/log-all.log)は、この 2 ファイルを 223・256 行目で seed している(`vfs seed: … <- …`)のに、519・552 行目で同じ path の `CreateFileW status 2` を出している。つまり『seed に無いから miss』ではなかった。seed した後に何かが(guest が MoveFile/Delete で)動かした、または path の綴りの扱いが違う可能性がある。これは seed の問題ではなく別の調べ物。

**作り直し**: 8 行から重複 2 行を除き(`seed2-build.sh` の rows())、6 行(BigFruit.png、AboutTxt.txt、microfont × 3 path、msgothic.ttc)にした。

**重複を bundle にする前に検出する検査(2 つ)**
1. seed の段: `lift/harness/web/final0129/pack_seed.py` に loader と同じ規則(guest path は 1 回。ASCII の大小文字を無視。directory の行は末尾の backslash を除いて比べる)を足した差分 `impl/0315-pack-seed-duplicate-guest-path.diff`。重複があれば pack を作る前に `vfs-seed: …: the guest path … is listed twice (line N)` で止まる(loader と同じ文言)。試験 `tools/test-pack-seed-dup.sh`: (1) 完全な重複、(2) 大小文字違い、(3) directory 行の大小文字違い、(3b) directory 行と同じ名前のファイル行、は止まり、(4) 今の list は pack できる: all hold(実測)。
2. 実 loader を native で通す: `tools/seedpack-unpack.py`(bundle の seed pack を `/data/seed` と同じ形に展開し、各ファイルの SHA-256 を header と照合)+ `tools/vfs_seed_check.c`(`lift/harness/vfs_seed.h` をそのまま include し `vfs_seed_from_env()` を呼ぶ。engine 本体が起動時に呼ぶ物と同じ関数)。`seed2-build.sh check [bundle]`。

**実測した結果**
- 負の対照: 壊れた最初の版 `…-184136ac` の pack を同じ手順で通すと、native の loader が gpu1 の見た文言と**同じ**に拒否する(`…vfs-seed-idle1.tsv:249: the guest path C:\Program Files\Image-Line\FL Studio 21\System\Internet\Default news.rss is listed twice (line 237)`、rc=-1)。= この検査は gpu1 の失敗を再現できる。
- 作り直した版 `…-8128f5be`: 232 file を展開(SHA-256 は全部 header と一致)、loader は **rc=0**、『vfs seed: done, 248 files, 51 directories created』(file 248 = cand1 の log が出した 242 + 今回の 6 行)。stderr 0 行。
- これは **native の loader だけ**(lifted code は動かしていない)。engine が起動して first idle まで進むかは gpu1 の実 Chrome の確認が要る(未確認)。
