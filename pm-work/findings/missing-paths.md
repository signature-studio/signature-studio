# missing-paths: engine が要求して volume に無かったパス(C-3, #1316)

この文書は、`tools/missing-paths/measure.mjs` の 3 回の起動で、engine が要求して VFS(volume)に無かったパスの記録です。件数は「要求が失敗した回数」です。「FL Studio に必要なファイルが無い」という判定ではありません。判定の根拠が無い点は「未確認」に書きます。

## 測定の条件

- 道具: `tools/missing-paths/measure.mjs`。起動は `tools/boot-check/boot-bundle.mjs` の `bootBundle`(受け入れと同じ起動)。engine-idle の後にログを読む。
- 記録: `lift/runtime/fle_vfs.c` の `fle_vfs_note_missing`(VFS の 20 箇所)と `lift/runtime/fle_load.c` の `not_found`(LoadLibrary の 7 箇所)。1 回の失敗で 1 行、engine のログ(stderr)に出る。
- 行の形: `missing-path: <API> <code> "<path>"`。code は 2(ERROR_FILE_NOT_FOUND)・3(ERROR_PATH_NOT_FOUND)・126(ERROR_MOD_NOT_FOUND)。path は guest が渡した文字列(印字できない文字と引用符は `\uXXXX`)。
- runner の値(この機械とは別枠): nproc 4、受け入れの開始時の loadavg 0.39 0.27 0.20。

## 結果(3 回の起動)

| 起動 | state | standIn | error | elapsedMs | 欠落の行数 |
|---|---|---|---|---|---|
| 1 | engine-idle | false | null | 9219 | 1279 |
| 2 | engine-idle | false | null | 9300 | 1279 |
| 3 | engine-idle | false | null | 9261 | 1279 |

- 起動確認(`boot-bundle.mjs`、受け入れの前段): elapsedMs 9416、loadavg 1.23 1.39 1.95。
- 3 回の欠落は、パスごとの件数が同じです(相異なるパス 410、相異なる (API, code, path) 411)。並びは 57 箇所が回ごとに違いました(推定: engine 内の処理の順序)。

## 内訳(1 回の起動あたり、1279 行)

| API | code | 行数 |
|---|---|---|
| GetFileAttributesW | 2 | 1015 |
| GetFileAttributesW | 3 | 228 |
| GetFileAttributesExW | 3 | 13 |
| GetFileAttributesExW | 2 | 1 |
| FindFirstFileW | 2 | 13 |
| FindFirstFileW | 3 | 4 |
| LoadLibraryW | 126 | 3 |
| CreateFileW | 3 | 2 |

パスの接頭辞による目安の分類(1 回の起動あたり。分類は推定):

| 分類 | 行数 | 読み(推定を含む) |
|---|---|---|
| 空の名前 `""` | 142 | GetFileAttributesW 129 行、GetFileAttributesExW 13 行。呼び出し元が空の名前で問い合わせている |
| `Data\Patches` の下 | 591 | 大半は、各 patch の `.nfo`(情報ファイル)の確認。実機でも無い patch があり得る |
| `C:\Windows\Fonts` | 133 | 実機の Windows には有るフォントが多い(推定)。volume に無い |
| ユーザーの FL Studio フォルダ | 177 | 設定・プリセット・ログ。初回起動で作られる物を含む |
| FL Studio 21 の木(`Data\Patches` 以外) | 138 | インストール木。下の「インストール木」の表 |
| 相対名 | 36 | `.nfo` の名前、と LoadLibraryW "WebView2Loader.dll"(126) |
| Downloads | 24 | `C:\Users\user\Documents\Image-Line\Downloads` の下 |
| VST・CLAP の既定フォルダ | 16 | この環境に無い |
| その他 | 22 | `C:\Users`・`C:\Windows` などの最上位 |
| 合計 | 1279 | |

### インストール木(#1317 と L-033 の (3) に関係する物)

seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)との照合。seed 表に無いものは、表に無いと書きます。インストーラの中に有るかは、この測定では分からない。

