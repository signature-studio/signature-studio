# 0052: VirtualAlloc が MEM_TOP_DOWN を受理する（lpAddress が NULL のときだけ。アリーナは上端から割り当てる）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。0017 C の「MEM_TOP_DOWN は abort」を、下の範囲に限って置き換える。それ以外の未対応フラグの扱い（0017 C）は変えない。実行記録は `docs/investigations/virtualalloc-top-down-run.md`。

## 背景

harness は `VirtualAlloc flAllocationType 0x101000 is not supported` で止まっていた（`adjustor-thunks-run.md` §5）。呼び出しは `f_407770` の 0x40779b で、`rcx=NULL`、`r8d=0x101000`、`r9d=4`（PAGE_READWRITE）、thunk は `f_4073c0`（IAT スロット 0x19715d0）。

## 一次資料で確認した値

出典: Microsoft Learn, "VirtualAlloc function (memoryapi.h)", https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc （取得 2026-10-21、ページの `updated_at` は 2025-07-01）。

| 名前 | 値（ページの表） |
|---|---|
| MEM_COMMIT | 0x00001000 |
| MEM_RESERVE | 0x00002000 |
| MEM_RESET | 0x00080000 |
| MEM_RESET_UNDO | 0x1000000 |
| MEM_LARGE_PAGES | 0x20000000 |
| MEM_PHYSICAL | 0x00400000 |
| MEM_TOP_DOWN | 0x00100000 |
| MEM_WRITE_WATCH | 0x00200000 |

したがって 0x101000 = 0x1000 (MEM_COMMIT) | 0x100000 (MEM_TOP_DOWN)。ほかのフラグは立たない（0x101000 & ~0x101000 = 0）。

ページが MEM_TOP_DOWN について書いていること（全文）: 「Allocates memory at the highest possible address. This can be slower than regular allocations, especially when there are many allocations.」

## ページが決めないこと

- 「highest possible address」が、アドレス空間のどこを指すか（上限、整列、他の割り当てとの関係）。
- MEM_TOP_DOWN と、lpAddress が非 NULL の組み合わせ。ページは lpAddress が非 NULL のときの MEM_TOP_DOWN の効果を書かない。
- MEM_TOP_DOWN と MEM_COMMIT だけ（MEM_RESERVE なし）で lpAddress が NULL の組み合わせ。ページは MEM_COMMIT 単独を NULL アドレスで使うと領域をシステムが選ぶ、とは読めるが、MEM_TOP_DOWN との組み合わせは書かない。0017 C の「MEM_COMMIT のみ、NULL アドレスは予約とコミットを同時に行う」と同じ推測を、そのまま TOP_DOWN 付きに適用する。ゲストの呼び出しがこの形（0x101000）である。

## 決定

1. **受理する値**: `type` が `MEM_COMMIT`、`MEM_RESERVE`、その両方、のいずれかに `MEM_TOP_DOWN` を足したもの（0x101000、0x102000、0x103000 を含む）。MEM_COMMIT も MEM_RESERVE も無い値（MEM_TOP_DOWN 単独を含む）、ほかの未知のビット、MEM_RESET などの未対応フラグとの組み合わせは、従来どおり `VirtualAlloc flAllocationType %#x is not supported` で止まる（値の名前でなく 16 進で出す。従来と同じ書式）。
2. **lpAddress が非 NULL の MEM_TOP_DOWN は止める**（`VirtualAlloc flAllocationType %#x (MEM_TOP_DOWN) with lpAddress %#llx is not defined`）。ページが効果を書かない。
3. **配置（アリーナ、wasm）**: MEM_TOP_DOWN 付きで lpAddress NULL の確保は、アリーナの**最も高い空き範囲**を取る。候補の開始を `アリーナ終端 - size` から割り当て粒度（65536）へ切り下げ、既存の予約（`fle_spans`、アドレス順）と重なれば、その予約の開始を新しい上限にして繰り返す。入らなければ NULL と ERROR_NOT_ENOUGH_MEMORY（8。0017 C の既存の選択と同じ）。下から取る従来の確保（MEM_TOP_DOWN なし）は変えない。
4. **配置（native）**: 変えない。ホストの `mmap(NULL)` が場所を選ぶ。MEM_TOP_DOWN は受理するが、配置には効かない（Linux の `mmap` の既定は上から割り当てるが、これはカーネルの挙動で、本実装は保証しない）。
5. **サイズ、ゼロ初期化、領域の記録、保護**: MEM_TOP_DOWN の有無で変わらない（`alloc_protect`、ページ状態、コミット時のゼロ化は既存のコード経路）。

実装: `lift/runtime/fle_sys.c`（`FLE_MEM_TOP_DOWN`、アリーナの `fle_host_claim(want, size, top)`、`fle_map_reserve(want, size, top)`）。`fle_sys_host_reserve`（fle_vmap.c が使う公開関数）は従来どおりで、`top` なしで `fle_host_claim` を呼ぶ。

## 選択肢

- A. MEM_TOP_DOWN を無視して、フラグを落として通常の確保にする。変更が最小だが、「highest possible address」を満たさない。ゲストがアドレスの高さに依存するか（0x407783 の `and rdi, 0xffffffffffff0000` と 64 ビット格納は読めているが、高さの依存は見ていない）わからないので、仕様のとおりに配置する案 B を採る。
- B（採用）. アリーナでは上端から取る。仕様の文言に最も近く、実装も小さい（アリーナの表は既にアドレス順）。
- C. 別のアリーナ（上位領域）を用意して top-down はそちらから取る。アリーナを 2 つ持つ理由が仕様にない。却下。
- D. lpAddress が非 NULL でも受理して無視する。ページが効果を書かないので、止める（決定 2）。

## 採用理由

- 仕様が書くのは「highest possible address」だけで、アリーナの上端が「highest possible」に最も近い解釈。
- アリーナは wasm の実行にだけ存在し、native はホストが決めるため、配置の差は native と wasm の実行の差になりうる。0045 が既に認めている差（保護が効かない、アドレスの値が違う）の延長で、停止点の一致は検査している（`investigations/virtualalloc-top-down-run.md`）。
- 粒度への切り下げは、0017 C の「NULL アドレスの基準は割り当て粒度に整列」と揃える。

## 撤回条件

次のどれかが見えたら、この決定を見直す（案 A に戻す、または配置を変える）。

- ゲストが MEM_TOP_DOWN の確保に対して、高いアドレスであることに依存しない（動作が配置に左右されない）ことが示され、かつ上端の配置が他の確保（`fle_vmap.c` のファイルマッピングなど、下から取る確保）と衝突して失敗を起こす。
- 実 Windows の測定、または Microsoft の別の文書で、MEM_TOP_DOWN の配置（上限、整列、lpAddress 非 NULL の扱い）が分かり、本決定の 3 と食い違う。
- アリーナの大きさ（`FLE_VMEM_ARENA_BYTES`、512 MiB）が変わり、上端の意味が変わる。

## 確認できていないこと

- 実 Windows で MEM_TOP_DOWN が返すアドレス。本実装の値は仕様の解釈で、実機と一致する保証はない。
- ゲストが返されたアドレスの高さに依存するか。
- native で MEM_TOP_DOWN が配置に効かないことが、ゲストの実行に影響するか（現時点の停止点までは、native と wasm の出力が一致した）。
- MEM_TOP_DOWN と MEM_COMMIT 単独（MEM_RESERVE なし）の NULL アドレスは、0017 C と同じ推測（予約とコミットを同時に行う）に基づく。
