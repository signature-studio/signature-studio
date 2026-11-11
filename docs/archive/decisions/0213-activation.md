# 0213: FL Studio のアクティベーションをブラウザ版で正当に行う経路の調査と設計 — 公式のオフライン認証(検証ファイル → FLRegkey.Reg → Unlock with file)が存在する。エンジンのネットワークは WinINet だけで、アカウント認証はメールとパスワードを URL のクエリに入れる。案 A(オフライン認証)を採り、機械識別・レジストリの永続・ファイル橋の要件を定めた。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0213、担当 act1、2026-11-04 03:52〜、予算 3 時間)。作業は HEAD **e7af37b** の `git archive` した木(`lift/out-act1/src/`)。`act1.diff` は新規 4 ファイル + 既存 2 ファイルへの追記 16 行(**削除 0 行**)で、最新 HEAD **0a3ab9a** の `git archive` に `patch -p1 --dry-run` が当たる(`logs/patch-dry-run.txt`。6 ファイル clean)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや資料を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**推測**＝実機や実サーバーの挙動・作業量の見積り。**出典なしの選択**＝名前つき(`S-act1-…`)で `tables/s-choices.tsv` に書いた。**実機 Windows・Chrome・Firefox・依頼者の実鍵・Image-Line のサーバーとの照合は 0 件**。速度の数字は書いていない(この機械は load 40〜55)。

## 冒頭の三点(正直さ)

1. **実行した本物と範囲(実測)**:
   - 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → **native -O0** にした物。Blend2D 置換・刈り込み・wasm 化はしていない)を、決定 0171 の fork プローブで動かした。構築 A1(層 1〜4 + 層 5)と A2(+ exp1 の保存ダイアログの行)。プローブは A1 で 17 本、A2 で保存・読み込みなど(§3、`tables/probes.tsv`)。**実エンジンが出した物**: 認証のフォーム `TUnlockForm`(Unlock on web / Unlock with account / Unlock with file…)、About の『Trial v26.1.7 [build 5653] … trial mode』、`TRedeemCodeForm`、`TInAppShopForm`、`ShellExecuteW` の 2 つの URL(`redirect/me`・`redirect/HowToUnlock`)、起動時のニュース/バージョン表の要求 2 本、検証ファイル(521 バイト)の保存ダイアログと読み込みダイアログの引数。
   - 静的な読み(§2): import・文字列・呼び元・逆アセンブル。**実行していない。**
   - 一次資料(§1): Image-Line のナレッジベース・公式マニュアル・EULA・Getting Started Manual、Microsoft Learn、MDN。取得は公開ページの GET だけ。
   - 単体テスト: `act1_net.rs` 9 件(合格)、mutation 2 本をテストが検出。**全体テスト**(`harness/test.sh full --fresh --jobs 2`、act1_net 8 件の版): results=179・passed=2,282・failed=0(基準 178/2,274/0 に +1 実行ファイル・+8 テストで一致。`logs/fulltest-act1.txt`)。最終版(act1_net 9 件)での `affected` 再実行(`--base-tree` で基準の木と比べる): 174/175 実行ファイルが対象で passed=2,282・failed=0(`logs/affected-final.txt`)。残る 1 実行ファイル `fe-lift/call_iat_dll`(1 テスト)は影響を受けず、全体テストで合格済み = 最終版の合計は **175 実行ファイル・2,283 テスト・0 失敗**(+ doc-test 4 結果 = 179 results)と読む。**a4 FRUITY 無効(基準の `a4-fruity.sh off`)は init_count 942/942・run_rank 895/895・functions_distinct 5,310・function_entries 1,009,731 = 基準と完全一致**(`logs/a4-fruity-off.txt`、gen 51 s・build 796 s、JOBS=2)。FLEngine の既定の生成 C の sha256 `cd9107ce…`(実測、不変)。
   - **模擬・推測の範囲**: 座標は PNG の読み(±3 px)、ファイル選択ダイアログの答えはハーネスが環境変数で与えた(依頼者の入力の代わり)、ネットワークは常に『届かない』、**同意のチェックは入れていない**、MachineGuid のテスト値は私が作った値。作業量は推測。**層 5 の WinINet 関数(`fle_act1_net.c`)は、実エンジンの走行では一度も呼ばれなかった**(0 回。アカウント認証は『同意』のチェックの先にあり、"Redeem code" は WinINet でなくブラウザを開く)。単体テスト(エンジンの呼び方を逆アセンブルから写した列)と静的な読みだけで確かめた。代役・画像の再生・事前生成は無い。
