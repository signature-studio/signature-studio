# 0335 O19: キー(VK shift)のうち FL に届かない物 — `app/src/input/keymap.ts` の全表とブラウザの予約

**実際に実行した物**: なし。読みのみ(`app/src/input/keymap.ts` 全行、`app/src/input/pointer.ts` の onKey、`app/test/unit/keymap.test.ts`、Chromium の `chrome/browser/ui/accelerator_table.cc` と `browser_command_controller.cc` の `IsReservedCommandOrKey`、Firefox の `browser/base/content/browser-sets.inc`、FL Studio の公式ショートカット表、W3C の `UI Events Code` の code 一覧)。**実行していない**: ブラウザ起動、キー送信、リロードの実測、F5 を転送したときの実機確認。**差分は作っていない**(PM の許可後)。class は **(b) 外部 API の契約の不足寄り**の話であるが、F5 を転送するためとは言っていない。単なる survey。

## 0. 結論(読み)

- `keyPolicy` が **forward しない + prevent しない**(= ページが受けてブラウザへ素通しする)キーは、事実上 **3 群**である: (i) 明示的にブラウザへ渡す (Tab、F5、Ctrl/Cmd+W)、(ii) 修飾そのもの (MetaLeft/MetaRight)、(iii) IME 変換中 (isComposing / keyCode 229)、(iv) 表に無い code → `unknown-code`。**F5 はそのうち「ページが意図的に手放した」1 つ**(`keymap.ts:60`: `if (e.code === 'F5') return { forward: false, prevent: false, why: 'reload' }`)。**F5 を押すとブラウザがリロードし、開いていた project が消える**。これは O19 の報告そのもので、再現は 13:32 の info で既に確定済み。
- **F5 は、ページから転送 + preventDefault できるはず**である。Chromium の予約命令一覧(`kReservedCommandIds`)にも Firefox の `reserved="true"` の一覧にも F5 は無い。ただし実機の Firefox 159(依頼者の)と Chrome 156 のどちらでも**未確認**であり、『できるはず』に留める。
- Chrome がページから**取れない**キー: Ctrl+W / Ctrl+T / Ctrl+N / Ctrl+Shift+N / Alt+F4 / Ctrl+Shift+T / Ctrl+1〜8 / Ctrl+Tab 等(下の表)。これらに重なる FL の操作は無い(F10 等は別)。
- FL が使う F キー(f1〜f12)の到達性: **F5 だけが現状届かない**。他は全部転送 + ブラウザ動作は抑止済み(F11 / F12 の実機到達は未確認)。

## 1. keyPolicy の全表(`app/src/input/keymap.ts:57-64`)

| # | 条件 | forward | prevent | why | 備考 |
|---|---|---|---|---|---|
| 1 | `e.isComposing \|\| e.keyCode === 229` | 0 | 0 | 'ime' | IME 変換中。触らない |
| 2 | `e.code === 'Tab'` かつ `sendTab` でない | 0 | 0 | 'tab-leaves-the-desktop' | キーボードがデスクトップから出られるようにする('send Tab to the engine' チェックボックスで変えられる。`main.ts:371,374`) |
| 3 | `e.code === 'F5'` | 0 | 0 | 'reload' | **O19: ページがリロードし、project が消える** |
| 4 | `e.code === 'KeyW'` かつ (ctrl \|\| meta) | 0 | 0 | 'close-tab' | ブラウザがタブを閉じる。**ページは止められない**(下表) |
| 5 | `e.code === 'MetaLeft' \|\| 'MetaRight'` | 0 | 0 | 'meta' | OS が取る |
| 6 | 表に無い code | 0 | 0 | 'unknown-code' | 下の (iv) |
| 7 | それ以外(表にあるキー) | 1 | 1 | 'forward' | **転送し、ブラウザ動作は抑止**(Space のスクロール、Backspace、Ctrl+S、Ctrl+P、F1、… 含む) |

- `forward=1, prevent=1` のキーは、デスクトップにフォーカスがある間だけ送られる(`pointer.ts:69-76` の `onKey`)。フォーカスが他のコントロールにあるときはブラウザの普段の挙動。
- `prevent=1` だが `forward=0` の組み合わせは無い(keyPolicy は 3 種類だけ: 1/1、0/0、0/0)。「ブラウザに渡さず、ゲストにも渡さない」ことはできない。

