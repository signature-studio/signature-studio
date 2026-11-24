# persist-registry: 言語の選択の保存先と、再読込をまたぐ保存(#1383)

この文書は、`tools/persist/measure.mjs` の実行(runner の値。この機械とは別枠。nproc 4、loadavg 1.2〜2.6)と、その前後の確認の記録です。根拠の無い点は「未確認」と書きます。

## 言語の選択の保存先(実測: レジストリ)

- 言語を変えると、エンジンの trace(`window-api:` 行)に次が出た(4 回とも同じ形。言語 de・es・ko・zh)。
  - `RegSetValueExW "HKCU\Software\<ベンダー>\Shared\Language" "Program language" type=1 size=6 value="de?"`(`?` は trace の表記で、NUL 1 文字)
- 再読込の後の起動で、`RegQueryValueExW "HKCU\Software\<ベンダー>\Shared\Language" "Program Language" -> found type=1 size=6 value="de?"` と読まれた(名前は大文字小文字を区別しない)。4 言語とも、書いた値と同じ値が読まれた(`registryKept`)。
- ファイルへの書き込み(言語の `.moe` の保存)は、前回の測定では見つからなかった。この測定の道具は、レジストリの行だけを取っている(ファイルの書き込みの trace は未確認)。

## 再起動の案内

- 言語を変えると、「Language change」の窓(`TMsgForm`、トップレベルの新しい窓)が出る。道具はこの窓の題名で判定する(4 回とも `restartPrompted: true`)。

## 再読込の後の停止(実測)

| 言語 | 変更の後の保存(trace) | 再読込の後に読まれた値 | 再読込の起動の停止 |
|---|---|---|---|
| es | `value="es?"` | `found ... value="es?"` | `missing-path: LoadLibraryW 126 "ILSvg_x64.dll"` の後に `fle: indirect call to 0 is not a known function entry`(`stopReason: missing-ilsvg`) |
| de | `value="de?"` | `found ... value="de?"` | `fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (not defined by the MSDN page or outside the emulated locale; not implemented)`(`lcid=0x400`、`"Überprüfe, ob alle Plugins vorhanden sind"` と比べた) |
| ko | `value="ko?"` | `found ... value="ko?"` | 同じ CompareStringW(`string1="플레이리스트"`、`string2="플러그인 렌더"`) |
| zh | `value="zh?"` | `found ... value="zh?"` | 同じ CompareStringW(`string1="淡入淡出"`、`string2="渲染插件"`) |
| ja | `value="ja?"` | `found ... value="ja?"` | CompareStringW(`string1="元に戻す/やり直し"`、`string2="制限をチェックする"`) |
| vi | `value="vi?"` | `found ... value="vi?"` | CompareStringW(`string1="Mờ dần"`、`string2="Mở tất cả các dự án demo"`) |

- ILSvg の停止は、`System\Languages\<code>.svg` と `ILSvg_x64.dll` の欠落(`missing-path` の行)。CompareStringW の停止は、英語以外の文字列の比較が、エミュレーションで未実装であること。どちらも保持の範囲の外で、#1395 が扱う。
- 英語の状態の再読込は止まらない(`defaultReloadState: engine-idle`。9,161〜9,274 ms)。
- fr も、この試行で ILSvg の停止(`stopReason: missing-ilsvg`、`error` null、`registryKept` true、`bytes` 1,687 B)。
- この試行の 7 言語(各 1 回)の停止: ILSvg が es・fr、CompareStringW が de・ko・zh・ja・vi。前回の試行では ja・vi が ILSvg だった。同じ言語でも止まり方が変わる(run や build による。未確認)。
- したがって、乱数の受け入れが `stopReason: missing-ilsvg` で通るのは、この試行では 7 件中 2 件(es・fr)だけ。

## 保存の領域

- `/fl/state/hkcu.log`(OPFS)。変更の後、再読込の前: 1,318 B(4 言語とも同じ。記録の大きさは言語の値の長さで決まる)。
- 停止の後の回復の起動で、保存は捨てられる(`closeRegState`)ので、再読込の後の大きさは 0 B。
- 上限は 100 MiB(`REG_STATE_LIMIT`)。超えると書かない(ログに出る)。

## 保存の時期と方式

- 値・鍵・削除の都度、`fle_reg.c` の `reg_journal` が 1 件を `fle_reg_page_put`(同期の import)で渡し、Worker が同期のアクセスハンドル(`createSyncAccessHandle`)で追記して flush する。ゲストの呼び出しは、書き終わってから戻る。
- 確認: 変更の後、ページを再読込すると(Worker は作り直される)、保存された値が読まれた。書き終わりは、ページを閉じる前に済んでいる。
- 起動の前に、`fle_reg_restore_saved` が、保存された記録を seed の値の後に再生する(保存された値が seed の値を上書きする)。再生の間は記録しない。
- 対象は `HKCU` の木の全て(鍵・値・型・データ)。`HKLM`・`HKCR` などは保存しない。
- 保存は利用者のブラウザの中だけ。外へ送らない(ビーコン `app/src/perf/` に値を入れない)。