2. **結論**: (a) **公式のオフライン認証は存在する**(§1)。エンジンはその UI を持ち、ネットワークに出ずに開く(§3.2)。(b) エンジンのネットワークは WinINet だけで、アカウント認証は**メールとパスワードを URL のクエリに入れる**(§2)。(c) **案 A(オフライン認証)を採用**、案 B は資格情報が中継を通るので採らない(§4.1)。(d) 案 A の要件: 機械識別の安定(配備ごとの MachineGuid を種ファイルで。OPFS に依存しない)、レジストリの OPFS 永続、保存/読み込みのファイル橋(題・フィルター・初期フォルダは実測、§3.3)、再読み込み = 再起動(§4)。作業量は約 1 週間(推測)。(e) **EULA の事実**: 利用規約(https://www.image-line.com/image-line-software-eula)の "Restrictions on use" に "The User shall not … reverse engineer, decompile, disassemble or create derivative works from the Products; … make access to the Products available to others in connection with a service bureau, application service provider, or similar business" がある。本プロジェクトの lift と公開デプロイに関わる。**法的判断はしない**。設計は『依頼者のみがアクセスする私的なデプロイ』を前提にする。ブラウザ内実行・仮想環境での認証の可否を直接述べた文言は見つからなかった。
3. **未確認**: Chrome・Firefox の出荷バンドルでの確認、実機 Windows との照合、**依頼者の実鍵 `FLRegkey.Reg` でのエンジンの受け入れ**、Image-Line のサーバーがこの識別で作られた検証ファイルを受け付けるか、Sign in and unlock の結果の戻し方(案 C。同意のチェックを入れないため URL を見ていない)、`TMsgForm` の本文が描かれない原因、実機でのエンジンの MachineGuid 欠落時の検証、作業量の実測。


## 1. 公式の手順(一次資料。2026-11-04 に取得。全文は `sources/` に抜粋、URL と該当文言を下に引く)

**結論(1)**: **公式のオフライン認証は存在する。** Image-Line のナレッジベース記事 "How to unlock FL Studio from trial using the Toolbar 'HELP > Unlock FL Studio'"
(https://support.image-line.com/action/knowledgebase?ans=119)に、節 "How to Unlock FL Studio 20.9 or Later Offline" がある。**FL Studio 20.9 以降は「検証ファイル(Validation File)」を介する 2 段**であり、
旧来の「アカウントから FLRegkey.Reg をダウンロードするだけ」の手順(20.8 まで)とは違う。

### 1.1 オンライン(アカウント)での認証 — 同記事の節 "How to Unlock FL Studio on a Computer with Internet Access"

- 「FL Studio が未認証なら、起動のたびに『Unlock』ボタンのあるポップアップが出る。押すとオンラインの認証が始まり、**アカウントへのログイン**を伴う」
  (原文: "each time you start FL Studio you will be presented with the following pop-up window with a button to 'Unlock'. Clicking this will start the online unlock process, which involves logging into your account and not much more")。
- 試用ライセンスの間、または FL Studio を開いたままなら、メニュー **HELP → Unlock FL Studio → Sign in and unlock**(原文: "'(1) HELP' then '(2) Unlock FL Studio' and '(3) Sign in and unlock'")。
- 「Automated unlock」: 「**既定の Web ブラウザが開き**、そこで Image-Line アカウントの email とパスワードでログインする。その後 FL Studio が再び開き、ライセンスが適用される」
  (原文: "Your default Web Browser should open and you can enter your Image-Line Account email and password … After this FL Studio should re-open and your license will be applied")。
- ブラウザが開かないとき: **"Unlock with account"** をクリックし、**パネル上で直接**ログイン情報を入力する(原文: "If your Web Browser fails to open click 'Unlock with account' … and enter your login details directly on the panel")。
- 2 要素認証(2FA)があるときは、アカウントの主メールに届く 6 桁の確認コードを "Enter the verification code here" に入れる。
- 最後に FL Studio を再起動する(原文: "FL Studio must be restarted to complete the unlock process")。
- 公式マニュアル(https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/envsettings_about.htm)の About 節: "Unlock with account - Enter your Image-Line Account email and password … **NOTE: Requires internet access.**"。

### 1.2 オフライン — 同記事の節 "How to Unlock FL Studio 20.9 or Later Offline"(原文の手順 1〜9 を要約。引用は原文)

1. FL Studio(オフラインの機械)で "Unlock with file" を選び、"**Save validation file ...**" で検証ファイルを作る(原文: "From FL Studio 20.9 onward, you will need to upload the Validation File in order to generate a valid RegKey when asked by our server. Create one from the pop-up dialog when you use Unlock with file and choose 'Save validation file ...'")。
   エンジン内の文字列も一致する: `licensevalidation.txt`、"The validation file has been saved."、"Go to your Image-Line account to download a license. Select the validation file when our website asks for it."(FLEngine_x64.dll の UTF-16 文字列。VA 0xf00b14 / 0xf00b5c / 0xf00bc4、関数 f_f006d0)。
2. **別の(インターネットにつながる)機械で**Image-Line アカウントにログインし、"Unlock products" → "Download Offline Unlock File for FL Studio 20.9 or Later"。
3. 「this link」から RegKey ファイルの生成に進み、手順 1 の検証ファイルを **Upload** する。
4. 少し待つと **`FLRegkey.Reg`** がダウンロードされる。**原文: "This file will only work on the computer from which the Validation File was generated. You will need to upload a Validation File for each computer you need to unlock offline."**
5. `FLRegkey.Reg` をオフラインの機械へ運び、FL Studio の **HELP → Unlock FL Studio → Unlock with file → Load license file...** で選ぶ。
6. FL Studio を再起動する。成功のとき "The registration succeeded"(エンジン内の文字列 "Success|The registration succeeded." と一致)。
- 20.8 以前の手順(検証ファイル無し)も同記事にある("Older FL Studio versions and VST/AU Plugins")。公式の「Getting Started Manual」(PDF、https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/boxinstaller/WebContent/FLStudio20_GettingStartedManual.pdf)の "HOW TO UNLOCK FL STUDIO WITHOUT INTERNET ACCESS" と、公式ダウンロードページ(https://www.image-line.com/fl-studio/download)の FAQ("If that doesn't work, use the offline unlock method")も同じ。
- 公式マニュアルの About 節: "Unlock with file - Download your FLRegkey.Reg file from your Image-Line account … Click 'Unlock with file' on the About panel and browse to your file, select it and Click OK. Restart FL Studio."。

**実エンジンの UI との一致(§3.2、実測)**: `HELP → Unlock FL Studio` で開く `TUnlockForm` は、KB の記述どおり、既定のタブ "Unlock on web"(同意のチェック + **Sign in and unlock**)、"Unlock with account"(E-mail address・Password・**Unlock**)、右の "Unlock with file…"(→ **Save validation file…** / **Load license file…**)、"Redeem code…"、"Buy FL Studio…" を持つ。KB と食い違う物は見つからなかった。

### 1.3 ライセンスの使用条件・仮想環境についての記載(確認できた範囲)

- 利用規約(EULA、https://www.image-line.com/image-line-software-eula 。サインインなしで読める版。`sources/eula.txt`):
  - 設置: 公式の Getting Started Manual に "**You may install FL Studio on any computer/s that you exclusively control.**"(複数の機械へ、自分が専有して管理する限り)。EULA 本文には機械の台数の明記を**見つけられなかった**(「1 人の個人ライセンス」で、"Any, even non-simultaneous, use by multiple people of a personally licensed copy of the Software is strictly prohibited")。
  - "The User may personally access its copy of the Software through a network, provided that the User has obtained a separate personal license."(ネットワーク越しに**自分の**コピーへ個人でアクセスすることは、個人ライセンスがある限り可)。
  - **ブラウザ内実行・仮想マシン・エミュレーションでの認証が許されるか許されないかを直接述べた文言は、読んだ範囲に見つからなかった**("virtual" は "virtual instrument"/"virtual cash" の意味でのみ出現)。**確認できなかった**。
  - **本プロジェクト全体に関わる制限として、EULA の "Restrictions on use" に "The User shall not … (i) duplicate, rent, lease, alter, modify, translate, **reverse engineer, decompile, disassemble** or create derivative works from the Products; (ii) make access to the Products available to others in connection with a service bureau, application service provider, or similar business" がある。** x86-64 → C → wasm の lift がこれに当たるかどうかの判断は私の範囲外で、事実として PM に報告する(法的な結論は書かない)。(ii) は、依頼者のみがアクセスする私的なデプロイと、他者に提供する公開デプロイとでは意味が違う、と**読む**。
  - 鍵・コードの扱い: "The User may not resell, lend, gift, transfer, or otherwise provide any Image Line Software license, registration key, unlock key, unlock code, or Image Line Account …"; "The User is responsible for maintaining the confidentiality of his account information and registration key … In case the User's registration key is found to be available or accessible to any third party on any website, ftp site or otherwise, the User's registration key will be blocked and rendered unusable without any reimbursement"。
    → **`FLRegkey.Reg` や検証ファイルを、公開のリポジトリ・公開の R2 バケット・ログ・ビーコンに置いてはならない**(依頼者のライセンスが失効しうる)。この設計の全案はこの文言を前提にする。
- 「アカウントの所有者確認のため政府発行の身分証明書を求めうる」(同 EULA、"Account Security and Ownership Verification")。サーバーが非公式のクライアントを拒む方針かどうかの記載は**見つけられなかった**。

## 2. エンジンの認証の実装(静的な読み。実行していない。`binaries/win64/FLEngine_x64.dll`、22,227,016 B、sha256 b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d)

方法: `strings`(ASCII・UTF-16、ファイルオフセット付き。`logs/strings-*.txt`)、PE の import 表(pefile)、`docs/investigations/goal-ladder/fle_index.py` の索引(関数 57,816・thunk 889・命令 2,580,490。`idx/fle.pkl`)、`g8-project-load/fle_query.py`(呼び元)、`fle_disasm.py`(capstone)、自作の `tools/strrefs.py`(関数が参照する文字列)・`tools/strusers.py`(文字列を参照する関数)。エッジは直接 call/jmp のみで、**仮想呼び出し(Delphi の仮想メソッド)・窓手続き・コールバックは追えない(下限)**。VA は preferred base 0x400000 のもの。

### 2.1 ネットワークのスタック — 結論: WinINet だけ。TLS・証明書検証・プロキシ・Cookie 保存はすべて wininet.dll の内側

| 層 | 観測(静的) |
|---|---|
| import | **wininet.dll 15 件**(`InternetOpenW`・`InternetConnectW`・`HttpOpenRequestW`・`HttpAddRequestHeadersW`・`HttpSendRequestExW`・`InternetWriteFile`・`HttpEndRequestW`・`HttpQueryInfoA/W`・`InternetReadFile`・`InternetOpenUrlW`・`InternetSetOptionW`・`InternetGetLastResponseInfoW`・`InternetGetConnectedState`・`InternetCloseHandle`)。**winhttp・ws2_32・wsock32・secur32・bcrypt・ncrypt・urlmon・iphlpapi の import は 0**(`tables/net-imports.tsv`)。FLEngine の IAT で TLS を行う DLL は無い |
| TLS | エンジンは https の URL を WinINet に渡すだけ(`HttpOpenRequestW` の dwFlags に `INTERNET_FLAG_SECURE` 0x00800000 を https のときだけ足す: f_64d520 の 0x64d690)。**TLS は wininet.dll(Schannel)の中**。エンジン自身は TLS の鍵・暗号を持たない |
| 証明書検証 | crypt32 の `CertFindCertificateInStore`・`CryptQueryObject`・`CertGetNameStringW`・`CryptMsgGetParam` と wintrust の `WinVerifyTrust` の**呼び元は f_1375990 の 1 件だけ**(起動の "Init module"・署名者 "Image Line NV" の文字列と同じ関数)。**自分の PE の Authenticode 署名の検査であり、サーバーの TLS 証明書の検証ではない**。サーバー証明書は WinINet が OS の信頼ストアで検証する |
| プロキシ | `InternetOpenW("FL Studio", dwAccessType=0 (PRECONFIG), NULL, NULL, 0)`(f_64d520 の 0x64d813〜0x64d82a。数値は wininet.h の値で Learn のページには無い)。**OS の設定に従う**。エンジンは独自のプロキシ設定 UI を持たない |
| 同期/非同期 | dwFlags = 0 → **同期**。`HttpSendRequestExW(req, NULL, NULL, 0, 0)` の後に本文を `InternetWriteFile`(1 回 0x400 バイトずつ)で書き、`HttpEndRequestW` で終える。応答は `HttpQueryInfoA`(0x16 = HTTP_QUERY_RAW_HEADERS_CRLF〔応答ヘッダー全体を CRLF 区切りの文字列で。1 回目はバッファの大きさの問い合わせ、2 回目で取得〕、0x20000013 = STATUS_CODE の数値、0x20000005 = CONTENT_LENGTH の数値。数値は wininet.h の値で Learn のページには無い)と `InternetReadFile`(0x400 バイトずつ) |
| 要求ヘッダー | エンジンが**自分で**組む(f_64d520 の文字列): `Host:`・`User-Agent:`("FL Studio")・`Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8`・`Accept-Language: en-us,en;q=0.5`・`Accept-Charset: ISO-8859-1,utf-8;q=0.7,*;q=0.7`・`Keep-Alive: 300`・`Connection: keep-alive`・`Content-Length:`。`HttpAddRequestHeadersW` の dwModifiers = 0x20000000(HTTP_ADDREQ_FLAG_ADD)。**MDN の「禁止された要求ヘッダー」(§4.8)に当たるのは Host・Accept-Charset・Keep-Alive・Connection・Content-Length の 5 つ** |
| Cookie | `HttpOpenRequestW` の dwFlags = 0x04400000(NO_CACHE_WRITE \| KEEP_CONNECTION、https なら +SECURE)。**NO_COOKIES を付けない = WinINet の Cookie 保存に任せる**。応答ヘッダー全体を `HttpQueryInfoA(0x16)` で**エンジンも読む**(f_64d520 の 0x64dbf5・0x64dc5d。何に使うかは読んでいない。Cookie は WinINet が自動で保存・送信するので、エンジンが Set-Cookie を自分で処理しているとは読めない)。リダイレクトは自動追従(NO_AUTO_REDIRECT を付けない)。ニュースの取得 f_64bfa0 は HTTP ステータス "200"・"302" の文字列を見る |
| 呼び出し点 | 汎用の HTTP 部品が f_64d520(POST/GET)・f_64bc40(単純な GET)・f_64bfa0(`ILNewsDownload`、`InternetOpenUrlW`)。f_64d520 の直接の呼び元 21 関数(`tables/wininet-functions.tsv`)は、FL Cloud(`credit.*`・`search.*`・`auth.*`・`sounds.*`)、Image-Line サポート(`fl-auth`・`fl-cloud-state`)、ストリーマー、SoundCloud・YouTube(OAuth)、分析(`events.support.image-line.com`)、ライセンス |

### 2.2 宛先ホスト(エンジンの文字列。実際に送られた物ではない。`tables/hosts-urls.tsv` に 82 行)

- **認証(アクティベーション)に関わる物は `support.image-line.com` の次の 7 本**:
  1. `https://support.image-line.com/member/regfile_unlock.php?username=%s&password=%s&checksum=%s&flags=%d&id=%s&v=%s&dn=%s`(f_bf94e0。**ユーザー名とパスワードがクエリ文字列に入る**)
  2. `https://support.image-line.com/action/redirect/FLStudioAuthentication?username=%s&password=%s&flags=%d&id=%s&v=%s&dn=%s`(同じ関数。**同じくクエリ文字列に資格情報**)。2FA のときは `&code_2fa=` を足す(0xbfa754)。f_bf94e0 は `flags & 2` で 1 と 2 を選ぶ
  3. `https://support.image-line.com/member/regcode.php?inapp=1`(f_bfcf20。アプリ内の購入後に登録コードを取る)
  4. `https://support.image-line.com/action/api/fl-auth`(f_bfc590。`Authorization:Bearer <token>`、GET)
  5. `https://support.image-line.com/action/api/fl-cloud-state`(f_c00810、POST、JSON)
  6. `https://support.image-line.com/action/license/serial?serial=%s`(f_c01510。20 桁のシリアルの登録)
  7. `https://support.image-line.com/member/licensebeforeshop.php?ord=%s&u=%s`・`.../jshop/?ord=%s`・`.../shop_inapp.php?ord=%s&inapp=1&v=%d`・`.../finished.php?o=%s&inapp=1`(アプリ内ショップ。**WebView2 の中のページ**)
- 機械識別(認証の要求に入る `id=`・`dn=`・`?machineid=%s&v=%s`、`flsdata`): f_bf94e0・f_bfa770・f_c013e0・f_f00d10・f_10c2010・f_10d1ab0 が、`SHFolder.dll!SHGetFolderPathW(CSIDL 0x1a = APPDATA)`、`GetComputerNameExW(NetBIOS)`、**`HKLM\SOFTWARE\Microsoft\Cryptography` の `MachineGuid`** を読む。
- 認証以外: FL Cloud は `cloud.image-line.com`(f_c01d90 が `%s` に入れる。値は FLCloud の設定 `Software\Image-Line\Shared\General`)、`streamer.image-line.com`、`gopher-fls.image-line.com`(サポートのチャット)、`events.support.image-line.com`(分析)、`support.image-line.com/redirect/…`(News・Versions の取得ほか)、`api.soundcloud.com`・`secure.soundcloud.com`・`www.youtube.com`・`www.image-line.com/flstudio/callback.html`(SoundCloud・YouTube の接続)。
- `https://www.image-line.com/fl-studio/buy-now/`・`support.image-line.com/redirect/me` などは `ShellExecuteW`(既定のブラウザで開く)の引数(f_8ea830 の汎用ラッパー経由)。

### 2.3 UI と判定の置き場所(UTF-16 の文字列と参照関数から。名前は私が付けた「読み」)

| VA | 読み | 根拠 |
|---|---|---|
| f_efe540 | 認証フォーム(`forms.unlockform:wpform`)の初期化 | 文字列 `forms.unlockform:wpform`・"Buy %s..." |
| f_f02e60 | 認証フォームの本体(Sign in and unlock / Unlock の 2 つのボタン、"Your web browser has been opened. Continue the unlock process there."、"Retrieving..."、"Downloading and installing your license.") | 文字列 |
| f_efcf70 | 認証の失敗の表示("Failed to unlock the program."、"Failed to download the license. Please check your internet connection and try again or use the \|Unlock with file\| method.")。**ネットワークの失敗時にエンジン自身が『ファイルで認証』を案内する** | 文字列 |
| f_c044c0 / f_c22a50 | アカウントでの認証の検証("You must enter your e-mail address"・"…password"・"Two Factor Authentication is enabled for this user."・"Invalid code."・"Authentication failed")。**メールが空ならネットワークに出る前に止まる** | 文字列 |
| f_f006d0 | **検証ファイルの保存**(`licensevalidation.txt`、"The validation file has been saved."、"File saved"/"File saving failed") | 文字列 |
| f_eff950 | **ライセンスファイル(FLRegkey.Reg)の読み込み**: 失敗の表示 "The registration code is incorrect. Please download it again from your user account."・"Installation of the registration code failed."・"Your registration doesn't include a license for this version of %s."・"The registration code is outdated."・"The license file couldn't be opened." と、`[HKEY_CURRENT_USER\Software\Image-Line\Registrations\FL Studio` | 文字列 |
| f_10d1ab0 | アプリ内の登録コード(`regcode.php?inapp=1`)の処理の結果: "Success\|The registration succeeded."、"Please restart %s for the registration to take effect."、"The application will now close." | 文字列 |
| f_f02630 | 全ユーザーへのコピー(`CopyLicenseToAllUsers.exe -load "%s"`、管理者として実行) | 文字列。**ブラウザでは使わない** |
| f_11390c0 | ウィザード(Welcome)の "Unlock" ボタン("Waiting for image-line.com"、"Downloading and installing your license"、"Unlock failed") | 文字列 |
| f_f067d0 | `licence-engine\LicenceEngine.dll` を LoadLibraryW(Rent-To-Own の管理。インストーラの同梱 DLL 一覧〔`bundled-dll-inventory.md`〕に無い) | 文字列。**未確認: 永久ライセンスの依頼者には関係しないと読む** |

- **エンジン自身が `.reg` の文章を解析する**: 文字列 "windows registry editor version 5.00"・"regedit4"・"hkey_current_user\\"・"dword"(0x7f78dc〜0x7f7994 のオフセット = VA 0xbf84dc 付近の関数群 f_bf80d0 ほか)。つまり `FLRegkey.Reg` は **Windows の regedit に渡されるのではなく、エンジン(Delphi のコード)が読んで** `RegCreateKeyExW`・`RegSetValueExW`・`RegFlushKey` で `HKCU\Software\Image-Line\Registrations\FL Studio 26.1`(値名 `ILRegKey`)に書く、と読む(文字列 `FL Studio 26.1`・`Software\Image-Line\Registrations\FL Studio 26.1`・`ILRegKey` は f_f00d10・f_10c2010・f_10d1ab0)。
- **判定の置き場所**: 認証済みかどうかの判定は、レジストリの `ILRegKey` と機械識別(APPDATA・コンピューター名・MachineGuid)を**エンジンのコードが**照合する(f_10c2010。advapi32 の Crypt* import は無く、判定はエンジン内部の計算。CryptoAPI 経由ではない)。**判定の中身は解析していない(偽装・迂回の対象にしないため、読む必要も無い)**。
- 現在の runtime で機械識別がどう見えるか(`lift/runtime`・`lift/harness/seeds`): **コンピューター名は定数 `DESKTOP-0000001`**(`fle_env.h`)、APPDATA は仮想ボリュームの `C:\Users\user\AppData\Roaming` 相当、**`HKLM\SOFTWARE\Microsoft\Cryptography` はレジストリの種(`reg-seed-nsis.tsv`・`reg-seed-app.tsv`)に無く、`RegOpenKeyExA` は `ERROR_FILE_NOT_FOUND`**(`fle_reg.c`: 「HKEY_LOCAL_MACHINE は Software だけ」)。**レジストリはメモリ上のみで、ページを閉じると消える**。

### 2.4 WebView2(エンジンが埋め込みの Web ページに使う)

エンジンは `WebView2Loader.dll` を LoadLibraryW し(`CreateCoreWebView2EnvironmentWithOptions`)、窓クラス `TEdgeBrowser` で Web ページを表示する(Browser パネル、『Buy FL Studio』= `TInAppShopForm`、`forms.unlockform:wpform`・`forms.welcomeunlock:wpform`・`forms.cloudaccountsform:wpform`)。JS 側への橋は `window.flHelper.onUnlockRequest(...)`・`onMenuCallBack(...)`(f_dabe80)。**現在の runtime は WebView2Loader.dll を『無い』(LoadLibraryW が 126)とし(決定 0126 の S-webview2-absent)、窓は灰色/黒のまま**(決定 0142・0191 の観測と一致)。ページの CSP は `frame-ancestors 'none'`・COEP `require-corp`・COOP `same-origin`(`app/shared/headers.ts`)で、**別オリジンの Image-Line のページを iframe に入れることは、COEP のため相手が CORP/COEP ヘッダーを返さない限りできない**(読み。相手のヘッダーは取得していない)。したがって埋め込み Web ページの代替は iframe ではなく**新しいトップレベルのタブ**になる。

## 3. 観測(実エンジンの native -O0 fork プローブ。ブラウザ・実機 Windows では 0 件)

**方法**: 決定 0171 の fork プローブ(1 操作ごとに最初の idle の同じ状態〔空のプロジェクト、ウィザードを Enter で閉じた後。`snap e03`〕から子を fork し、終了コード・窓表・描画面を集める)。
- 構築 **A1** = FLEngine_x64.dll と同梱 5 DLL(dsp_ippv2・Blend2D・quickfontcache・freetype・ILTools)を x86-64 → C → native -O0 にした物。層 1〜4(決定 0171・0178・0191 の行 = `ia2-imports.txt`)に**層 5**(`wininet.dll` の 10 行)を足した。実行した関数の種類 5,398・入口の総数 1,052,558 は ia2 の D3〜D8 と**同じ**(`logs/gen-A1.txt`)。起動の 2 つの snapshot(`wiz0`・`e03`)の `g3-snap`・`g3-progress`・`g3-surface`・`g3-wintable` の 11 行は、ia2 の最終構築 D8 の同じ走行と**完全に同じ**(`logs/startup-vs-ia2-D8.txt`。16 snapshot の 82 行比較は回していない)。
- 構築 **A2** = A1 に、決定 0188 の `exp1-imports.txt`(`IFileSaveDialog` の 5 行と `SetThreadExecutionState`)を足した物。理由: A1 で "Save validation file…" を押すと、エンジンは `CoCreateInstance(CLSID_FileSaveDialog)` を呼んだ後に何もせず(Show なし。ファイルも書かれない)、`ia2-imports.txt` に exp1 の行が無い(HEAD 74f2d8f が『ia2-imports.txt lacked six exp1 rows』と記録した欠落と同じ)ためと読んだ。**現在の出荷の行 `ship2-imports.txt` には exp1 の 6 行が入っている**(`IFileSaveDialog` を含む。HEAD のファイルで 7 件ヒット)ので、出荷の構成では A1 の症状は起きない。
- 走行: A1 で 17 本のプローブ(`runs/a1`・`runs/follow`)、A2 で保存・読み込み・識別・時刻・償還の走行(`runs/f-base`・`f-guid`・`g-late`・`h-neg`・`i-redeem`)。一覧は `tables/probes.tsv`。

**模擬の範囲**: (a) 座標は窓の PNG の読み(±3 px)。(b) ファイル選択ダイアログの答えは**ハーネスが環境変数で与える**(`FLE_FILEDLG_SAVE`・`FLE_FILEDLG_PICK`。依頼者の入力の代わり。エンジンの出力ではない。stderr に `THE SELECTION WAS GIVEN BY THE HARNESS` が出る)。(c) ネットワークは**常に『届かない』**(層 5 の答え)。(d) **『I agree to the Image-Line Terms of Use』のチェックは入れていない**(利用規約への同意は依頼者本人の行為なので、ハーネスでも押さない。このため "Sign in and unlock" がブラウザを開く URL は観測していない〔静的な読みだけ。§2.2〕)。(e) 資格情報は一切入力していない(メール・パスワードの欄は空のまま)。(f) §3.3 の機械識別のテストに使った MachineGuid は**私が作ったテスト用の値**(`seeds/reg-seed-testguid.tsv`。ライセンスでも実在の機械の識別でもない)。**代役・画像の再生・事前生成は無い**。Chrome・Firefox では測っていない。

### 3.1 起動(最初の idle まで)に出るネットワーク要求

| 要求 | 内容(値は伏せてある) | エンジンへの答え |
|---|---|---|
| `InternetOpenUrlW` ×2 | `https://support.image-line.com/redirect/downloadnewsrss?user=<0>&v=<11>&flags=<3>&flsdata=<0>&id=<74>` と `…/redirect/downloadversionsxml?…` 同形(ニュースとバージョン表。スレッド `ILNewsDownload`/`ILVersionsDownload`) | 決定 0132 のとおり NULL + 12007(変えていない) |

読み: `user` と `flsdata` は空(未認証)、`v` は 11 文字(版の文字列)、`flags` は 3、**`id` は 74 文字 = 機械識別(§4.2)が認証の無い要求にも毎回付く**。起動の間に WinINet の `InternetConnectW`・`HttpOpenRequestW`・`HttpSendRequestExW` の呼び出しは**無かった**(層 5 の記録は 0 件)。

### 3.2 認証に関わる操作(A1。表: `tables/probes.tsv`、要求の記録: `tables/net-calls.tsv`、画像: `png/`)

| 操作 | エンジンの反応(`g3-win`) | 外向きの要求 |
|---|---|---|
| (基準)何もしない `noop` | — | 0 |
| **HELP → Unlock FL Studio** | **`TUnlockForm`** 題 `FL Studio is in trial mode.`、540x283、画面 (690, 398)。タブ "Unlock on web"(既定: 『I agree to the Image-Line Terms of Use』のチェック、**"Sign in and unlock"**、"Refund policy")と "Unlock with account"(E-mail address・Password〔目のアイコン〕・**"Unlock"**)。右の "More options": **"Unlock with file…"**・"Redeem code…"・"Buy FL Studio…"。下に "Don't unlock"(`png/a1_help_unlock-lid139.png`、`b_unlock_account_tab-lid139.png`) | **0** |
| → "Unlock with file…" | 同じ `TUnlockForm` が **"Unlock with file" パネル**に変わる: 左 "You will need this file to download a license." + **[Save validation file…]**、右 "Install license file to unlock FL Studio." + **[Load license file…]**、下 [Go back] [More information…](`png/b_unlock_file-lid139.png`) | 0 |
| → [More information…] | (窓の変化なし) | `ShellExecuteW(NULL, "https://support.image-line.com/redirect/HowToUnlock")` → SE_ERR_NOASSOC(何も起動しない)。**静的な読みの文字列 `HowToUnlock` と一致** |
| → "Sign in and unlock"(同意のチェックなし) | `TMsgForm` 題 `Error`(445x182)。**本文の領域は 1 色(86,80,71)だけ = 本文が描かれていない**(アイコンと Ok だけ。`png/b_signin_noagree-lid146.png`)。エンジンの文字列からは "You must agree to "Image-Line Terms of Use" to proceed." のはず(読み) | 0(**ネットワークに出る前にエンジン自身が止めた**) |
| → "Unlock with account" タブで "Unlock"(同意なし、欄は空) | 同じ `TMsgForm` 『Error』(本文は空) | 0 |
| → "Redeem code…" | `TRedeemCodeForm` 題 `Redeem code`(395x173): 入力欄 "Enter your 20 digit code" + [Redeem]。**明らかに無効な 20 桁(0 が 20 個。私が作った値)を入力して [Redeem] を押す**(`runs/j-redeem`): | `ShellExecuteW(NULL, "https://support.image-line.com/action/license/serial?serial=00000000000000000000")` → SE_ERR_NOASSOC(何も起動しない)。**WinINet の呼び出しは 0**。= 『Redeem』はアプリ内で送信せず、既定のブラウザで公式のページを開く(静的な読み `license/serial?serial=%s`〔f_c01510〕と一致)。注意: ia2 の `ShellExecuteW` の記録は URL を全文 stderr に出す(本物の登録コードを入れた走行では漏れる。act1 は本物を入れていない) |
| → "Buy FL Studio…" | `TInAppShopForm` 『Buy FL Studio』(灰色一面。WebView2 が無い) | 0 |
| HELP → About | `TMIDIForm` 題 `Settings - About FL Studio`、668x900。**`Trial v26.1.7 [build 5653]`、`FL STUDIO 2026`、`Windows - 64Bit`、`FL Studio is in trial mode.`、ボタン "Unlock FL Studio"**(`png/a1_help_about-lid22.png`)。= 認証前の状態の基準 | 0 |
| HELP → Buy | `TInAppShopForm` 『Buy FL Studio』(灰色一面) | 0 |
| HELP → My account | (窓なし) | `ShellExecuteW(NULL, "https://support.image-line.com/redirect/me")` → SE_ERR_NOASSOC。**静的な読みの URL と一致** |
| OPTIONS → Manage accounts | `TCloudAccountsForm` 題 `Manage cloud accounts` | 0 |
| FILE → Save(空のプロジェクト → Save as ダイアログの Save) | `TInAppShopForm` 『Buy FL Studio』が出て、題が `Project_1.flp - FL Studio 2026` に変わる(決定 0191 の観測と一致。エンジン自身の試用ダイアログで、**偽装していない**) | 0 |

**観測のまとめ(A1)**: 認証のための窓(Unlock・About・Redeem・Buy・Manage accounts)は**ネットワークに一度も出ずに**開く。アカウントでの認証は、同意のチェック(エンジンが先に確かめる)を通らない限り、**WinINet に到達しない**。外へ出る要求は起動時のニュースとバージョン表(2 本、`id` 付き)だけ。ShellExecuteW は 2 か所(`redirect/me`・`redirect/HowToUnlock`)で観測し、いずれも何も起動しない(ia2 の既定)。

**発見(失敗時の表示に関わる)**: エンジンの `TMsgForm`(『Error』など)の**本文が描かれない**。`b_signin_noagree` と `c_account_empty` の 2 回とも、本文領域の全画素が 1 色だった(`runs/follow/dump/b_signin_noagree-lid146-445x182.bgrx` の x 95〜437、y 32〜117 を数えた)。決定 0129 の『文字が無い』とは別で、主窓・Unlock 窓の文字は出ている。**原因は調べていない**(未確認。読み: 本文が WebView2 の領域、またはこの窓だけが使う文字の経路が出していない)。→ **このままでは、認証の失敗の文言(§4.6)を依頼者は読めない**。act1 の範囲外だが、案 A の『失敗時の表示』の前提なので PM の判断点に挙げる。

### 3.3 オフライン経路(A2: Save validation file… / Load license file…)

A1 では "Save validation file…" を押しても何も起きなかった(`d_save_validation`: エンジンは `CoCreateInstance(CLSID_FileSaveDialog)` を呼んだ後、`SetFileName` も `Show` も呼ばず、ファイルも書かれない。`runs/follow/probes/d_save_validation.err`)。`ia2-imports.txt` に決定 0188(exp1)の `IFileSaveDialog` の行が無いためと読み、行を足した A2 で再テストした(ハーネスが `FLE_FILEDLG_SAVE`・`FLE_FILEDLG_PICK` で保存先・選択を与える)。

**(a) Save validation file…(`runs/f-base/probes/f_save_validation.err`、実測)**: エンジンは `IFileSaveDialog` を作り、次を設定して `Show` する。

| 項目 | 値 |
|---|---|
| 提案名 `SetFileName` | `licensevalidation.txt` |
| 初期フォルダ `SetFolder` | `C:\Users\user\Documents`(`SHGetKnownFolderPath(FOLDERID_Documents)`) |
| フィルター `SetFileTypes[0]` | 表示名 `License validation files (*.txt)`、仕様 `*.txt`(`SetFileTypeIndex 1`) |
| `SetOptions` | `0x20000802` = FOS_DEFAULTNOMINIMODE \| FOS_PATHMUSTEXIST \| FOS_OVERWRITEPROMPT(0x2・0x800・0x1000 は `fle_filedlg.c` の定義。0x20000000 = FOS_DEFAULTNOMINIMODE は shobjidl.h の値の記憶で UNOFFICIAL) |
| 題 `SetTitle` | (設定なし) |

ハーネスが `C:\Users\user\Documents\lv.txt` を答えると、エンジンはそこに **521 バイト**の検証ファイルを書き、`TMsgForm` 題 `File saved` を出す(本文は §3.2 の問題で空)。この窓は**モーダル**で、Enter(`tap 13`)で閉じるまで主窓とフォームを塞ぐ(後続のクリックは効かない)。

**(b) 検証ファイルと機械識別(`logs/validation-compare.txt`。中身は解析していない)**: 521 バイト、**100% が印字可能な ASCII、1 行**(テキスト化された不透明な塊と読む)。仮想マシンの識別の材料(コンピューター名 `DESKTOP-0000001`、APPDATA のパス、ユーザー名)は**平文で入っていない**(ASCII でも UTF-16LE でも検索して 0 件)。識別を変えると変わる: MachineGuid の無い走行(`f-base`)と、**私が作ったテスト用の MachineGuid**(`00000000-0000-4000-8000-0000000a0001`)を種に入れた走行(`f-guid`)で、同じ 521 バイトのうち **479 バイトが違う**(sha256 `e851287a…` 対 `95391e25…`)。起動の `wiz0` の入口数も違う(56,506,070 対 56,507,735。MachineGuid を読んで計算する経路が走った)。**MachineGuid が無くてもエンジンは検証ファイルを作る**(欠落で失敗しない)。**9 仮想秒遅らせて保存し直しても同じバイト列**(sha256 `e851287a…` が一致。`runs/g-late`、`logs/validation-compare.txt`)= 仮想時計には依存しない。ただし、この環境の時計と乱数は決定的なので、乱数の混入の有無は区別できない(**未確認**)。

**(c) Load license file…(`runs/h-neg/probes/h_load_back.err`、`runs/follow/probes/c_file_dialogs.err`、実測)**: エンジンは `IFileOpenDialog` を作り、次を設定して `Show` する。

| 項目 | 値 |
|---|---|
| 題 `SetTitle` | `Browse for registration file...` |
| フィルター | 表示名 `RegKey file (*.reg)`、仕様 `*.reg`(`SetFileTypeIndex 1`) |
| 既定の拡張子 | `reg` |
| 初期フォルダ | `C:\Users\user\Downloads`(`SHCreateItemFromParsingName`) |
| 提案名 | `untitled` |
| `SetOptions` | `0x20001000` = FOS_DEFAULTNOMINIMODE \| FOS_FILEMUSTEXIST |

→ 公式の手順で依頼者が受け取る `FLRegkey.Reg` は `*.reg` に当たる。**橋(opfs1 の段階 2)はこのフィルターと FOS_FILEMUSTEXIST を扱う必要がある**(§4.4 の R2)。

**(d) 否定のテスト: 検証ファイルを『ライセンスファイル』として読ませる(`runs/h-neg`)**: 「File saved」(モーダル)を Enter で閉じた後、"Load license file…" の答えとして、**エンジン自身が書いた検証ファイル**(明らかに登録コードではない物)をハーネスが与えた。エンジンは `CreateFileW` 1・`SetFilePointer` 4・`ReadFile` 1・`CloseHandle` 1 でファイルを読み(importlog、`runs/h-neg/probes/h_load_back.err`)、**`Reg*` の呼び出しは 0 回(= レジストリに何も書かない)**、ネットワークの呼び出しも 0、停止もしない(終了コード 0)。その後 `TMsgForm` 題 `Error`(385x226)が出る。ボタンは **"Ok" と "Go.."**(`png/h_after_load-lid156.png`)。"Go.." は "Go to account" の切れた表示と**読む**(エンジンの文字列 "Error|The registration code is incorrect." + "Please download it again from your user account." + "Go to account" が同じ関数 f_eff950 にある)。本文は §3.2 の問題で描かれない。→ **エンジンの拒否は完全にエンジン内(ローカル)で行われ、拒否のときは何も書かれない**。実鍵での成功の経路(レジストリへの書き込み)は、このテストでは見えない(**未確認**)。偽造ではない: 渡した物はエンジンの出力で、登録コードの形をしていない。

**(e) 取り消し(`h_load_cancel`)**: "Load license file…" のダイアログでハーネスが何も選ばない(`Show: cancelled`)と、`TUnlockForm` はそのまま(メッセージなし、停止なし)。橋(opfs1)が『取り消し』を返せば、エンジンが自分で処理する。

**(f) 行の欠落の影響**: A1(exp1 の行なし)では "Save validation file…" は何も起こさなかった(§3.3 冒頭)。出荷の行 `ship2-imports.txt` には exp1 の 6 行が入っているので、出荷の構成では起きないはずだが、**出荷バンドル(Chrome・Firefox)では確かめていない**。

## 4. 設計(実装はしていない。作業量は私の見積りで『推測』。実機・実サーバー・実鍵での確認は 0 件)

PM の判断(2026-11-04、中間報告 1 への返信): **案 A(オフライン認証 = エンジンの『Unlock with file』をそのまま使う)を採用。案 B は採らない。案 C は観測の記録だけ。** 以下はその前提で書く。

### 4.0 前提となる事実(§1〜§3)

1. 公式のオフライン認証がある(20.9 以降は検証ファイル → `FLRegkey.Reg` の 2 段)。**エンジン自身がその UI を持つ**(`TUnlockForm` の "Unlock with file…" → "Save validation file…" / "Load license file…"。§3 の観測)。エンジンは検証ファイルを書き、`FLRegkey.Reg` を読み、**エンジンのコードが**検証して `HKCU\Software\Image-Line\Registrations\FL Studio 26.1` に書く。依頼者のパスワード・トークンは、この経路のどこにも現れない。
2. 現在の runtime は、レジストリがメモリ上のみ、`MachineGuid` が無い、コンピューター名が定数(§2.3)。
3. 公式の手順は **「Restart FL Studio」で完了**する(ブラウザ版ではページの再読み込み)。
4. **偽装の禁止(再確認)**: 私は `FLRegkey.Reg` の中身を作らず、解析して生成もしない。エンジンが依頼者の本物のファイルを読んで判定する。設計のどこにも『エンジンの判定に代わる物』は無い。

### 4.1 案の比較と採否

| 案 | 内容 | 資格情報の経路 | 採否と理由 |
|---|---|---|---|
| **A 採用** オフライン認証(ファイル) | エンジンの Unlock with file をそのまま使う。ファイルの受け渡しは opfs1(決定 0216)のエクスプローラーを通す。認証結果(レジストリ)と機械識別を安定させる | **なし**(依頼者が自分のブラウザのタブで Image-Line にログインする) | 資格情報が本プロジェクトのコード・中継・ログ・ビーコンを通らない。公式に存在する手順で、エンジンは失敗時に自分で案内する(f_efcf70)。実装が小さく、ネットワークの橋に依存しない |
| **B 不採用** アカウント認証(WinINet → fetch 橋 + 同一オリジン中継) | エンジンの Unlock with account | メールとパスワードを **GET の URL のクエリ**に入れる仕様(`…/regfile_unlock.php?username=%s&password=%s&…`、`…/FLStudioAuthentication?username=%s&password=%s&…`。§2.2)。中継(vite middleware / Cloudflare Worker)とページのメモリを通る | **採らない**: 資格情報が中継・実行基盤・各種ログに乗りうる。パスワードは漏れると取り返しがつかない(鍵は再発行できる)。禁止ヘッダー(Host・Connection・Keep-Alive・Content-Length・Accept-Charset を エンジンが自分で付ける)、Cookie(WinINet の保存に任せている = 橋が保存を持つ必要)、リダイレクト、CORS 不明のため、橋は約 3 週間(推測)の規模になる。2FA の確認コードも窓に入れる必要。詳細は 4.8 に記録 |
| **C 記録のみ** ブラウザの別タブで Sign in and unlock | 既定のブラウザで `https://support.image-line.com/redirect/WebsiteUnlock?machineid=<id>&v=<版>` を開く(読み。§2.2・§3)。ログインは Web 側 | なし(別タブ) | 結果の受け取りはエンジンが Web 側へ**問い合わせて**行うと読む(WinINet が要る = 案 B と同じ橋)。成立の条件が不明なので設計しない。観測は §3 |
| D 不成立 アプリ内ストア / Web ページの埋め込み | WebView2 の代わりに iframe | — | Image-Line のページ自身が `Content-Security-Policy: frame-ancestors 'self' https://www.image-line.com`(login)/`https://*.image-line.com`(www)を返す(2026-11-04、公開ページの GET のヘッダー。`sources/headers-public-pages.txt`)ので、他オリジンの iframe に入らない。さらにページの COEP `require-corp` もある |

### 4.2 機械識別の安定性(PM 要件 ①)

**FLRegkey.Reg は『検証ファイルを生成した機械でしか動かない』**(Image-Line 原文、§1.2)。検証ファイルに入る機械識別が何から作られるかを、エンジンのコードから読んだ(静的。`tables/machine-identity.tsv`、証拠は逆アセンブル `logs/dis-bf94e0.txt` と呼び元の表)。

**識別の材料(読み。24 関数が同じ並びで繰り返している: f_bf94e0・f_bfade0・f_bfcfc0・f_bfe2b0・f_f00d10・f_f03cb0・f_f2dbb0・f_f40840・f_f51fd0・f_ffbff0・f_103ca70・f_10b4700・f_10c2010・f_10c7cd0・f_10d1ab0・f_117f850・f_11a57c0・f_11bfc10・f_11d9340・f_11dd800・f_11fc770・f_121d7e0・f_125da90・f_12875a0)**:

| # | 材料 | エンジンの取り方 | 現在の runtime の値 | ページを開くたびに変わるか | 決める所 |
|---|---|---|---|---|---|
| 1 | アプリデータのフォルダ | `SHFolder.dll!SHGetFolderPathW(CSIDL_APPDATA = 0x1a)` | `C:\Users\user\AppData\Roaming`(`fle_env.c`、定数) | 変わらない(コンパイル時の定数) | runtime(コード) |
| 2 | コンピューター名(NetBIOS) | `GetComputerNameExW(0)`(25 呼び出し点がすべて NameType 0) | `DESKTOP-0000001`(`fle_env.h` の `FLE_VIRTUAL_COMPUTER_NAME`、定数) | 変わらない | runtime(コード) |
| 3 | **MachineGuid** | `RegOpenKeyExA(HKLM, "SOFTWARE\Microsoft\Cryptography")` + `RegQueryValueExW("MachineGuid")` | **無い**(レジストリの種に無く、`ERROR_FILE_NOT_FOUND`。§2.3) | 変わらない(常に無い) | **レジストリの種** |

読み: 3 つの文字列を連結し、**SHA-256**(f_bf89b0 が初期値 8 語を `0x20daeebb` との XOR で隠して置く。`0x20daeebb ^ 0x4ad308dc = 0x6a09e667` など SHA-256 の初期値と一致する)のブロック(64 バイト、`div 0x40`、長さ x 8 ビット)に通し、16 進文字列にして `id=` に入れる(ニュースの取得 URL の `id=` は 74 文字: 観測 §3.1。**SHA-256 の 16 進 64 桁と 74 文字は合わない。差の 10 文字の中身は読んでいない = 未確認**)。**識別に使われていない物(読みの範囲)**: ボリュームのシリアル(`GetVolumeInformationW` の呼び元 3 関数は上の 24 関数に含まれない)、CPU ID(`cpuid` 命令は f_bf94e0 に無い)、MAC アドレス(iphlpapi の import なし)、ユーザー名(`GetUserNameW` の import なし)、画面・GPU。**この読みは検証ファイルの中身では確かめていない**(中身の解析はしない)。§3.3 の観測は、検証ファイルが **MachineGuid を変えると変わる**(同じ 521 バイトの 479 バイトが違う)こと、コンピューター名・APPDATA・ユーザー名が平文では入っていないこと、時刻で変わらないことだけを示す。

**設計: 材料 3 の MachineGuid を、依頼者のデプロイ固有の定数として与える。**

- **与え方(第一候補): 既存のレジストリの種の機構(決定 0116。`FLE_REG_SEED`)に 1 ファイル足す**。ブラウザのバンドルは、`build-engine-bundle.sh` が `pack_seed.py --extra reg-seed-nsis.tsv --extra reg-seed-app.tsv …` で種を `seedpack.bin` に詰め、マニフェストの `engine.env` に `FLE_REG_SEED=/data/seed/reg-seed-nsis.tsv:/data/seed/reg-seed-app.tsv` を書いている(`app/scripts/build-engine-bundle.sh` の 300・337 行目)。ここへ**配備ごとに作る 1 行のファイル**(`HKLM\SOFTWARE\Microsoft\Cryptography<TAB>MachineGuid<TAB>REG_SZ<TAB><小文字の GUID>`)を `--extra` で詰め、`FLE_REG_SEED` の末尾に足す。**新しい runtime のコードは要らない**: native のプローブで、同じ書式のテスト用ファイル(`seeds/reg-seed-testguid.tsv`)を `FLE_REG_SEED` の末尾に足すだけで、エンジンが MachineGuid を読み、検証ファイルが変わった(§3.3 (b))。書式は Windows の MachineGuid と同じ `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`。値が無ければ今のまま(MachineGuid なし)。コンピューター名(`DESKTOP-0000001`)は**今の定数のままでよい**(変える理由が無く、変えると認証が外れる)。
- **値の生成と置き場**: 依頼者が `uuidgen` で 1 回だけ作り、デプロイの入力(GitHub Actions の Variables、またはマニフェスト生成時の環境変数 `FL_MACHINE_GUID`)として `build-engine-bundle.sh` に渡す。**リポジトリには値を一切コミットしない**(`.gitignore` の対象に生成物を置く)。種ファイルは `seedpack.bin` の中でバンドルの一部として R2 に置かれ、Cloudflare Access の内側にある(秘密ではないが公開もしない)。
- **具体(未実装。`build-engine-bundle.sh` の `stage_seed` と `pack-assets.mjs` の `--env FLE_REG_SEED=…` への追加、約 10 行)**:
  - 種ファイル(1 行、タブ区切り、`reg-seed-machine.tsv`):
    `HKLM\SOFTWARE\Microsoft\Cryptography<TAB>MachineGuid<TAB>REG_SZ<TAB>xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
  - 入力の名前: **`FL_MACHINE_GUID`**(ビルドの環境変数。GitHub Actions なら Variables の同名の変数を env に渡す。`uuidgen | tr A-Z a-z` の値。書式を正規表現 `^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$` で検査し、合わなければビルドを止める)。未設定なら種ファイルを作らず、今のバンドルと同じ(MachineGuid なし)。
  - 詰め方: `pack_seed.py --extra "$WORK/in/reg-seed-machine.tsv"`(ファイルは `/data/seed/reg-seed-machine.tsv` として入る。`pack_seed.py` は `seed/<basename>` で詰める)。`seedpack.bin` は `[ ! -s ]` でキャッシュされるので、**GUID が変わったら作り直す**(キャッシュのキーに GUID の sha256 の先頭を入れる)。`--env FLE_REG_SEED=/data/seed/reg-seed-nsis.tsv:/data/seed/reg-seed-app.tsv:/data/seed/reg-seed-machine.tsv`。
  - 検証: native のプローブで `FLE_REG_SEED` の末尾に同じ形のファイルを足すと検証ファイルが変わる(§3.3 (b))。バンドルでは、同じ GUID で 2 回ビルド → 検証ファイルの sha256 が一致、別の GUID → 不一致、を `Save validation file…` で確かめる(検証ファイルは Download で取り出し、**リポジトリ・ログに載せない**。sha256 だけを記録してよい)。
  - **値はリポジトリにコミットしない**。マニフェスト(R2)にも書かれない(`engine.env` には種ファイルの**パス**だけが入り、値は `seedpack.bin` の中)。
- **OPFS が退避されても識別は変わらない**: 識別の材料 1〜3 はすべて**バンドル(種ファイル)とコードにあり、OPFS にもレジストリの永続にも依存しない**。ブラウザがストレージを消しても、**同じ `FLRegkey.Reg` をもう一度『Load license file…』で読めば再び認証される**(新しい検証ファイルの Upload は要らない)。→ 依頼者は `FLRegkey.Reg` を自分のブラウザの外(自分の PC のフォルダ)に保管しておく。
- **却下した案**: (b) OPFS に初回生成して永続 — OPFS が退避されると別の機械になり、毎回、検証ファイルから出直し(Image-Line が検証ファイルの発行回数を数えるかは記載が見つからず**確認できなかった**)。(c) 定数のまま(MachineGuid なし)— 全デプロイで識別が同じになり、『1 台に縛る』公式の意味が消える(他人の鍵が他人のデプロイで動きうる)。
- **観測済み**: エンジンは MachineGuid が欠落していても検証ファイルを作る(§3.3 (b))。**未確認**: GUID の書式の制約(テスト用の値は Windows の書式だった)、Image-Line がブラウザ内のこの識別を『依頼者が専有して管理する 1 台のコンピューター』と扱うか(EULA の "any computer/s that you exclusively control"。法的判断は私の範囲外。**私的なデプロイで依頼者のみがアクセスすることが設計の前提**)。

### 4.3 認証結果の永続(PM 要件 ②。opfs1・決定 0216 の段階 3 と共有する設計)

- **永続の対象は実鍵の読み込みで確定する(未確認)**: 静的に見えるのは `HKCU\Software\Image-Line\Registrations\FL Studio 26.1` の `ILRegKey` までで、ほかに書く物(文字列 `License\`・`RemoveLicenses.exe`・`CopyLicenseToAllUsers.exe` は『全ユーザーへのコピー』と『ライセンスの削除』の経路で、f_f02630 は管理者として別プロセスを起動する = ブラウザでは使わない)があるかは、依頼者の実鍵での最初の通しテストで、native のプローブに `importlog on` を付けて `CreateFileW`・`WriteFile`・`Reg*` の呼び出しを記録して確かめる。永続の方式(下)は『部分木 + 指定ファイル』を一般に扱えるようにしておく。
- **永続する物**: (1) レジストリの部分木 `HKCU\Software\Image-Line\Registrations`(認証結果。`FL Studio 26.1\ILRegKey` ほか。エンジンが `FLRegkey.Reg` を読んで `RegCreateKeyExW`・`RegSetValueExW`・`RegFlushKey` で書く)。(2) 依頼者が置いた `FLRegkey.Reg` のコピー(OPFS の `/fl/license/`。再認証用。**機密**)。それ以外のレジストリは起動ごとに種から作り直す(今のまま)。
- **方式**: `fle_reg.c` が対象の部分木への書き込みで dirty を立て、`RegFlushKey` と終了で部分木全体(キー名・値名・型・データ。数 KB)を直列化してホストへ渡す。Worker が `FileSystemFileHandle.createSyncAccessHandle()` で OPFS の `/fl/state/registrations.bin` に書く(MDN: Baseline、2023-03 から全ブラウザ、**専用 Worker のみ**〔`sources/mdn-sah.txt`〕。依頼者の Firefox 159 も該当。この機械の Firefox では未確認)。起動時はエンジンの入口の前に読み、`reg_seed.h` と同じ手順(`RegCreateKeyExW`/`RegSetValueExW` を入口の前に呼ぶ)で復元する。**識別(4.2)は OPFS に入れない**。
- **opfs1 の段階 3 との接点**: 段階 3 は『OPFS をゲストの実体に』(`createSyncAccessHandle` は開くのが非同期なので JSPI / ヘルパー Worker + `Atomics.wait` / 先読みの 3 候補。決定 0216 §8)。レジストリの永続は**ゲストのファイルではなくホストの状態**なので、段階 3 を待たずに作れる。ただし同じ OPFS のルート(例 `/fl/state/`)と同じ Worker の書き込み経路を使う。要件は `MERGE-NOTES.md` の「opfs1 へ渡す要件」に書いた。
- **消えたとき**: OPFS が消えるとレジストリの認証結果が消える。識別が変わらない(4.2)ので、`FLRegkey.Reg` をもう一度読めば戻る。`navigator.storage.persist()` を要求する(追い出しの抑止。許可はブラウザ次第)。

**実鍵の通しテストの計画(永続の対象を確定する。依頼者の操作と実鍵が要る。act1 では実施していない)**:
1. 依頼者が `uuidgen` で MachineGuid を 1 つ決める(以後この値を使い続ける。リポジトリに入れない)。
2. その値の 1 行の種ファイルで A2 型の native プローブを起動し(`FLE_REG_SEED` の末尾に足す。§3.3 の `f_save_validation` と同じ手順)、`Save validation file…` で検証ファイルを作る(**同じ識別で作ること**。識別が違う検証ファイルから得た鍵は通らない)。依頼者が自分のブラウザで Image-Line にアップロードし、`FLRegkey.Reg` を受け取る。
3. 鍵ファイルは**権限 0600 の非 git の私的なディレクトリ**(例: `lift/out-<名前>/private/`)に置き、報告・ログ・ビーコン・コミットに載せない。使い終わったら消す(EULA: 鍵の露出は失効の理由になる)。依頼者の PC のブラウザ版で試せるなら、鍵は依頼者の PC から出さない方がよい(その場合、手順 4 のログはブラウザのエンジンのログで代える)。
4. 同じ識別で起動し、`Load license file…`(ハーネスが `FLE_FILEDLG_PICK` で鍵を指定)の前後で `importlog on/off` を付け、**`CreateFileW`・`WriteFile`・`SetFilePointer`・`Reg*` の呼び出しと引数(キー名・値名・データの大きさ)**を記録する。→ 永続する物の一覧(レジストリのキーと値、ファイル)を確定する。**必要な小さな道具**: `HKCU\Software\Image-Line\Registrations` の部分木を reg-seed 形式(`HKCU\...<TAB>名前<TAB>型<TAB>データ`)の TSV に書き出す関数(`fle_reg.c` が木を持つので約 40 行)を終了時に呼ぶ。
5. 手順 4 の TSV を `FLE_REG_SEED` の末尾に足して**起動し直し**(= 『Restart FL Studio』の native 版)、**認証後の観測**(§4.5 の (a)(b)(c)): `HELP → Unlock FL Studio` のフォームの題、`File → Save` で『Buy FL Studio』が出ないこと、`HELP → About` の名前とライセンス。窓表の差で判定する。
6. 結果が『受け入れられない』ときは、エンジンの `TMsgForm` の種類(本文が読めるようになるまでは、ボタンの組と窓の大きさで区別する: 『Ok と Go..』= 登録コードが不正〔読み〕)と、`importlog` の `Reg*` の有無で原因を切り分ける。
7. 永続の実装(4.3 の方式)を入れた後、ブラウザ(Chrome・Firefox)で同じ通しを 1 回行う。

### 4.4 ファイル橋と依頼者の操作(PM 要件 ③)

エンジンの "Save validation file…"(保存の共通ダイアログ = `IFileSaveDialog`。決定 0188 でモデル済み)と "Load license file…"(開く = `IFileOpenDialog`)は、決定 0193 の橋(`FLE_FILEDLG_BROWSER`、ページの箱 + `<input type=file>` + Download)を通る。**現在の既定の入口 SHIP2 は橋と exp1 の行を含む**(決定 0208、HEAD 74f2d8f〜)ので、保存ダイアログのモデルは出荷の構成にある(A1 で `Save validation file…` が何も起こさなかったのは、act1 が使った `ia2-imports.txt` に exp1 の行が無かったため。§3)。opfs1 の段階 2 は、この箱をエクスプローラー(選択モード)に差し替える。**要件**:

1. 保存モード: エンジンの提案名(`licensevalidation.txt`)をそのまま出し、保存先を OPFS にし、**依頼者がそのファイルをダウンロードできる**(Files 窓の『ダウンロード』、またはページの Download)。
2. 開くモード: フィルター(エンジンが `SetFileTypes` で渡す拡張子。未観測、§3.3)に従い、OPFS 内または**ローカルからのアップロード**で選べる。
3. 検証ファイルと `FLRegkey.Reg` は**サーバー・R2・git・ビーコン・ログに行かない**(EULA の鍵の機密。§1.3)。ブラウザの OPFS とダウンロードだけ。
4. 保存・開くの結果はエンジンの `Show()` が返す(エンジンが「保存した」「開けない」を自分で判断する)。ページは判断しない。

**依頼者の操作(5 行。▲ は『opfs1 の段階 2 の決定後に更新』する所)**:
1. ブラウザ版の FL Studio で `HELP → Unlock FL Studio → Unlock with file… → Save validation file…` を押し、保存の箱で名前を `licensevalidation.txt` のまま保存する。保存されたファイルを ▲Files 窓の『Downloads』(保存先は opfs1 段階 2 で決まる)から自分の PC へダウンロードする。
2. **自分の PC のブラウザの別タブ**で Image-Line にログインし、`Unlock products → Download Offline Unlock File for FL Studio 20.9 or Later → this link` で 1 のファイルを Upload し、`FLRegkey.Reg` をダウンロードする(パスワードをこちらに渡さない。鍵は自分の PC の安全な場所に残す)。
3. ブラウザ版の FL Studio で `Unlock with file… → Load license file…` を押し、▲箱(またはアップロード)で 2 の `FLRegkey.Reg` を選ぶ。
4. エンジンが "The registration succeeded." / "The application will now close." を出したら、ブラウザのページを再読み込みする(公式の『Restart FL Studio』)。※『TMsgForm の本文が描かれない』問題(§3.2、txt1 が調査)が直るまで、メッセージの文言は読めない。窓が閉じた/変わったことで判断する。
5. `HELP → About` の上部に名前とライセンスが出て、保存で『Buy FL Studio』が出なくなったことを確認し、結果(成功か、エンジンのエラーの種類)だけを報告する。`FLRegkey.Reg` と検証ファイルは報告に添付しない。

### 4.5 再起動と状態変化の観測(PM 要件 ④)

- **再起動 = ページの再読み込み**。"The application will now close." のあと、エンジンは終了する(`ExitProcess` 相当、読み。f_10d1ab0 の文字列)。ページは『エンジンが終了した』を既に表示できる(エンジン終了の表示)ので、ユーザーに『再読み込みしてください』と出すのは**ページ側の 1 行の案内**でよい(エンジンの表示を置き換えない)。再読み込み時、レジストリの部分木は OPFS から入口の前に復元される(4.3)。
- **認証後に変わると読む物(観測方法)**: (a) `HELP → Unlock FL Studio` のフォームの題 `FL Studio is in trial mode.`(`TUnlockForm`、観測 §3.2)が出なくなる、または別の状態になる、(b) `File → Save` で『Buy FL Studio』(`TInAppShopForm`、決定 0191)が出ない、(c) `HELP → About` の上部に名前とライセンスコード("Your name and license code will show at the top of the HELP > About panel"、Getting Started Manual)、(d) `Options → General settings(F10)→ Account` の "License plate"(公式マニュアル envsettings_account.htm)。native -O0 の fork プローブで、**レジストリを復元した状態から起動して (a)(b) の窓表(`g3-win`)を比べる**のが再現可能な観測(`runs/` の手順と同じ。実鍵が要る)。
- **依頼者の実鍵が無ければ確認できないこと**: エンジンが本物の `FLRegkey.Reg` を受け入れるか(lift の不具合、機械識別の不一致、版の不一致 = "Your registration doesn't include a license for this version")。受け入れない場合は、エンジンが §2.3 の文言を出すので、それを報告してもらう。

### 4.6 失敗時の表示

エンジン自身のダイアログに任せる。偽装・補完はしない。例: "The registration code is incorrect. Please download it again from your user account."、"Your registration doesn't include a license for this version of %s."、ネットワークが必要な操作を押したとき "Failed to download the license. Please check your internet connection and try again or use the |Unlock with file| method."(f_efcf70)。**§5 の小さな実装は、WinINet の失敗を『停止』でなく文書どおりのエラーにするので、アカウント認証を押してもエンジンは落ちず、上の文言を出す**(実エンジンから層 5 が呼ばれたかは §5)。

**ただし現状、`TMsgForm` の本文が描かれない**(§3.2。『Error』も『File saved』も本文領域が 1 色)ので、依頼者はこれらの文言を読めない。ボタンの組(『Ok』と『Go..』= 登録コードが不正〔読み〕)と窓の大きさ・題でしか区別できない。原因の調査は txt1(決定 0219)が引き継いだ。**案 A の依頼者向けの手順(§4.4)は、この問題が直るまで『文言が読めない』前提で書いてある**。ページ側にエンジンの文言を補う表示は足さない(偽装になりうる)。

### 4.7 作業量(推測)

| 項目 | 内容 | 日数(推測) |
|---|---|---|
| 識別 | `build-engine-bundle.sh` に入力(`FL_MACHINE_GUID`)を 1 つ足し、1 行の種ファイルを作って `--extra` と `FLE_REG_SEED` に足す(runtime の新規コードなし)。native のプローブと Chrome/Firefox で検証ファイルが配備ごとに決まることの確認 | 0.5 日 |
| 永続 | `fle_reg.c` の dirty と直列化、ホスト関数、Worker の OPFS 書き込み、入口前の復元、テスト(Chrome + Firefox) | 3〜4 日 |
| 保存・開く | opfs1 の段階 2(保存モード・開くモード・ダウンロード)に任せる。act1 としては、検証ファイルの保存名とフィルターの確認 | 1 日(opfs1 に含まれない分) |
| 通しテスト | 依頼者の実鍵 1 回(§4.4 の 5 行)。native のプローブで復元後の窓表 | 0.5 日 + 依頼者の操作 |
| **合計** | | **約 1 週間(5〜6.5 日)** |

### 4.8 案 B の記録(採らないが、何が要るかの記録)

- **WinINet → fetch の橋**: ゲストの `HttpSendRequestExW`/`InternetReadFile` は同期。ブラウザ側は非同期の `fetch`。JSPI(決定 0193 が使っている)または `SharedArrayBuffer` + `Atomics.wait`(依頼者のブラウザは `crossOriginIsolated true`)。エンジンの Worker から `fetch` を直接呼べる。
- **禁止ヘッダー**(MDN "Forbidden request header"、`sources/mdn-forbidden.txt`): `Host`・`Accept-Charset`・`Keep-Alive`・`Connection`・`Content-Length`(エンジンは 5 つとも自分で付ける)は設定できない。`User-Agent` は Chrome が黙って落とす。ページの CSP は `connect-src 'self'` で外へ出ない。CORS は**要求を送っていないので不明**。→ 同一オリジンの中継が必須。
- **Cookie**: エンジンは NO_COOKIES を付けず、Cookie の保存と送信は WinINet 任せ(エンジンは応答ヘッダー全体を `HttpQueryInfoA(0x16 = RAW_HEADERS_CRLF)` で読む)。`fetch` では `Set-Cookie` は応答ヘッダーとして読めない(禁止された応答ヘッダー名)ので、橋は C 側に Cookie 保存(ドメイン・パス・Secure・有効期限の最低限)を持ち、中継が `Set-Cookie` を別名のヘッダーに写し、`HttpQueryInfoA(0x16)` の文字列を C 側で組み立て直す必要がある。リダイレクトは自動追従 → 中継は `redirect: 'manual'`。証明書の検証は中継の実行環境(ゲストに TLS 検証の呼び出しは無い)。
- **中継**: `app/tools/`(vite middleware)と `app/worker/`(Worker)に `POST /__net/relay`(本文 = 上流の URL・ヘッダー・本文)、宛先の許可リスト(`support.image-line.com`・`www.image-line.com`)、内容を改変せず、URL・ヘッダー・本文をログに残さない。ループバックのダミー上流でテスト。
- **資格情報のリスク**: エンジンが URL のクエリにメールとパスワードを入れる(§2.2)。中継を POST の本文にしても、中継から上流へはそのまま送る。中継の実行基盤が復号して見る。ブラウザのメモリに現れる。2FA の確認コードを窓に入れる必要。**漏れたパスワードは取り返しがつかない**。緩和: 使い捨てのパスワード、2FA、認証後に変更。
- **見積り**: 約 3 週間(推測)+ 実サーバーでの確認が依頼者の操作で 1 回以上。Image-Line のサーバーがデータセンターの IP の要求を弾くか(サーバーは Cloudflare の背後に見える)は、**要求を送っていないので不明**。

## 5. 小さな実装(opt-in): ゲストの WinINet を『停止』でなく文書どおりのエラーで終える

**何をしたか**(新規 `lift/runtime/fle_act1_net.c/.h`、層 5 の行 `lift/harness/seeds/act1-imports.txt`(10 行だけ)、テスト `lift/crates/fe-lift/tests/act1_net.rs`、既存 `fle_wininet.c`(+11)・`fle_wininet.h`(+5) への追記 16 行・削除 0 行):

- 層 5 の行(`wininet.dll <API> fle_imp_<API>` の 10 本)があるときだけ、エンジンの HTTP 部品 f_64d520 が呼ぶ次の API が答えられる。**行が無い構築は 1 バイトも変わらない**(既定の生成 C の sha256 は `cd9107ce…` のまま〔実測、`logs/felift-default-act1.sha`〕。行のある構築の生成 C は、その行の分だけ別物)。
  `InternetConnectW`(HTTP: ハンドルを返す。Learn: "for others, the actual connection is not established until the application requests a specific transaction")、
  `HttpOpenRequestW`(ハンドル)、`HttpAddRequestHeadersW`(TRUE。ヘッダーは名前だけ記録)、
  **`HttpSendRequestExW` → FALSE + `ERROR_INTERNET_CANNOT_CONNECT`(12029)**(`-DFLE_ACT1_NET_ERROR=12007` で `ERROR_INTERNET_NAME_NOT_RESOLVED`)、
  その後の `InternetWriteFile`・`HttpEndRequestW`・`HttpQueryInfoA/W`・`InternetReadFile` → FALSE + `ERROR_INTERNET_INCORRECT_HANDLE_STATE`(12019)、
  `InternetGetLastResponseInfoW` → TRUE・エラー 0・空の文字列。`InternetCloseHandle` は `fle_wininet.c` の弱い参照 `fle_act1_close_handle` で新しいハンドルも閉じる。
- **偽の成功は返さない**: 転送が成功する答え(`HttpSendRequestExW` の TRUE、応答の本文、ステータスコード、Cookie)は存在しない。応答は常に『サーバーに届かない』。**ライセンス・認証の判定には一切触れない**(エンジンが失敗を見て、自分のメッセージを出す)。
- 1 回の呼び出しごとに、stderr へ **タブ区切りの 1 行**(`fle: act1-net<TAB>#n<TAB>API<TAB>…`)を書く: ホスト・ポート・動詞・オブジェクト名・クエリの**名前と値の長さだけ**・ヘッダーの**名前だけ**・バイト数。**値・本文・トークンは書かない**(テストで固定: `SECRETUSER`・`SECRETPASS`・`Bearer` が stderr に出ないこと)。
- ニュース取得の `InternetOpenUrlW`(決定 0132、NULL + 12007 のまま)にも同じ形の 1 行を足した(`fle_wininet.c` から弱い呼び出し。**答えは変えない**)。ニュースの URL は `…/downloadnewsrss&flsdata=…&id=…`(`?` なしの `&` 区切り)なので、`&` 以降も値を伏せる。
- **実エンジンの走行では層 5 の関数は一度も呼ばれなかった**(A1・A2 の 30 本ほどのプローブで 0 回。アカウント認証は『同意』のチェックの先にあり、"Redeem code" はブラウザを開くだけ。§3.2)。確かめたのは単体テストと静的な読みだけ。`InternetOpenUrlW` の 1 行(起動時の 2 本)だけが実エンジンで出た。
- テスト `act1_net.rs` 9 件(合格): 要求の列〔Connect → Open → AddHeaders → Send(12029) → Write/End/Query/Read(12019) → Close x3〕、記録の伏せ字、12007 への切り替え、`InternetGetLastResponseInfoW`、**見たことのない形(FTP、ユーザー名つき Connect、NULL のオブジェクト名、2 回の Send ほか 9 通り)は名前つき停止のまま**、NULL の動詞は GET、act1 のファイルを入れない構築では未知のハンドルの Close が従来どおり停止、ニュース URL の伏せ字。既存の `wall_fr13`(WinINet の 5 テスト)も合格のまま。

## 6. 制約の遵守・出典のない選択・限界

### 6.1 絶対の制約(PM の恒久の方針。決定 0183/0190/0191 の PM 節)の遵守

| 制約 | この作業での事実 |
|---|---|
| ライセンスの判定を偽装・迂回しない | **0 件**。エンジンの判定(`ILRegKey` と機械識別の照合、f_10c2010 ほか)は**読んでいない**(場所を特定しただけ)。試用版のダイアログ(『Buy FL Studio』)は A1/A2 の走行でエンジン自身が出した物で、回避していない |
| 偽の認証応答・偽のネットワーク応答・トークンの捏造 | **0 件**。層 5(`fle_act1_net.c`)は転送が成功する答えを持たない(`HttpSendRequestExW` は FALSE + 12029、以後は 12019。mutation M2 〔偽の TRUE〕をテストが検出する: `logs/mutation-act1.txt`) |
| ライセンスファイルの偽造 | **0 件**。`FLRegkey.Reg` の中身を作らず、解析もしていない。§3.3 で『Load license file…』に渡したのは**エンジン自身が書いた検証ファイル**(明らかに登録コードではない物)で、エンジンの拒否を観測する否定のテストであり、鍵の偽造ではない |
| 観測のためにエンジンの外向きの要求を記録して『ネットワークエラー』を返すのは可 | 層 5 の記録は host・port・verb・object・クエリの**名前**・ヘッダーの**名前**・バイト数だけ。値・本文・トークンは出さない(テストで固定) |
| 依頼者の資格情報(パスワード・トークン・認証コード)をログ・ファイル・git・ビーコンに残さない | **0 件**。資格情報は一切入力・取得していない(メール・パスワードの欄は空のまま。『同意』のチェックも入れていない) |
| 実際の Image-Line のサーバーへ、依頼者のアカウントでの認証要求を送ってはいけない | **0 件**。Image-Line へ送った物: 公式ドキュメント・EULA・公開ページ(ログイン前)・Microsoft Learn・MDN の GET と、公開 2 ページ(`support.image-line.com/action/login`・`www.image-line.com/`)の**ヘッダー取得**(`curl -D`)のみ。ログインページの GET は Cookie を受け取った(`flSession`・`_cfuvid`。値は保存していない)が、認証要求ではない。エンジンの通信は**常に『届かない』**で、実サーバーへは 0 件 |

### 6.2 出典のない選択(S- の番号。全文は `tables/s-choices.tsv`)

S-act1-net-error(12029、代替 12007)・S-act1-connect-lazy(Learn どおり。出典あり)・S-act1-state-error(12019)・S-act1-lastresp-empty(出典なし)・S-act1-log-redaction・S-act1-forms-stop・S-act1-handle-region・S-act1-flag-values(wininet.h の数値。UNOFFICIAL)・S-act1-machine-id(設計: マニフェストの定数。出典なし)・S-act1-estimates(作業量。出典なし)・S-act1-relay-shape(案 B の記録)・S-act1-probe-states(プローブの出発点)・S-act1-test-guid(テスト用の MachineGuid)・S-act1-no-agree(同意のチェックを入れない)。

### 6.3 限界(そのまま記録)

- 観測は native -O0 の fork プローブだけ。**Chrome・Firefox の出荷バンドルでの確認は 0 件**。実機 Windows との照合は 0 件。
- **依頼者の実鍵(`FLRegkey.Reg`)での確認は 0 件**。エンジンが本物のファイルを受け入れて認証済みになるか、レジストリへの書き込みが lift したコードで通るかは**未確認**(§4.5)。私は鍵を作らない。
- 検証ファイルの中身は解析していない(機械識別との関係は、材料を変えたときの差だけを見た。§3.3)。Image-Line のサーバーが、このブラウザ版の識別で作られた検証ファイルを受け付けるか(`FLRegkey.Reg` を発行するか)は**未確認**(送っていない)。
- 「Sign in and unlock」の結果の戻し方(案 C)は**未確認**(同意のチェックを入れないため、開く URL を観測していない。静的には `redirect/WebsiteUnlock?machineid=…` と読む)。
- `TMsgForm` の本文が描かれない問題の**原因は未確認**。
- 作業量は私の見積り(推測)で、実測ではない。
- EULA の読みは私の読みで、法的な判断ではない。

---

## PM の決定と取り込み(2026-11-04)

- **取り込み**: `act1.diff`(6 ファイル、削除 0 行)を当てた。層 5 の行(`act1-imports.txt`)は opt-in で、既存の構築には何も足さない。FLEngine の既定の生成 C は `cd9107ce…` のまま。PM 実測: 全体テスト 183 results / 2,307 passed / 0 failed(基準は 180 / 2,287。+api2 + dec1 wall_dec1 7 + act1_net 9)、a4 FRUITY 無効 942/942、895/895、関数 5,310、入口 1,009,731(不変)(dec1・api2 と同時の 1 回)。
- **方針(担当案 A を採る)**: 認証は、エンジン自身の『ファイルでアンロック』の手順(公式の KB の手順)で行う。検証ファイルの保存、依頼者のアカウントでの鍵の取得、鍵ファイルの読み込み、の順。偽の判定・偽のトークン・資格情報の URL 渡し(案 B)は採らない。鍵は検証ファイルを作った機械でのみ有効(公式の記述)なので、**依頼者側の作業が必要**: (1) このサービスで出した検証ファイルを自分のアカウントでアップロードする、(2) 鍵(FLRegkey.Reg)を OPFS の領域に置く。手順は決定文書の 5 行。OPFS 側の保存先と永続化(`HKCU\Software\Image-Line\Registrations`)は opfs1 の段階 3 で実鍵により確認する(未確認)。
- **MachineGuid**: 既存の `FLE_REG_SEED` の 1 行(`HKLM\SOFTWARE\Microsoft\Cryptography<TAB>MachineGuid<TAB>REG_SZ<TAB><guid>`)で与える。値は依頼者の機械ごとに固定して持ち、リポジトリには置かない(テスト用の値は全て `00000000-0000-4000-8000-0000000a0001` の偽物)。
- **やらないこと**: ライセンス検証の迂回・回避、試用版ダイアログの無効化、ネットワークの偽応答。起動時の WinINet の 2 本(news/versions)は『サーバーに届かない』で答える(偽の成功なし)。
- **EULA**: 『逆アセンブル・逆コンパイル・派生物の作成を行わない』『サービスとして第三者にアクセスさせない』という条項があることを事実として記録する(法的判断はしない。配備は依頼者のみがアクセスする私的なもの)。