- 言語: `System\Languages\en.moe` と `System\Languages\<版>\en.moe`(code 2・3)。seed 表にある言語は `System\Languages\en.svg` の 1 件だけ。
- テーマ: `Artwork\Themes\__current.flstheme`(code 3)。seed 表に無い。
- テンプレート: `Data\Templates\Minimal\Basic with limiter\Basic with limiter.NFO`(CreateFileW 3、2 行)。seed 表の `Data\Templates\` は空のフォルダ。
- デモ: `Data\Demo projects\Demo songs\`(code 2、2 行)。seed 表に無い。
- 本体: `FLEngine_x64.dll`(code 2、5 行)と `FL64.exe`(GetFileAttributesExW 2)。どちらも seed 表に無い。lift された本体のイメージが volume の外に有るかは、この測定では確かめていない。

## 未確認: 要求の時点で seed 済みの木が見えていない箇所がある

この測定の欠落には、seed 表の木の中にある物が含まれます。例: `Artwork\Skins\Default` は seed 表に 3 ファイルあるのに、要求の時点で「無い」と返りました。確認のため、同じ道具で 1 回だけ起動し、ログの順序を見ました(一時ファイルで実行し、削除しました。行番号はその起動のもの)。

- 行 44〜46: `vfs seed: C:\Program Files\Image-Line\FL Studio 21\Artwork\Skins\Default\...`(seed 済み)
- 行 268: `vfs seed: done, 246 files, 51 directories created`
- 行 299: `missing-path: GetFileAttributesW 2 "C:\Program Files"`
- 行 300: `missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line"`。code 2 は最後の要素だけが無いことなので、この時点で `C:\Program Files` は有る。つまり、行 299 と 300 の間に `C:\Program Files` が作られたことになる(推定)。
- 行 336: `missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Artwork\Skins\Default"`

読み: 要求の時点の volume は、seed の後の状態と一致していない可能性がある。原因は未確認です。確かめるべきことは次のとおり。(a) seed の後、engine-idle までに、木を消す・作り直す処理が走るか。(b) 要求が見る volume が、seed を置いた物と同じか。(c) survey-c3-fl-resources.md の表題 `in ?trial mode?` と関係するか。

この未確認が解けるまで、上の一覧は「要求の時点で volume に無かったパス」として扱います。「FL Studio に必要な物が無い」とは言いません。

## 性能(未確認)

- 1 回の起動で 1279 行を追加する。engine のログは stderr として page の engineLog に入り、EngineLogView が 100 ms ごとに表示を更新する。
- 起動時間への影響は未確認。記録の無い対照の起動が無いため、elapsedMs(9219・9300・9261 ms)を比べる基準が無い。

## 見つからなかったパスの一覧

回数は 1 回の起動あたりの行数。3 回の起動すべてで同じ回数だった。空の名前は `（空）` と書いた。