## ファイル管理の窓での見え方(決定と理由)

- `/fl/state` は `app/src/files/paths.ts` の `PROTECTED_PATHS` に既に在る。ファイル管理の窓では見える。名前の変更・移動・削除の前に、`protectedHit` で確認の問いが出る(`app/src/files/explorer.ts`。止めはしない)。
- 理由: 隠すと、利用者が保存の場所を知る手段が無くなる。見せて確認を挟むと、誤って消して設定を失うことを防げる(`/fl/state` の扱いは、既存の `/fl/license` と同じ)。

## 直した方式と、残っている停止

- 保存: 上の方式(HKCU の木の全て、変更の都度、OPFS の 1 ファイル)。
- 安全策(暫定): エンジンが異常に終わった回(trap、または非 0 の終了)は、保存を捨てる。捨てないと、言語を変えるたびに起動が止まり続ける。捨てると、言語の選択は失われる(クラッシュした回の設定)。#1395 が直すまでの暫定。
- 残り: 英語以外の言語で起動すると止まる(ILSvg の欠落と、CompareStringW の未実装)。これは #1395。

## 受け入れの判定で見えたこと

1. (解消済み)`fe-build bundle` は、インストーラが無いときは `pack.args` の plugins の `--lazy-file`(`lift/out-app/plugins/ott/vst-ext.wasm`)を省く(main の commit 4d73666a)。前の試行(古い main)では、この行で落ちていた。この試行は HEAD 82b38b07 のまま、仮の物を置かずに bundle `de8572dd79ce` を作った。ログに `fe-build: 外部プラグインのインストーラが無いので、省く: lift/out-app/plugins/ott/vst-ext.wasm=plugins/ott.wasm` が出た。
2. 受け入れの言語は乱数で選ぶ。再読込の後の停止(ILSvg・CompareStringW・その他)は、道具が `engineStoppedAfterReload` と `stopReason` に記録し、`error` を立てない。判定は `registryKept`・`restartPrompted`・`defaultReloadState` などを見るので、停止の種類によらず通る(#1395 が直る前でも)。

## 測定の値(runner の値。この機械とは別枠)

この試行(HEAD 82b38b07。nproc 4、loadavg 2.04・1.42・1.66)の値。
- 言語 vi: 変更の trace `RegSetValueExW "HKCU\Software\<ベンダー>\Shared\Language" "Program language" type=1 size=6 value="vi?"`、再読込の後 `RegQueryValueExW … "Program Language" -> found type=1 size=6 value="vi?"`(`registryKept` true)。`restartPrompted` true(窓「Language change」)。
- 再読込の後の停止: `stopReason` `compare-string`(`fle: CompareStringW: lcid=0x400 … string1="M\u1EDD d\u1EA7n"`)。`error` null。道具が回復の起動をして、`recoveredIdle` true。
- 既定(英語)の再読込: `defaultReloadState` `engine-idle`(6,195 ms)。
- `bytes` 1,318 B(変更の後、再読込の前)。`bytesAfterReload` 0 B(停止した回の保存は捨てられる。上の「安全策」)。
- `preBootBytes` 311,115,470 B(上限 314,368,711 B 以下)。`elapsedMs`(最初の engine-idle まで)7,782 ms(上限 12,500 ms 以下)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 回とも `engine-idle`、`error` null、`trapped` false、`hostErrors` 0、`welcomeClosed` true、`opened` true(elapsedMs 7,617〜7,834 ms)。
- 単体テスト(`npm test --prefix app`): 70 files passed(1 skipped)、1,382 tests passed(10 skipped)。
- 判定式(受け入れの `node -e`)の合成データの確かめ: 通る 3 件(停止 compare-string・missing-ilsvg・other)と、落ちる 4 件(`registryKept` 偽・`restartPrompted` 偽・既定の再読込が止まる・`error` 非 null)。7 件とも期待どおり。

前の試行(古い main。仮の物を置いた)の値: `preBootBytes` 311,115,470 B、`elapsedMs` 11,098〜11,559 ms、`bytes` 1,318 B。

## 未確認

- fr・ja・vi の、この試行での再測定(前回は ILSvg)。
- 起動が 40 秒を超えた回(de の回復の起動で 1 回。fr の最初の起動で 1 回。`startStalls` `recoverStalls` で数える。#1376 の類か)。
- 保存を捨てる範囲(HKCU の全て)を、狭めるべきか。
