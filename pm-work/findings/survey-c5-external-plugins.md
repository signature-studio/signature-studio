# survey-c5-external-plugins: 外部プラグイン(Serum 2・OTT・Vital)の現状(読みと入力の確認)

実エンジン: 実行なし(入力が無い)。外部プラグインのインストーラが runner に無いことを、grep と find の出力で確かめた。ビルドも、プラグインの読み込みも実行していない。

判定: 未確認(外部プラグインの実測が無いため。製品の経路は読みだけで書いた)。

## 実測

### 1. 入力(インストーラ)の確認

`grep -n 'installers/' ci/artifacts.lock | cut -c1-60` の出力(ベンダー名の部分は `<社>` に書き替えた):

```
5:installers/flstudio_win64_26.1.7.5653.exe	1106033456	04393
6:installers/Install_<社>_Serum2_2.1.5.exe	1313084056	507b72
7:installers/Install_<社>_OTT_137.exe	8397928	d4e1d3f62a1db
8:installers/VitalInstaller.exe	41808960	5bf1b8c7b08ca55d9b0
```

5 行目が FL Studio、6〜8 行目が外部プラグインの 3 件(Serum 2・OTT・Vital)。

`grep -n 'R2_ARTIFACT' .github/workflows/agent.yml` の出力(鍵の名前だけ。値は読んでいない):

```
69:          AWS_ACCESS_KEY_ID: ${{ secrets.R2_ARTIFACT_ACCESS_KEY_ID }}
70:          AWS_SECRET_ACCESS_KEY: ${{ secrets.R2_ARTIFACT_SECRET_ACCESS_KEY }}
```

- 鍵を使う段は、`extract` ジョブの「インストーラを取り、抽出する(キャッシュが無いときだけ)」(agent.yml:66)。
- この段が取るのは FL のインストーラだけ(agent.yml:74 の `aws s3 cp` の行)。
- `run` ジョブ(agent.yml:78 以降)に R2_ARTIFACT_* の参照は無い(上の grep は 2 行だけ)。

`find . -path ./node_modules -prune -o \( -iname '*serum*' -o -iname '*OTT*' -o -iname 'Vital*' \) -print | grep -v '^./.git/' | head -20` の出力:

```
./app/test/unit/report-throttle.test.ts
./app/test/unit/plugins-ott.test.ts
./app/src/plugins/ott-host.ts
./app/src/engine/report-throttle.ts
```

- インストーラ(exe など)は 0 件。
- `report-throttle` の 2 件は、`*OTT*` が throttle の中の ott に当たったため。

`ci/external-inputs.sha256` の 1 行目は `serina1-serum2.zip`。この行の物は、`ci/artifacts.lock` に無く、find(`*serum*`)でも runner に無い。

### 2. 結論

外部プラグインの入力は runner に無い。根拠は次の 2 つ。

1. find の出力に、インストーラが 0 件。
2. R2 の鍵を使うのは agent.yml:69〜70 の 1 箇所だけで、その段は FL のインストーラしか取らない。作業ジョブに、このバケットの鍵は無い。

よって、この実行ではビルドも読み込みも行えない。実測は止める。

### 3. 製品の経路の件数(読みの材料。実行はしていない)