## 2. 表に無い code(= 6 の 'unknown-code')

KEY_TABLE は 103 エントリ(W3C の code 一覧と突き合わせると、欠けているのは **AltGr, Command, Eject, Fn, FnLock, Help, Meta, Power, Shift, Alt, Control**(汎用名) の 11 個)。**KeyboardEvent.code が実際に発火する値は具体名**なので、「表に無い」は事実上 **AltGr / Help / Fn / FnLock / Eject / Power** に限られる:
- Chrome の `code` は左/右修飾を区別するので、汎用名(Shift / Alt / Control / Meta)は実質発火しない(通常 `ShiftLeft/ShiftRight` 等)。
- AltGr は Windows の Chrome で通常 `AltRight` として来るので表にある。Help は X11 系で出得る。
- Fn / FnLock / Eject / Power は多機能キー。これらを送る意味は無い。
- **したがって unknown-code で FL に届かない物は、事実上 AltGr・Help・多機能キーだけ**で、FL の操作に重なる物は無い(AltGr 経由の入力は AltRight として表にある)。FL の実操作への影響は無い。

## 3. Chrome / Firefox が予約するキー

### Chrome(Chromium `accelerator_table.cc` の `kReservedCommandIds`、render へ行かない)

| 命令 | キー | 種別 |
|---|---|---|
| CLOSE_TAB / CLOSE_WINDOW | Ctrl+W / Ctrl+Shift+W | ページは止められない |
| NEW_TAB | Ctrl+T | 同上 |
| NEW_WINDOW / NEW_INCOGNITO_WINDOW | Ctrl+N / Ctrl+Shift+N | 同上 |
| NEW_ISOLATED_WINDOW | (incognito) | 同上 |
| RESTORE_TAB | Ctrl+Shift+T | 同上 |
| SELECT_NEXT_TAB / SELECT_PREVIOUS_TAB | Ctrl+PgDn / Ctrl+PgUp | 同上 |
| CYCLE_TO_NEXT_TAB / CYCLE_TO_PREV_TAB | Ctrl+Tab / Ctrl+Shift+Tab | 同上 |
| EXIT | (Chrome 終了) | 同上 |

`IsReservedCommandOrKey`(browser_command_controller.cc)は、上記 + 全画面中の IDC_EXIT/IDC_FULLSCREEN を予約する。**リロード(F5 / Ctrl+R / Ctrl+Shift+R)は `kReservedCommandIds` に無い**。ただし F5 / Ctrl+R はアクセラレータ表にあるので発行元はブラウザ。**ページが keydown イベントに preventDefault すれば、リロードは止まる**。未確認: 実機(Chrome 156 の実測はしていない)。

### Firefox(`browser-sets.inc` の `reserved="true"`)

| キー | 命令(for のID) |
|---|---|
| Ctrl+N / Cmd+N(Windows ほか) | 新規ウィンドウ(newNavigator) |
| Ctrl+T | 新規タブ(newNavigatorTab) |
| Ctrl+W | 閉じる(close) |
| Ctrl+Shift+W | ウィンドウを閉じる(closeWindow) |
| Ctrl+Shift+P | プライベートブラウズ(privatebrowsing) |
| Ctrl+Shift+Q (Windows/Lin) | アプリ終了(quitApplication) |
| F11(全画面離脱のみ) | 全画面解除(exitFullScreen, disabled) |

**VK_F5 → `Browser:Reload` は `reserved` が無い**。Ctrl+R(Windows は `navigatereloadshortcut` の accel)も reserved でない。**ページの preventDefault で止まるはず**。未確認: 依頼者の Firefox 159。

### 結論(予約の区分)

