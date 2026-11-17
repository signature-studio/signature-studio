# 0005: STMXCSR と LDMXCSR の応答

状態: 作業者（PM）が決定。依頼者の決定は含まない。実装は `lift/crates/fe-lift/src/mxcsr.rs`（振り分けは `lib.rs` の 1 行）、runtime は `lift/runtime/fle_mxcsr.h` と `fle_mxcsr.c`（新規）。

## 文脈

前回（`lift/out-tls2/run2.txt`）の停止点は 0x40a3b2 の `stmxcsr (%rsp)`。thunk 0x40a3b0 は `push 0; stmxcsr [rsp]; pop rax; ret`。呼び出し元は 0x412281 の `call 0x40a3b0`。詳細は `docs/investigations/import-none-0x40a3b0.md`。

## 仕様の出典

- STMXCSR: https://www.felixcloutier.com/x86/stmxcsr （非公式の転載。ページ冒頭に「UNOFFICIAL, mechanically-separated, non-verified」とある。2026-10-20 取得）
  - 読んだ箇所: 「Stores contents of MXCSR register to m32」「The reserved bits in the MXCSR register are stored as 0s.」「m32 := MXCSR」
- LDMXCSR: https://www.felixcloutier.com/x86/ldmxcsr （同上。2026-10-20 取得）
  - 読んだ箇所: 「Loads the source operand into the MXCSR control/status register. The source operand is a 32-bit memory location.」「The default MXCSR value at reset is 1F80H.」「#GP: For an attempt to set reserved bits in MXCSR.」「MXCSR := m32」
- MS の公式: `_mm_getcsr(void)` は `unsigned int`、`_mm_setcsr(unsigned int)` は `void`（x64 intrinsics 一覧 https://learn.microsoft.com/en-us/cpp/intrinsics/x64-amd64-intrinsics-list?view=msvc-170 、2026-10-20 取得）。MS の個別ページは試した URL が 404 で、解説文は未確認。
- Rust の `_mm_setcsr` の説明（doc.rust-lang.org）: MXCSR を設定する。ただし丸めや masking を変えると未定義動作になり得る、と書いている。これは lift の対象外の問題（下の「仕様の外」を参照）。
- 公式 PDF（SDM Vol. 1 §11.6.4、Vol. 2 の STMXCSR、LDMXCSR）は未取得。felixcloutier の転載の内容を、公式と同じとみなすかは未確認。

## 決定

1. STMXCSR m32: runtime の `fle_mxcsr`（既定 0x1F80）の 32 bit を m32 に書く。
2. LDMXCSR m32: m32 の値を読む。予約ビット（31:16）が 1 なら stderr に出して abort（`fle_unsupported` と同じく止める。黙って無視しない）。
3. MXCSR は runtime の 1 つのグローバル変数。`fle_runtime.h` は変えない。

## 仕様から言えること／言えないこと

- 言える: 既定値 0x1F80、stmxcsr の書き込み、ldmxcsr の予約ビットの #GP（felixcloutier の転載）。
- 言える（実測）: ホスト x86_64 で予約ビットの LDMXCSR は SIGSEGV で止まった（テスト `ldmxcsr_reserved_bit_*`、Linux の実測）。
- 言えない（未確認）:
  - 公式 PDF の本文との一致。
  - 生成 C の abort が仕様の #GP と同じ扱いになるかどうか（abort は作業者の選択。仕様は例外の種類を強制しない）。

## 仕様の外（作業者の判断）

- `fle_mxcsr` の rounding と masking は、`fle_sse.h` の SSE 演算に反映しない。現状の SSE は既定（RN、例外 masking）で動く。LDMXCSR で rounding や FTZ/DAZ を変えたあとの演算の結果は、lift では再現されない。今回の停止点の範囲では、読み出し以外の利用は確認していない。
- 将来、呼び出し元がこの値で演算を変えるなら、SSE 側の対応が要る（この決定の対象外）。