| 項目 | 件数 | 出典(コマンドと出力) |
|---|---|---|
| 製品の画面・engine(app/src・app/worker・app/shared)から plugins を呼ぶ行 | 0 行 | `grep -rn "plugin-client\|plugins/driver\|plugins/ott\|plugins/protocol\|plugin-worker" app/src app/worker app/shared --include=*.ts` は 9 行を返し、すべて app/src/plugins/ の中(plugin-worker.ts:12・46、ott-host.ts:2・4、plugin-client.ts:3、node-worker.ts:1・3・4・7) |
| app/src/plugins の中のファイル | 8 件 | `ls app/src/plugins`(driver-worker.ts、driver.ts、node-worker.ts、ott-host.ts、plugin-client.ts、plugin-worker.ts、protocol.ts、test-signal.ts) |
| plugins を使うファイル(テストと helper だけ) | 3 件 | `grep -rln "plugins/" app --include=*.ts ...` の出力(app/test/helpers/echo-plugin-worker.ts、app/test/unit/plugins-ott.test.ts、app/test/unit/plugins-protocol.test.ts) |
| OTT の harness の source(lift/harness/vst) | 0 件(ディレクトリが無い) | `git ls-files lift/harness/vst lift/harness/web/mt1` は 0 行。`test -f lift/harness/vst/main_vst.c` は no |
| OTT の harness の build(lift/harness/web/mt1/plugin-worker/build.sh) | 0 件 | `test -f lift/harness/web/mt1/plugin-worker/build.sh` は no |
| OTT のテストの出力先(lift/out-mt1) | 無い | `test -d lift/out-mt1` は no |
| opt-in と書かれた行(lift/runtime・lift/harness/two・lift/crates の .c .h .rs) | 55 行(44 ファイル) | `grep -rn -i "opt-in\|opt in" ... \| wc -l` は 55。`grep -rl ... \| wc -l` は 44 |
| うち「no longer opt-in」(既定で有効に変わった旨)の行 | 3 行 | fle_user_nowin.c:25、fle_gdi.c:4296、fle_locale.c:949 |
| それ以外の opt-in の行 | 52 行 | `grep -v "no longer opt-in" \| wc -l` は 52 |

既定で無効の物の件数は、この表からは確定しない。読んだのは 6 件だけで、すべて「link が供給しない限り何もしない」と書かれていた(下の「コードの読み」3)。残りの 46 行の既定の状態は未確認。

## コードの読み

### 1. 製品の経路に、プラグインを読み込む仕組みはあるか

- 仕組みは在る。app/src/plugins/ に、1 インスタンス 1 Worker の部品がある(plugin-worker.ts:1、plugin-client.ts:39〜56、protocol.ts:1〜8、driver.ts:1〜2、ott-host.ts:1〜4)。
- ただし製品からは呼ばれていない。呼ぶのはテストだけ(上の表の 3 件)。driver.ts:2 は「Used by the tests; the engine integration (S3/S4) replaces it.」と書く。engine との統合(S3/S4)は、この版には無い。
- ブラウザ用の WorkerHandle は無い。plugin-client.ts:6〜11 の interface を満たす実装は node-worker.ts(node の worker_threads だけ)しか無い。`grep -rn "WorkerHandle\|makeWorker" app --include=*.ts` の出力は、plugin-client.ts と node-worker.ts の 2 ファイル。
- テストは、入力が無いと skip する。app/test/unit/plugins-ott.test.ts:15 の `have` は、wasm と ott.vst3 と seed.tsv の 3 つが無いと偽になり、:42・:57・:81 の `describe.skipIf(!have)` で 3 件とも skip される。

### 2. OTT の参照先

- app/src/plugins/ott-host.ts:1 は lift/harness/vst(decision 0156/0164)の harness の wasm を指す。:2 は lift/harness/web/mt1/plugin-worker/main_vst_ext.c(decision 0173)を指す。
- 2 つとも、リポジトリに無い(上の表の 2 行)。
- app/test/unit/plugins-ott.test.ts:2 が要求する build.sh・vfs-seed-vst.tsv(lift/harness/vst/vfs-seed-vst.tsv)も無い(`test -f lift/harness/vst/vfs-seed-vst.tsv` は no)。
- 入力の ott.vst3 は、インストーラ(R2 の artifact)から抽出する物で、作業ジョブには無い(上の 1 と 2)。

### 3. Serum 2 の経路(lift/harness/vst3a と lift/harness/two)

