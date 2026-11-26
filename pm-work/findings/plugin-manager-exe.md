# PluginManager.exe(FL の Plugin Manager): 調査と設計(#1499)

状態: 段 2 まで途中(接頭辞の 1 行だけ commit)。受け入れは未確認。

## 0. 要点

- exe の本体は、この作業のジョブの木に無い。ファイルシステム全体を `find` で調べ、`PluginManager.exe` は見つからなかった(`lift/out-app/host` と `host-extra` も同じ)。取り出しの表(`ci/extract-dlls.tsv`・`ci/resources-prefixes.txt`・`ci/plugin-builtin-prefixes.txt`)のどれにも、この実行ファイルは在らなかった(接頭辞の 1 行は、この作業で足した)。
- 作業者の段は、抽出物を main の鍵で R2 のキャッシュから復元する(`.github/workflows/agent.yml` の「抽出物を R2 のキャッシュから取る」。`fail-on-cache-miss: true`)。その鍵の入力に `ci/resources-prefixes.txt` が在る。だから、接頭辞の行が main に入るまで、exe は作業者の木に入らない。`pm-work/workflow-design.md` の 47 行目が、この順序を定めている(「新しい seed の行を足す Issue は、実物の接頭辞が `ci/resources-prefixes.txt` に在ることを、起票の前に確かめる(無ければ、先に足す。足す前に、作業者は受け入れを通せない)」)。
- 判定は、受け入れの全体(`plugin-manager-opens` を含む)が通ったときだけ push する。exe が無い木では、窓は出ないので、この Issue の受け入れは 1 つの作業者のジョブでは通らない。段を分ける案を、末尾に書く。
- 受け入れの実測(この作業のジョブ。9 節): 起動・流れ A〜D・`npm test`・`plugin-database-lists-builtins`・`file-open-demo` は通った。`plugin-manager-opens` は、両方の表示で ok が偽。受け入れの最後の判定(E2E の行の検査)で止まり、終了コードは 1。

## 1. PE の表

インストーラの一覧(`inputs/fl-installer-files.tsv` の 27399 行目)の値:

| 項目 | 値 | 出所 |
|---|---|---|
| 大きさ | 7,158,856 B | 一覧の bytes |
| SHA-256 | 66100a71d04bddf340bc8f56a1823846662ad44f4b422c7059dad4b2eb32f1e7 | 一覧の sha256 |
| 種類 | exe | 一覧の kind |
| import する DLL(16 件) | winmm, shlwapi, comdlg32, comctl32, shell32, user32, version, oleaut32, advapi32, imagehlp, kernel32, shfolder, ole32, msacm32, api-ms-win-crt-string-l1-1-0, gdi32 | 一覧の imports |

task の挙げた 11 件(winmm・shlwapi・comdlg32・comctl32・shell32・user32・version・oleaut32・advapi32・imagehlp・kernel32)は、全て一覧に在る。16 件の名前は、`lift/runtime`・`lift/harness` の C のソースに現れる(関数の単位の実装の有無は未確認)。

未確認(exe の本体が無いので `fe-pe` を走らせられない): Machine・ImageBase・サブシステム(GUI か)・エントリの RVA・`.reloc` の有無・import の関数の一覧・Delphi の RTL の目印。x86-64 か否かも、ここでは確かめられない。

## 2. VFS の置き場所: `resources` の仕組み(接頭辞の行 + seed 行 + lazy の組)

決め: `ci/resources-prefixes.txt` に、完全なパス `System\Tools\Plugin Manager\PluginManager.exe` の 1 行を足す(commit 959e71ba)。seed 行と pack の組は、接頭辞が main に入った後の作業で足す(下の理由)。

