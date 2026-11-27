# deploy.yml の E2E の判定(Issue #1407)。入力は run.mjs の出力(表示ごとに 1 行の JSON)。-s で全部を読む。
# シャード(deploy.yml の matrix。1 シャードに 1 つか 2 つの表示)ごとに判定するので、行数とシナリオ数は決め打ちしない。1 行以上、シナリオは各行に 1 つ以上。
# 全ての行が servedHeadersMatch で、error が無い。シナリオは skipped でなく、dprOk が偽でない(ページの DPR が宣言の表示と同じ。random の行は null)。
# 印の無いシナリオ(expectFail: null)は ok が真で、pageErrors・consoleErrors・stuckToasts・failedRequests が 0。
# 印の有るシナリオ(既に在る欠陥。印は Issue の番号か new)は ok を問わず、pageErrors・consoleErrors・failedRequests が 0 であること。stuckToasts は問わない(その Issue が表示の欠陥を扱う。数は JSON に出る)。
# 印の有るシナリオの、エンジンの停止(engineStops が 0 より大きい、または engineStuckRunning)は失敗。ただし expectStop(停止を宣言した印)が在るシナリオは停止を許す(lib/plan.mjs の scenarioPasses と同じ規則)。
# 通ったら標準エラーに警告を出す(落とさない)。achievedFps は数値であればよい(120 Hz に届かなくても落とさない)。
(.[].scenarios[] | select(.expectFail != null and .ok == true)
  | debug("E2E 警告: \(.name) は印 \(.expectFail) の付いた既知の欠陥だが通った。印を外す") | empty),
(length >= 1 and all(.[];
  .servedHeadersMatch == true and .error == null
  and (.display.achievedFps | type) == "number"
  and (.scenarios | length) >= 1
  and all(.scenarios[];
    .skipped != true and .dprOk != false
    and (if .expectFail != null
      then .pageErrors == 0 and .consoleErrors == 0 and .failedRequests == 0
        and (.expectStop != null or ((.engineStops // 0) == 0 and (.engineStuckRunning // false) == false))
      else (.ok == true and .pageErrors == 0 and .consoleErrors == 0 and .failedRequests == 0 and .stuckToasts == 0) end))))