- lift/harness/vst3a/main_vst3a.c:1〜6。Serum 2 を扱う native の harness(decision 0165, PROVISIONAL)。注釈は「vst3a copy of main_vst.c」だが、main_vst.c はリポジトリに無い(上の表)。usage は `vst-native FILE [...]` で、PE を 1 つ受け取る(:5)。native と wasm の build は同じ source(:4)。wasm の build レシピ lift/harness/vst3a/build_vst3a_wasm.sh は在る(`test -f` は yes)。
- lift/harness/two/wrp1_vst3.c:1〜4。注釈は「Decision 0183 (wrp1, PROVISIONAL, OPT-IN: linked only into the wrp1 native build; nothing else defines the two hooks)」。Serum2.vst3 を engine の real module として読むときの、TLS・IAT・TLS callback の処理。
  - 何を link するか: fle_realmod_pre_attach と fle_realmod_pre_thread の 2 つの hook(lift/runtime/fle_realmod.c:172〜177 の弱い参照)、CSIDL の追加(lift/runtime/fle_ole.c:171〜172 の弱い参照)。
  - 既定で有効か: 無効。link されない限り、弱い定義は何も呼ばれない。
- 既定で無効と読める箇所(読んだ 6 件):
  - lift/runtime/fle_realmod.h:5〜7「Without a table (the default: fle_realmod.c holds weak definitions with count 0) no real module exists」。
  - lift/runtime/fle_realmod.c:173〜174「A link that defines fle_realmod_pre_attach ... without it nothing happens」。
  - lift/runtime/fle_realmod.c:176〜177(pre_thread の注釈。同じ形)。
  - lift/runtime/fle_ole.c:171〜172「the default (no definition) stops as before」。
  - lift/runtime/fle_modules.h:82〜84「Decision 0190 (opt-in) ... compiles ... with -DFLE_GMHEX_RETURNS ... Without the macro nothing changes.」。
  - lift/runtime/fle_realmod.h:85〜88「Decision 0183 (wrp1, PROVISIONAL, OPT-IN: a link that supplies no path table behaves as before)」。
- wrp1_vst3.c を link する build は、リポジトリに無い。
  - `grep -rln "wrp1" lift ... --include=*.sh --include=*.mjs --include=*.yml --include=*.rs --include=*.txt --include=*.tsv ...` の結果に、.sh・.mjs・.yml の物は無い。ヒットは seeds の一覧(*.txt)、noreturn-audit の表(allow.tsv)、テスト、fe-build の計画の表だけ。
  - wrp1_vst3.c を compile するのは、テスト lift/crates/fe-lift/tests/wall_wrp3.rs:74〜88 だけ。
  - 注釈の「wrp1 native build」の実体は、リポジトリに無い。
- テスト用の表。lift/crates/fe-lift/tests/wall_wrp1.rs:56〜65 は、自前の stub(fle_call_guest など)と、serum2.vst3 を含む realmod の表(:60)を持つ。製品の表ではない。
- 製品のビルドの入力の表(ci/extract-dlls.tsv・lift/harness/web/pack.args・lift/harness/hyb/hyb-args.tsv)に、serum・vst3 の行は無い(`grep -rn -i "serum\|vst3"` は ci/artifacts.lock の 3 行だけを返す)。
- ただし、製品の engine の build は real module の表を使う。lift/harness/two/harness-inputs.txt:10 が lift/out-app/gen/realmod.c を入力に挙げ、lift/crates/fe-build/tests/plan/engine.txt:277 がそれを emcc で compile する。表の中身(どの module が入るか)は、この読みでは分からない(未確認)。

### 4. 台帳との関係

- ledger/instructions.tsv:3(L-002)は「オプトイン・オプトアウトの処理を禁止する。既定で全部有効」。判定の段は guard/flags.sh。
- Serum 2 の経路の link(wrp1_vst3.c、fle_ole.c の CSIDL、fle_realmod.c の hook、fle_realmod.h の path table)は、すべて opt-in で、既定では無効。L-002 とは逆の状態である。
- この link は製品の build に入っていないため、現状の製品の動作は変わらない(読み)。

### 5. 配信の方針(L-035)と比べた差(読みのみ)

