# deploy.yml の E2E の判定(Issue #1407)。入力は run.mjs の出力(表示ごとに 1 行の JSON)。-s で全部を読む。
# 全 4 行が servedHeadersMatch で、error が無く、シナリオが 3 つ。印(expectFail)の無いシナリオは ok と各カウンタ 0。
# 印の有るシナリオは ok を問わず、各カウンタ 0。通ったら標準エラーに警告を出す(落とさない)。achievedFps は数値であればよい(120 Hz に届かなくても落とさない)。
(.[].scenarios[] | select(.expectFail != null and .ok == true)
  | debug("E2E 警告: \(.name) は印 \(.expectFail) の付いた既知の欠陥だが通った。印を外す") | empty),
(length == 4 and all(.[];
  .servedHeadersMatch == true and .error == null
  and (.display.achievedFps | type) == "number"
  and (.scenarios | length) == 3
  and all(.scenarios[];
    .pageErrors == 0 and .consoleErrors == 0 and .stuckToasts == 0
    and .failedRequests == 0 and (.expectFail != null or .ok == true))))
