# Blocks のサンプル(#1366): 測定の記録

この文書は、Blocks.flp を開いたときの窓(「Problems loading the project」)の原因と、サンプルの配信の結果を記録します。
数値は runner の値です(この機械とは別枠。nproc 4)。

判定: **受理**(受け入れの判定どおり。Blocks を開いて `projectDialogs` が空、`error` が null。出力は 6)。
未確認: Blocks の Basic 808 系の探索(5)。起動前の量は、manifest の増分 約 5.9 KB がある(4.2)。

## 1. 欠けていた物(Blocks が要求した 4 件)

Blocks.flp のサンプル参照(`%FLStudioFactoryData%` の下)。一覧 `inputs/fl-installer-files.tsv` と照合した(大きさと SHA-256 は一覧の値。作業の木の実物の SHA-256 も一覧と一致する)。

| 参照(`Data\Patches\Packs\Legacy\Drums\Dance\` の下) | 大きさ (B) | SHA-256(一覧) |
|---|---|---|
| `DNC_Kick.wav` | 5,254 | 3b53ec2451d0d3ecddf4ab968467e87170910b094f6590a4ff9947069eece38b |
| `DNC_Clap_5.wav` | 14,448 | 7b4bf3b93cb017c33d1358ba7902f71167ee35248aa8bc5571404dd375c29cf1 |
| `DNC_Hat.wav` | 4,856 | b427aaa6ae50f1d5c3bd37a07d12a388fd9f4b3995397050174e71935b107383 |
| `DNC_Snare_4.wav` | 6,088 | 5bd8a48ca10addd1cd706d7f89ec028308318a200a646b888d9c28d96b3b1d03 |

4 件の計は 30,646 B。Blocks.flp(57,737 B)は一覧と一致する。

## 2. 配信の範囲

- フォルダ `Data\Patches\Packs\Legacy\Drums\Dance\` は 33 件・642,458 B。
- 配信するのは `DNC_*.wav` の 21 件・193,734 B(この Issue の範囲)。`pack.args` と seed 表に入る。起動の後の取得。
- 範囲の外として残したもの:
  - 同じフォルダの 12 件・448,724 B(`Basic 808 Clap.wav`・`Basic 808 HiHat.wav`・`Basic 808 Kick.wav`・`Basic 808 Snare.wav`、`CB_Clap.wav`・`CB_Hat.wav`・`CB_Kick.wav`・`CB_Snare.wav`、`Clap Basic.wav`・`Hat Basic.wav`・`Kick Basic.wav`・`Snare Basic.wav`)。取り出しの段では作業の木に入るが、bundle には載せない。理由は、Issue の範囲の決め方(21 件)。4 の 5 を参照。
  - `Data\Patches\Misc\Used by demo projects\` の DNC 2 件、`Data\Patches\Packs\Legacy\Instruments\Strings\` の 2 件。
  - `Data\Patches\` の残り(#1345 の範囲外)。

## 3. 変更

- `lift/harness/seeds/vfs-seed-idle1.tsv`: 21 行を `@lazy-file:` で足す(SHA-256 は一覧の値。作業の木の実物と照合した)。
- `lift/harness/web/pack.args`・`lift/crates/fe-build/tests/plan/bundle.txt`: 21 組の `--lazy-file`。
- `app/src/bundle/manifest.ts`: `lazy` の `maxItems` を 64 から 128 へ。理由: 遅延の件数が 75 になった(前は 54 件。これに DNC の 21 件を足した)。`pack-assets.mjs` の検証が `/lazy: must have <= 64 items` で bundle を止めた。**app/ の変更なので、main の push で `deploy.yml` が動く。**
- `tools/p3-audio-probe/measure.mjs`:
  - 開いた後の窓を `projectDialogs` に、欠けた `Data\Patches\` の `.wav` の要求を `projectMissing`・`projectMissingLines` に記録する。
  - Welcome の窓は Escape で閉じる。Default の行は開かない(4.3)。その後 Ctrl+O で開く。
  - 診断の行(stderr)を足す(Ctrl+O の前の窓の一覧、開けなかった時のエンジンの状態)。

## 4. 測定

### 4.1 Blocks を開く(受け入れの probe)

- `error`: null。`projectDialogs`: []。開いた後の題名: `blocks.flp - FL Studio 2026`。
- `project: 0 problem box(es), 60 distinct factory sample request(s) not found (64 lines)`。
- 60 件は、すべて `Basic 808` 系 4 名(各 15 フォルダへの探索)。`DNC_*` は 0 件。
- 音の出力(10 秒): `quantaDelta` 3,448、`underrunDelta` 0(音が鳴っているかの判定ではない。参考)。

### 4.2 起動前に取る量(`tools/delivery-split/measure.mjs`、3 回)

- `preBootBytes`(preStart の合計): **310,573,509 B**(3 回とも同じ)。内訳: engine wasm 165,083,600 B・seed pack 66,707,009 B・DLL pack 77,444,112 B・engine glue 1,338,788 B。遅延の行は preStart に入らない(確認済み)。
- 前回の値: 310,588,073 B(前回の bundle)。差 −14,564 B は、engine の版が違う(main の取り込み)ことによる。この測定では、差の内訳は分けられない。
- `elapsedMs`(engine-idle まで): 11,237.5・10,870.8・10,974.1 ms(loadavg 0.50 0.89 1.48)。前回: 10,642.1・10,683.5・10,798.2 ms(loadavg 2.08 1.41 1.33)。engine の版と負荷が違うので、差の原因は分けられない。
- **増分(推定)**: `manifest.json` は 31,774 B。21 件の遅延エントリは約 5,866 B(整形なしの概算)。manifest は起動の前に読まれる(推定。preStart の一覧には入らない)。起動前に取る量は、約 5.9 KB 増える。対象は 310 MB に対して 0.002%。P1 の時間への影響は無視できる(推定)。

### 4.3 シーン(`tools/delivery-split/measure.mjs` の scene)

- 3 回とも scene の `error`: `the project open: not within 120000 ms`(state null)。Welcome の Default の行を開いた後で止まる。
- 同じ行を開く経路を probe で確かめた: Default の行を開いた直後に、エンジンが止まった(`Engine: stopped (stopped, exit code 134)`)。エンジンの記録は次のとおり。
  - `[err] missing-path: LoadLibraryW 126 "Python\Python312.dll"`
  - `[err] fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no frame-based handler took it`
- scene が失敗した時の画面(scene の写真。保存後に削除): Welcome の窓が残り、エンジンの行は「computing」、最後の入力は Default の行(235, 117)。scene では止まって見えて、120 秒で失敗した(probe では exit 134 になった)。どちらも、Default の行の読み込みで止まる。
- 前回の報告の「`state exited after the scene`」と同じ現象。原因は、Default の行の読み込み(`Python312.dll` を読めず、処理されない例外)。
- 疑う commit(**未確認**。二分探索はしていない。engine のビルドは 1 回 約 33 分): 前回の bundle(03:48 UTC)の後に main に入った `0f753ff4`(DPI 240 の修正。runtime の応答の変更)、`305bee51`・`2107e2d0`(#1362 の WebView2 の面)、`380922c0`(dsp_ippv2 の閉路)。
- 4.2 の値は scene より前に取った。scene の値は確認できない。

### 4.4 起動の確認

- `tools/boot-check/boot-bundle.mjs`: `state` engine-idle・`standIn` false・`error` null。`timelineIdle` 10,814 ms。

## 5. 未確認と、監査・受付への判断

- **Basic 808 系**: Blocks の探索は、`Basic 808 Kick/Clap/HiHat/Snare.wav` の 4 名を、各フォルダ(最初は `Packs\Legacy\Drums\Dance\`)へ探す(60 件の行)。このときは窓が出ない。Blocks のどのチャンネルがこれを鳴らすかは、この測定では分からない(未確認)。4 件(182,132 B: Kick 74,468・Clap 46,492・HiHat 28,180・Snare 32,992)を足すと、lazy が 4 件増える。遅延なので起動前の量は変わらない(manifest の増分を除く)。配信の範囲の判断は監査・受付が行う。
- 起動前の量の増分(manifest 約 5.9 KB)は、Issue の「起動前に取る量は増やさない」と厳密には一致しない。判断は監査・受付が行う。
- 4.3 の engine の停止(Default の行)は、#1342 の scene と同じ。新しい回帰の可能性があり、疑う commit は 4.3 の候補。

## 6. 受け入れの出力(この run。07:50 UTC。`ACCEPT EXIT=0`)

```
boot-bundle: state engine-idle, standIn false, error null
measure.mjs (Blocks): error null, projectDialogs [] (0 problem box(es)), 60 distinct factory sample request(s) not found (64 lines), project "blocks.flp - FL Studio 2026"
ACCEPT EXIT=0
```

`fe-build bundle` は、`lazy` 75 件(DNC 21 件を含む)で通った。