- (a) 入力と配る物: 入力は、3 件のインストーラ(R2 の artifact。ci/artifacts.lock:6〜8)。作業ジョブは鍵が無いので取れない(実測の 1 と 2)。配る物は、方針どおり lift した wasm と必要な data(推定)。インストーラは配らない。wasm を作るには入力が要るので、入力を読む段と、lift する段が要る。agent.yml の 40〜84 行(extract の段まで)に、wasm を作る段は無い。残りの行は読んでいない(未確認)。
- (b) ビルドの出力の経路: vst3a の harness は PE を 1 つ受け取る(main_vst3a.c:5)ので、1 プラグイン 1 wasm の形に近い(推定)。realmod の経路で読むには、wrp1 の link が要る(推定)。その link は製品の build に無い(上の 3)。real module の表は、ビルドが与える(fle_realmod.h:5)。1 wasm に 1 module なら、表は 1 行になる(推定)。vst3a の harness の wasm を使えば、wrp1 の link を経由せずに済む可能性がある(推定)。ただし、wrp1_vst3.c:1〜4 が vst3a からの移植と書いているので、vst3a 単体で動くかは、この読みでは確かめていない(未確認)。
- (c) plugin-client.ts と ott-host.ts: 1 インスタンス 1 Worker の形は、plugin-worker.ts:1 と plugin-client.ts:56〜59 に書いてあるとおり。READY までに、Worker の中で DllMain から pre-roll まで走る(plugin-worker.ts:5〜7、:88〜93)。取得は含まない。files は呼び出し側が渡す(plugin-client.ts:16、:93)。WebAssembly.compile も呼び出し側(app/test/unit/plugins-ott.test.ts:19)。postMessage(init) は転送リストを付けていない(plugin-client.ts:104)ので、files の各 Uint8Array は Worker へ複製される(読みの事実)。大きさへの影響は推定。ブラウザ用の WorkerHandle は無い(上の 1)。

## 未確認

- 外部プラグインの実測。入力が無いため、Serum 2・OTT・Vital のどれも、読み込み、インスタンス化、音声の 1 ブロックの処理を確かめていない。
- 既定で有効か。opt-in の注釈 52 行のうち、読んだのは 6 件だけ(fle_modules.h:82、fle_ole.c:171、fle_realmod.c:172・176、fle_realmod.h:85、wrp1_vst3.c:1)。残りの行の既定の状態は未確認。
- guard/flags.sh が、lift/ の -D マクロ(FLE_GMHEX_RETURNS など)を判定の対象にするかどうか。判定の道具は読んでいない。
- 製品の engine の realmod 表の中身(lift/out-app/gen/realmod.c)。生成物はリポジトリに無いので、どの module が入るかは未確認。
- ci/external-inputs.sha256:1 の serina1-serum2.zip の役割。artifacts.lock に無く、runner にも無い。
- 製品のページの SharedArrayBuffer の前提(クロスオリジンの分離)。この読みでは確かめていない。
- 1 プラグイン 1 wasm の大きさと起動時間。測っていないので、推定の材料が無い。

## 次の手順の案

以下はすべて推定。監査・受付が決める。

(a) 作業ジョブが入力を取る経路を、extract ジョブと同じ形で足す。R2_ARTIFACT_* の 1 組は、バケットごと読める可能性があるので、渡す job と段を最小にする(外部入力を取る段だけ。推定)。
(b) オプトインの link を既定で有効にする(台帳 L-002)。対象は wrp1_vst3.c と wrp1_vst3_imp.c の link、fle_realmod.h の path table、fle_ole.c の CSIDL の拡張、fle_modules.h の -DFLE_GMHEX_RETURNS の扱い。製品に #ifdef を足さずに、ビルドの表を与える形で行う(推定)。
(c) プラグインの処理は、C-2 の音声の経路(音声を作る Worker)の後に入れる。取得とコンパイルは音声の Worker の外で行う(計画の論点 5。推定)。
(d) 1 プラグイン 1 wasm の試行で、vst3a の harness の wasm を使うか、realmod 経路を使うかを決める(推定)。
(e) 入力が取れた後に、C-5 の実測を行う(既定の経路で、1 ブロックの音声を処理し、P3 の underrun を n=3 で測る)。