理由:
- `lift/plugins/*/data.tsv`(#1478)は、プラグインの Data(`Plugins\Fruity\<種別>\<フォルダ>\Data\`)のための仕組み。取り出しは `ci/plugin-builtin-prefixes.txt`(`Plugins\Fruity\Effects\`・`Plugins\Fruity\Generators\`)の上で動く。exe は `System\Tools\` の下なので、その接頭辞に入らない。
- `lift/plugins/*/plugin.tsv` の行は `.dll` に限られる(`lift/crates/fe-build/src/plugins.rs` の `validate`:「the file is not a DLL」)。realmod の入口は DllMain として呼ばれる(`lift/runtime/fle_realmod.c` の 235 行)。exe は同じ形に入らない(段 3)。
- `resources` の一覧は、全ての作業の段で `lift/out-app/host-extra/resources` と `lift/out-app/host/resources` に置かれる(agent.yml の「言語・テーマ・テンプレート・プロジェクトを R2 のキャッシュから取る(全ての作業)」)。deploy の「resources を取り出す」も同じ一覧を読む。だから本番の bundle の入力にできる。
- 大きさ: 一覧の完全なパスの行は、足す前が 27 件・4,830,812 B。足すと 28 件・11,989,668 B で、上限の 30,000,000 B 以下(`spec/workflow.test.sh` の (r5) が検査し、通った)。
- lazy: exe は 7 MB で、起動に要らない(Options の Plugin Manager を開いた時だけ要る)。起動の前の pack に入れると、起動前の大きさが増える。既存の大きい resources は、`lift/harness/seeds/vfs-seed-idle1.tsv` の `@lazy-file:resources/...` の seed 行(114 件)と、`lift/harness/web/pack.args` の `--lazy-file` の組で配る。exe も同じ形にする。
- 未着手: seed 行と pack の組。`app/scripts/pack-seed.mjs` は、seed 行の実物を木から読んで sha256 を照合する。接頭辞が main に入る前は、作業者の木に実物が無いので bundle が落ちる(`workflow-design.md` の「問題 A」と同じ型)。

## 3. lift(未着手。論点だけ)

既存の `plugin.tsv` の形(本体・hook・args・omit)が exe に合わない点:
- `validate` が `.dll` を要求する。
- realmod の入口は `AddressOfEntryPoint` を DllMain(`DLL_PROCESS_ATTACH` など)として呼ぶ(`fle_realmod.c` の 235 行)。exe の入口は引数無しのスレッドの本体として呼ぶ。
- hook の 3 点(Delphi エフェクトの hook)は、この exe には当たらない(`none`)。
- omit(setjmp の入口)は、exe の中身を見るまで分からない。

最小の拡張の方針: 段 1 の PE の表(Machine・エントリ・`.reloc`)が無いと、拡張の形は決められない。段 1 の前に決めると、推測で共有の表を触ることになるので、決めない。

## 4. 起動の経路(設計だけ。コードは書いていない)

現状:
- `fle_imp_ShellExecuteW`(`lift/runtime/fle_wall_ia2.c` の 152 行)は、関連付け無し(31)を返す(決定 `S-ia2-shellexecute-noassoc`)。`ShellExecuteA`・`ShellExecuteExW` は `lift/runtime/fle_api2_shell.c` で同じ。
- `GetModuleHandle(NULL)` は、engine の像を返す(`lift/runtime/fle_env.c` の 406 行)。
- `ExitProcess` は `fle_stop` で、engine を止める(`lift/runtime/fle_proc.c` の 279 行)。

使える部品:
- 協調スレッド(決定 0122、`lift/runtime/fle_thread.c`): 1 つのホストのスレッドの上に、ゲストごとのスタック。切り替えは `schedule()` と `fle_progress_slice()` だけなので、engine の本体は止まらない。`fle_thread_spawn(body, arg)`(`lift/runtime/fle_thread.h` の 173 行)で作れる。
- realmod の読み込み: `fle_realmod_find_path`(接尾辞の照合。`Dll::suffix` と同じ規則)、`fle_realmod_load`・`fle_realmod_getproc`(`lift/runtime/fle_realmod.h`)。
- 待ち: `WaitForSingleObject` は、スレッドの handle を受ける(決定 0122。`lift/runtime/fle_event.c` の 210 行)。

設計(決めることの一覧。FL がどの呼び出しを使うかは、実測で決める):
1. 呼び出し: `ShellExecuteW` の lpFile が、この exe の完全なパスの接尾辞に一致するときだけ、realmod を読み、入口を `fle_thread_spawn` の本体(`fle_call_guest(entry, ...)`)として走らせる。戻り値は 33 以上(成功)。それ以外の URL・ファイル種類は、今まで通り 31。決定 `S-ia2-shellexecute-noassoc` の範囲は、この exe に限る。
2. `GetModuleHandle(NULL)`: exe のスレッドからは exe の基底を返す必要がある(Delphi の HInstance と、リソースの読み込み)。スレッドごとの既定の像を持たせる。他のスレッドは今のまま。要決定。
3. `ExitProcess`: exe のスレッドからの呼び出しは、そのスレッドの終了として扱い、engine は止めない。他のスレッドからの呼び出しは、今のまま `fle_stop`。要決定。
4. 待ちと終了コード: `ShellExecuteExW` の `hProcess`(SEE_MASK_NOCLOSEPROCESS のとき)は、そのスレッドの handle を返す。`GetExitCodeProcess` は、スレッドの終了コードを返す。要決定。
5. コマンドラインとカレントディレクトリ: `lpParameters` は exe のコマンドラインに入れる。`lpDirectory` の既定は exe のフォルダ(`System\Tools\Plugin Manager\`)。FL の実際の値は未確認。
6. `-flpath`: FLEngine の文字列からの推定で、未確認。

観測(9 節の E2E。両方の表示で同じ): Options の 6 番目を選ぶと、FL は `C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe` を `GetFileAttributesW` で調べ、無い(`missing-path`)。その後の `ShellExecute` の行は、E2E の記録に無い(6 番目の行数は記録されていない)。つまり、FL は起動の前に、この完全なパスの存在を確かめる。段 4 の呼び出しを決めるときは、この存在の確認が通った後の呼び出し(`ShellExecuteW`・`ShellExecuteExW`・`CreateProcessW` のどれか)を、exe が在る木で実測する。ビルドの記録では、`CreateProcessW` も C の runtime で未モデルと出ている(起動の論点として残す)。

## 5. 窓(未着手)

Plugin Manager の窓のクラスと題名は、exe を動かした後に、窓の表で確かめる。今の窓の表の実測(`pm-work/findings/crash-hunt-2026-11-25.json`)は、`TPlugListForm`(題名 `PlugList`)と `TPluginMonitorForm`(題名 `Plugin performance monitor`)。Plugin Manager の窓は、まだ確かめていない。

## 6. E2E(`plugin-manager-opens`)

`tools/e2e/scenarios/smoke/plugin-manager-opens.mjs` の `expectFail: '#1499'` の印は、外していない。外すのは、受け入れで ok になったとき。今は exe が作業者の木に無いので、窓は出ず、ok にならない。実測(9 節): 両方の表示で `ok: false`・`engineStops: 0`・`engine-idle` のまま。窓は 10 秒の間に現れなかった(`no Plugin Manager window in the window table 10 s after the item`)。

## 7. 段ごとの進み

| 段 | 状態 | 中身 |
|---|---|---|
| 1 PE | 未着手(入力が無い) | 一覧の値だけ。PE の表は未確認 |
| 2 VFS | 途中 | 接頭辞の 1 行を commit(959e71ba)。seed 行と pack の組は未着手(接頭辞が main に入るまで足せない) |
| 3 lift | 未着手 | 論点だけ(段 3 の節) |
| 4 起動 | 未着手 | 設計だけ(段 4 の節)。コードは無い |
| 5 窓 | 未着手 | 窓の表は未確認 |
| 6 E2E | 未着手 | 印は残した |

## 8. 監査への依頼: 段を分ける

1. 接頭辞の 1 行(959e71ba)を main に入れる。次の extract で、接頭辞の一覧の鍵が変わるので取り直され、exe が `lift/out-app/host-extra/resources` に入る。
2. 次の Issue(または、この Issue の再試行): PE の表(段 1)・seed 行と pack の組(段 2 の続き)・lift の方式(段 3)。受け入れは、bundle が通り、`plugin-manager-opens` の手前の確認(起動の確認・流れ A〜D)まで。
3. その次: 起動の経路・窓・E2E(段 4〜6)。受け入れは、`plugin-manager-opens` が ok になるまで。

この Issue の受け入れを 1 回で通すことは、今の木では出来ない。

## 9. この作業の受け入れの実測

実エンジン: 受け入れコマンドを、この作業のジョブの木で実行した(`npm run build`・`fe-build engine`・`fe-build bundle`・boot-check・welcome-flow・`npm test`・`tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2`)。終了コードは 1(E2E の行の検査で止まった。`guard/scan.sh` は走っていない。単独では、指摘 0 件・終了コード 0)。

runner の値。この機械とは別枠。nproc 4・loadavg [2.41,1.87,1.85](流れの記録の末尾の値)。

| 検査 | 結果 | 値 |
|---|---|---|
| 起動(boot.json) | 通った | engine-idle・standIn false・error null・elapsedMs 11,338 |
| 流れ A〜D | 通った(4 行) | 4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。elapsedMs 11,144〜11,351。preBootBytes 313,271,260 |
| `npm test --prefix app` | 通った | 74 ファイル通過・1 件飛ばし。1,455 件通過・10 件飛ばし |
| E2E の 2 表示の行 | 通らない | 行は 2 つ。`plugin-manager-opens` は、両方で `ok: false`・`expectFail: "#1499"`(この Issue の受け入れは `expectFail: null` を要する) |

E2E の各シナリオ(両表示の同じ結果):
- `start-welcome-default`: ok true
- `file-open-demo`: ok true
- `plugin-database-lists-builtins`: ok true(lift/plugins/ の 2 件が一覧に出る)
- `knob-right-click-permissions`: ok false・`expectFail: "#1406"`(この Issue と無関係の、既存の既知の印)
- `plugin-manager-opens`: ok false。Options の 1〜5 は設定の窓を開く。6 番目は `GetFileAttributesW` の `missing-path`(上の観測)。窓は現れず、engine は `engine-idle` のまま 10 秒

Plugin Manager の窓の表の行(クラスと題名): 無し。

段ごとの状態は 7 節のとおり。