| API | code | パス | 回数 | 起動 |
|---|---|---|---|---|
| GetFileAttributesW | 3 | （空） | 129 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\ILGlyphsEx.ilfont` | 25 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\Share-Regular.ttf` | 19 | 1・2・3 |
| GetFileAttributesW | 2 | `.nfo` | 18 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\OpenSans-Regular.ttf` | 17 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\MS Sans Serif.ttf` | 14 | 1・2・3 |
| GetFileAttributesExW | 3 | （空） | 13 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\PT_Sans-Web-Regular.ttf` | 8 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\MS Sans Serif.ttf` | 7 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\Barlow-Medium.ttf` | 7 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\OpenSans-CondLight.ttf` | 7 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Impulses\Default.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\1-bar overdub recording.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Big meter.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Chorus send.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Default.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Dual joypad control.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Flanger send.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\General purpose air mastering.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\General purpose mastering 2.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\General purpose mastering 3.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\General purpose mastering 4.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\General purpose mastering.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Guitar stuff.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Mastering for strings.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Mono tape simulation.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Outside the club.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Reverb send.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Stereo tape simulation.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets\Techno pounder.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\4 steps.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\DrumRoll 1.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\DrumRoll 2.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\DrumRoll 3.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Randomness.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\1st_listen.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Activate.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\AllYourBase.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\alphabet_soup.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\AreYouReady.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Arpegiator.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Barty1.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Barty2.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Bicycle.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\bump.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Burning_ears.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\CantHoldMeBack.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\CheckYourReality.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\chopper.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\clouds.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\ComeOnDJ.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\countdown.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Cyber_me_Large male.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Cyber_me.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Default.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Devistating.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\EastCoast.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\EasyOnTheCut.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\electrogroove.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\FeelTheRythym.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\FlowWiththeMusic.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\FreakMeBaby.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\fruity feeling.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\FruityFun.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\FunkyMusic.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\GoodbyeSoberDay.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Hardman.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\higherstate.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\human beatbox.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\IHaveFreedom_long.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\ItSeemsWeHaveAProblem.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\IwouldLikeToIntro.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\LadiesAndGentlemen.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\LetMySoulFlyFree.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\LiquidAdrenalin.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\LockItDown.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\London.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\m0000nit.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\NoneOfThemKnew.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\OK.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\OneMansDream.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Ooooooooooooo_Giant male.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Oxygene_Robotoid.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Pilchards on toast.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\PoundTheBeat.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\ReeKuf_Large male.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Retrovertigo.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\RippedIt.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\rock me.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\RockMeBaby.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\ShutYoMouth.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\SingItWithMe.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Smoking.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\StealMyBodyHome.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Superstar.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Targetlocked_Colossus.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\TestLow_hi.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\TestLow_Tipsy.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\TheFutureIsNow.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\TimeToGroove.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\Transmission.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\UKbeat.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\WestCoast.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\WhatsNewPussyCat.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\WhoNeedsMonoRendering.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech\YouNeedAHaircut.nfo` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\QuickFontCache_x64.dll` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.dll` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\Fruity microfont.ttf` | 6 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll` | 5 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\PT_Sans-Narrow-Web-Regular.ttf` | 5 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\segoeui.ttf` | 5 | 1・2・3 |
| GetFileAttributesW | 2 | `dsp_ippv2_x64.dll` | 5 | 1・2・3 |
| GetFileAttributesW | 2 | `QuickFontCache_x64.dll` | 5 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\dsp_ippv2_x64.dll` | 4 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Backup.nfo` | 4 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Project bones.nfo` | 4 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Templates.nfo` | 4 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Skins\Default\ScopeGradient.bmp` | 4 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Blend2D.dll` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Demo projects\NewStuff.nfo` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Soundfonts\STR_Ensemble.nfo` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\Fruity microfont.ttf` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Blend2D.dll` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\dsp_ippv2_x64.dll` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line.nfo` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Internet\Default news.rss` | 3 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Support\Logs\Startup\Startup 2021-01-01_000000.log` | 3 | 1・2・3 |
| GetFileAttributesW | 3 | `Shared\QuickFontCache_x64.dll` | 3 | 1・2・3 |
| CreateFileW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Templates\Minimal\Basic with limiter\Basic with limiter.NFO` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `Blend2D.dll` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\CLAP\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Demo projects\Demo songs\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\ILTools_x64.dll` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Recent files.scr` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\VST3\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\VstPlugins` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Help\HelpAliases.scr` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Help\HelpPlugins.scr` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Envelopes\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\LFO\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Maps\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Samples\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Recent files.scr` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_ALL.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_PLUGINS.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_PRESETS.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_PROJECT.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_SOUNDS.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tab_STARRED.tabconfig` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Envelopes\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\LFO\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Maps\` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Metronome2.wav` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\PreviewTrack.fst` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\waveform colors.bmp` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Internet\Versions.xml` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Lists\Chords.map` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Lists\MP3Genres.txt` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Mapping\Vel curve 0.env` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Mapping\Vel curve 1.env` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Themes\__current.flstheme` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Typing to piano\Layout Piano.scr` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\Arial Black.ttf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `c:\windows\fonts\fruity microfont.ttf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\JosefinSans-SemiBold.ttf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `c:\windows\fonts\ms sans serif.ttf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\segoeuisl.ttf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\WebSymbols-Regular.otf` | 2 | 1・2・3 |
| GetFileAttributesW | 2 | `freetype_x64.dll` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Common Files\CLAP\` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Common Files\Steinberg\VST2` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Common Files\VST2` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Common Files\VST3\` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Artwork\Themes\__current.flstheme` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\26.1.7.5653\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\26.1.7\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\26.1\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\26\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\en\LC_MESSAGES\Default.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Steinberg\VstPlugins` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\26.1.7.5653\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\26.1.7\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\26.1\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\26\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Languages\en\LC_MESSAGES\Default.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Scores\` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Themes\__current.flstheme` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Samples\` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\26.1.7.5653\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\26.1.7\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\26.1\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\26\en.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\en\LC_MESSAGES\Default.moe` | 2 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Skins\Default\Title.png` | 2 | 1・2・3 |
| LoadLibraryW | 126 | `imm32.dll` | 2 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.en` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.EN` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.en-US` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.ENU` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.en` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.EN` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.en-US` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.ENU` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.en` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.EN` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.en-US` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\QuickFontCache_x64.ENU` | 1 | 1・2・3 |
| FindFirstFileW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\UserTabs\*.tabconfig` | 1 | 1・2・3 |
| FindFirstFileW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Reference scales\*.*` | 1 | 1・2・3 |
| FindFirstFileW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\UserTabs\*.tabconfig` | 1 | 1・2・3 |
| FindFirstFileW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Installed\*.*` | 1 | 1・2・3 |
| FindFirstFileW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Reference scales\*.*` | 1 | 1・2・3 |
| GetFileAttributesExW | 2 | `C:\Program Files\Image-Line\FL Studio 21\FL64.exe` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Artwork` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Artwork\Skins` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Artwork\Skins\Default` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Artwork\Wallpapers` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Demo projects` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Channel envelopes\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Channel presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Envelopes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Impulses` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Misc` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Mixer presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Packs` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Soundfonts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Speech` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Projects\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\System` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\System\Articulator` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\System\Metronome2.ZGR` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Templates` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Data\Templates.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\freetype_x64.dll` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Help` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Delay 2` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Filter` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Generators` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Generators\MIDI Out` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Generators\MIDI Out\Data` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\Arial Black.ttf` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\segoeui.ttf` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\freetype_x64.dll` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.dll` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Backup\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Clipboard files\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Project bones\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Projects\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Recorded\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Rendered\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Browser\Infos\Sliced audio\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Mapping` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Toolbar presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Typing to piano` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Hardware specific` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Internet` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Internet\About` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Internet\Versions.xml` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Texts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Program Files\Image-Line\FL Studio 21\System\Texts\About` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\AppData` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Artwork` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Artwork\Thumbnails` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Recorded.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Rendered` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Rendered.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Shapes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Sliced audio` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Sliced audio.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Help` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Channel presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Channel presets.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Chord progressions` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Clipboard files` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Clipboard files.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Envelopes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\LFO` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Maps` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Envelopes\Samples` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Mixer presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Mixer presets.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Effects.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Generators.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Installed.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin presets.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Speech` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Speech.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Backup` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Project bones` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Audio scripts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Cache` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\CacheDate` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Customization` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\LibraryTags` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\SearchCache` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\Tags` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Browser\UserTabs` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Envelopes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\LFO` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Maps` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\shortcuts.txt` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\FL Studio Remote Scripts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Hardware` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Internet` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Mapping` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Mapping\Generic` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Note color presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Note color presets\Auto saved.ncp` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Note name presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Piano roll scripts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Skins` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Themes` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Toolbar presets` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Touch keyboard` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Typing to piano` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Support` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Support\Logs` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Support\Logs\Startup` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Users\user\Documents\Image-Line\FL Studio\Support\Logs\Startup\` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\batang.ttc` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\ILCursors.ilfont` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\ILLogos.ilfont` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\malgun.ttf` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\msyh.ttc` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `c:\windows\fonts\segoeui.ttf` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Fonts\seguiemj.ttf` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Image-Line` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `C:\Windows\Image-Line\4096-{1C588F8C-DCE4-4D23-BFD8-2E006150E3EF}\Zip` | 1 | 1・2・3 |
| GetFileAttributesW | 2 | `ILTools_x64.dll` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `\en_US\LC_MESSAGES\default.moe` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Artwork\Fonts\ILGlyphsEx.ilfont` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Effects.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Effects\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Generators.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Generators\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Installed.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Plugin database\Installed\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Thumbnails\edition-allplugins.png` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Thumbnails\edition-fruity.png` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Thumbnails\edition-producer.png` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Config\Thumbnails\edition-signature.png` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Languages\en_US\LC_MESSAGES\default.moe` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Program Files\Image-Line\FL Studio 21\System\Plugin Formats\formats.txt` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\AppData\Roaming` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\AppData\Roaming\Image-Line` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\AppData\Roaming\Image-Line\Edge` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\FL CLOUD Sounds.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\FL CLOUD Sounds\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Optional Features` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Packs.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Packs\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Plugin presets.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Plugin presets\` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\Downloads\Scores.nfo` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Artwork\Screenshots` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Audio\Recorded` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Channel envelopes` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Presets\Plugin database\Installed\.Plugins.ini` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Projects\Templates` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Data\Articulator\Samples` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\en\LC_MESSAGES\default.moe` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Lists` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Users\user\Documents\Image-Line\FL Studio\Support\New plugin archive` | 1 | 1・2・3 |
| GetFileAttributesW | 3 | `C:\Windows\Image-Line\4096-{1C588F8C-DCE4-4D23-BFD8-2E006150E3EF}` | 1 | 1・2・3 |
| LoadLibraryW | 126 | `WebView2Loader.dll` | 1 | 1・2・3 |

## 再現

```
npm run build --prefix app
cargo build --release --locked --manifest-path lift/Cargo.toml
rm -rf lift/out-app/b2d-src lift/out-app/gen
lift/target/release/fe-build engine
lift/target/release/fe-build bundle
node tools/missing-paths/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > missing.jsonl
```

stdout は 3 行の JSON(`state`・`standIn`・`error`・`elapsedMs`・`missing`)。stderr は `run <n>: <API> <code> "<path>"` の行(読むための出力)。