| 区分 | キー | ページが preventDefault 可能か |
|---|---|---|
| **取れない**(両ブラウザ共通) | Ctrl+W / Ctrl+T / Ctrl+N(・新規ウィンドウ全般) / Ctrl+Shift+W / Ctrl+Shift+T / Ctrl+Shift+N / Ctrl+Tab 系 | 不可(ページは受け取らない) |
| **止められる** | F5 / Ctrl+R / Ctrl+Shift+R / Alt+←→(履歴)/ F1 / F3(検索) / F4(Ctrl) / F6 / F10(メニュー) / F11(全画面 enter) | 可(未検証)。**現状は Ctrl+R は手放していない**(F5 だけ) |
| **FL に綴らない_os 予約の弁明でない**が、実ブラウザで未確認 | F11 / F12 (Chrome: F11→全画面、F12→DevTools はアクセラレータ) | 可否は未確認(下注) |

**Ctrl+R の注意**: 現状はデスクトップにフォーカスがあれば、**Ctrl+R は転送され**(FL の Export wave file)、ブラウザ動作は抑止される(`keymap.test.ts:68` 相当のテストで意図されている通り)。F5 だけが止めていない。**Ctrl+R と F5 を両方「転送 + preventDefault」にしても、ゲストには届かない**。デスクトップにフォーカスが無い(F5 の理由による再読込後など)ときは、Ctrl+R・ブラウザのメニュー・アドレスバーから再読込できる。

## 4. FL が使う F キーと、FL に届くかどうか

F5 / F6 / F7 / F8 / F9 / F10 / F11 / F12 / F3 / F4 の公式定義(FL Studio のマニュアル「Keyboard & Mouse Shortcuts」の Window Navigation): 

| キー | FL の操作(公式) | 内部の実機確認 | keymap | 届くか |
|---|---|---|---|---|
| F1 | This Help | — | 表にある | 届く(転送 + 抑止) |
| F2 | Pattern 名編集 / Mixer トラック名変更 | — | 表にある | 届く |
| F3 | アクティブ窓のツールセレクタ | — | 表にある | 届く |
| F4 | 次の空のパターン(名前ダイアログ付き) | — | 表にある | 届く(単独 F4。Ctrl+F4 は Chrome の CLOSE_TAB で予約だが、F4 単独は別) |
| **F5** | **Toggle Playlist(Play の窓の開閉)** | 既テストで開閉を確認 | **手放し(0/0)** | **届かない(= リロードして project が消える)** |
| F6 | Toggle Step Sequencer | 既テストで開閉を確認 | 表にある | 届く |
| F7 | Toggle Piano roll | 既テストで開閉を確認 | 表にある | 届く |
| F8 | Open Plugin Picker(＋Alt+F8: Sample Browser ／ Ctrl+F8: Project Picker) | TPlugListForm の開閉を確認 | 表にある | 届く(単独・Alt・Ctrl とも表にある) |
| F9 | Show/hide Mixer | 既テストで開閉を確認 | 表にある | 届く |
| F10 | Show/hide MIDI settings(TMIDIForm) | 開閉を確認 | 表にある | 届く |
| F11 | Show/hide Song info window | — | 表にある | 届く(ただし Chrome が F11 を全画面に割り当てている場合は不可。未確認) |
| F12 | Close all windows | Native のキー走査で Piano roll 時に `DispatchMessageA` で停止する壁として記録(決定 0215) | 表にある | 届く(ただし Chrome が F12 を DevTools に割り当てている可能性。未確認) |

## 5. F5 を送れるようにするときの要確認(未実施)

- F5 を `{forward: true, prevent: true}` に変えたら、ブラウザで実際にリロードしないか(Firefox 159 / Chrome 156 で再検証が要る)。Ctrl+R が既に届くことを考えれば F5 も届くはずだが、**実検証していない**。
- `keymap.test.ts` の期待値(`Tab, F5 and Ctrl+W are left alone and not forwarded`)と `pointer-dom.test.ts`(F5 は defaultPrevented=false)と `app/README.md` の一行が変わる。
- リロード経路は残る: デスクトップにフォーカスがない時の Ctrl+R / メニュー / F5。

## 6. 状態

- [ ] PM 承認後に F5 の文面変更(キー表 + テスト + README)
- [ ] 実機 Chrome で F5 が FL の Playlist を開閉するか再確認(14:15 以降)
- [ ] F11 / F12 のブラウザ到達の実機確認(Chrome のアクセラレータと競合しないか)

---
未確認: 上記の『preventDefault 可否』はソースの読みであって実機検証ではない。実機(Firefox 159 / Chrome 156)での確認は PM の指示待ち。
