【ci1 至急: 8 モジュールの本番デプロイが止まっている。原因は R2 の lifecycle の step(権限)。直しの差分あり】

事実(CI のログ):
- run 41987198683(a6331e92)の build job は成功(34 分)。deploy job が step 13「R2: the beacons (perf/) expire after 180 days」で失敗: `aws s3api get-bucket-lifecycle-configuration` が `AccessDenied`(キャッシュ用の R2 認証は『オブジェクトの読み書き』で、バケットの設定は読めない)。ci/stage-deploy.sh lifecycle がそこで die する。
- その時点で済んでいる: step 12「upload the bundle's files」(オブジェクトは R2 に上がった)。済んでいない: step 14 Worker の新版、step 15 既定 manifest の切替、step 16 Access の確認。**本番は切り替わっていない**(e61f80af〔5c4b6902〕のまま)。
- run 41988827762(5535f603、0248 の push)も、build は bundle キャッシュが当たって skipped、deploy が failure(同じ step)。今後の push も、この step が通らない限り全部同じ所で止まる。

直し(lift/out-git/ci1-results/c5ed023b/ci1-lifecycle.diff、基準 HEAD 5535f603、`git diff` 63 行。ci/stage-deploy.sh と ci/test-lifecycle.sh だけ):
- lifecycle の読み/書きが `AccessDenied` のとき: `::warning title=R2 lifecycle not applied::…` と step summary の 1 行を出して exit 0(デプロイは続く)。perf/ の 180 日の期限切れは衛生用で、デプロイの条件にしない。それ以外のエラー(接続できない等)は今までどおり失敗。旗・設定なし。
- ci/test-lifecycle.sh に stand-in の `aws` で 5 通り: 読み AccessDenied→警告で rc 0、読み NoSuch+書き ok→追加で rc 0 警告なし、読み NoSuch+書き AccessDenied→警告で rc 0、読み他のエラー→rc 2、書き他のエラー→rc 2。全部通過。shellcheck -S warning は clean。
- 別の道(認証を直す)は R2 の Admin 権限のトークンが要り、依頼者に頼むことになる(owner-contact に反する)ので取らない。perf/ の期限切れは、そのトークンが無い間は掛からないままになる(警告として CI に出る)。

手元の bundle(c5ed023b)は完成: wasm 141,795,291 B、sha256 3d0279d45493e42eb5d86e189700e5d701cef808010685e63eb3119bb5b5cac3(期待値と一致)。いまから Chrome/Firefox の確認と、上がった R2 のオブジェクトとのバイト比較(CI ログの値)に進む。報告3 は 60 分以内。
