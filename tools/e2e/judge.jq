# deploy.yml の E2E の判定(Issue #1407)。入力は run.mjs の出力(表示ごとに 1 行の JSON)。-s で全部を読む。
# シャード(deploy.yml の matrix。1 シャードに 1 つか 2 つの表示)ごとに判定するので、行数とシナリオ数は決め打ちしない。1 行以上、シナリオは各行に 1 つ以上。
# 全ての行が servedHeadersMatch で、error が無い。印(expectFail)の無いシナリオは ok が真で、各カウンタ 0。
# 印の有るシナリオ(既に在る欠陥。印は Issue の番号)は ok を問わず、pageErrors・consoleErrors・failedRequests が 0 であること。stuckToasts は問わない(その Issue が表示の欠陥を扱う。数は JSON に出る)。
# 通ったら標準エラーに警告を出す(落とさない)。achievedFps は数値であればよい(120 Hz に届かなくても落とさない)。
(.[].scenarios[] | select(.expectFail != null and .ok == true)
  | debug("E2E 警告: \(.name) は印 \(.expectFail) の付いた既知の欠陥だが通った。印を外す") | empty),
(length >= 1 and all(.[];
  .servedHeadersMatch == true and .error == null
  and (.display.achievedFps | type) == "number"
  and (.scenarios | length) >= 1
  and all(.scenarios[];
    .pageErrors == 0 and .consoleErrors == 0 and .failedRequests == 0
    and (if .expectFail != null then true else (.ok == true and .stuckToasts == 0) end))))
