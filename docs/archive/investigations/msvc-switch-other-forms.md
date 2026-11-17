# MSVC の `jmp reg` switch 表のうち、決定 0089 の表の形に当たらない形の実数と、3 モジュールの停止点 `0x489f8fb` の形（調査のみ。測定のみ）

状態: **調査（決定ではない）**。ソース（fe-decode、fe-lift、runtime、tests、docs/decisions、docs/README.md、既存の docs）は編集していない。新規ファイルはこの文書と `lift/out-forms/` の下だけ。作業中の骨組み（節ごとに追記して保存している）。

凡例: **実測** = コマンドの出力を見た事実。**読み** = 実測（バイト列・ソース）から読んだ解釈。**未確認** = 確かめていない。**意見** = 作業者の意見（事実ではない）。

## 目次（書く順）

0. 前提（HEAD、入力、コマンド）
1. 形の分類（F0〜F5）と件数（Blend2D 種なし／種あり、dsp_ippv2 `--pdata`／縮小、QFC、FLEngine）
2. 各形の代表例（VA と逆アセンブル 10 行）
3. 停止点 `0x489f8fb`（`f_489f7d0`）の詳細（命令列、表、ガード、表の長さ、移動先、F1 の件数）
4. 停止点の先の見通し（読み。静的）
5. 拡張の選択肢の表（A／B／C）と意見
6. 決定が要る点
7. 限界

## 0. 前提

### 0.1 HEAD と入力（実測）

- HEAD `d618380ca05b24413083b0f083d4aa0b80de4693`（決定 0094）。`git status --short` は未追跡 3 件（`lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`。この調査の前からある）。追跡ファイルの変更なし。この調査が作ったのは `docs/investigations/msvc-switch-other-forms.md`（未追跡）と `lift/out-forms/`（`.gitignore` の `lift/out*/` に当たるので `git status` に出ない）のみ。
- fe-lift: `lift/out-git/target-m94/debug/fe-lift`（SHA-256 `99425babe958f7f7380ea2d46468e45d6dff88fbc5589e41982c5b2f25ddd0e9`。読み取り専用。`CARGO_TARGET_DIR` には指定していない）。
- 入力（SHA-256）: Blend2D `lift/out-rdefs/three/dll/Blend2D.4700000.dll` = `24e01ec9295870db531e87637aca0aa6cb5c397409346e64a2928d706589b4cb`（基底 0x4700000 は測定用の値）、dsp_ippv2 縮小 `lift/out-rdefs/three/dll/dsp_ippv2_x64.2000000.dll` = `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`（基底 0x2000000。測定用。`--pdata` のみの dsp も同じファイル）、QFC `lift/out-qfc/quickfontcache_x64.dll` = `afc5a0ed70eb7e4af3d0f4baf771bd6f53eb014e237f857f23023471047736d2`、FLEngine `binaries/win64/FLEngine_x64.dll` = `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`。種: `lift/out-seed58/seeds/seeds58-4700000.txt` = `ca6439b6…`、`lift/out-rdefs/three/dll/seeds-idx0-plus-initterm2.txt` = `cf664439…`。
- この調査が作ったもの（すべて `lift/out-forms/`）: `tool/`（Rust 製の測定ツール `forms-scan`。`git archive HEAD` で `src/` に展開した fe-pe と fe-decode を**無改変の library** として使う。ワークツリーの crate は使っていない）、`py/`（`forms_fn.py`、`forms_table.py`。pefile 読み取りのみ）、`res/`（サイトごとの TSV、逆アセンブル、集計）、`logs/`、`fn/`（関数のバイト列の切り出し）、`tmp/`。

### 0.2 実行したコマンド（実測。cwd は `lift/`。出力は `lift/out-forms/logs/` と `res/`）

```
# 統計（HEAD の fe-lift。生成 C は /dev/null）→ logs/stats.{b2d0,b2d58,dsp,dspred,qfc,fle}.txt（register_jmp_sites、msvc_tables、msvc_rejected の全件）
fe-lift lift Blend2D.4700000.dll /dev/null --pdata
fe-lift lift Blend2D.4700000.dll /dev/null --pdata --seeds seeds58-4700000.txt
fe-lift lift dsp_ippv2_x64.2000000.dll /dev/null --pdata
fe-lift lift dsp_ippv2_x64.2000000.dll /dev/null --seeds seeds-idx0-plus-initterm2.txt        # 縮小
fe-lift lift quickfontcache_x64.dll /dev/null --pdata
fe-lift lift FLEngine_x64.dll /dev/null --pdata
# 形の分類（forms-scan。closure の `register_jmp_sites` と同じ集合を列挙して分類）→ res/NAME.{sites.tsv,dis.txt,summary.txt,sweeponly.tsv}
forms-scan PE NAME OUT --pdata [--seeds FILE]
# 停止点
python3 py/forms_fn.py Blend2D.4700000.dll 0x489f8fb out-forms/fn/f489f7d0       # .pdata の関数のピースを切り出す
objdump -D -b binary -mi386:x86-64 -M intel --adjust-vma=0x489f7d0 out-forms/fn/f489f7d0.piece0.bin
python3 py/forms_table.py Blend2D.4700000.dll 0x2fd000 17 0x19f7d0 0x19f960      # 表の読み
```

`PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=lift/out-forms/tmp`。native の実行、gdb、3 モジュールの実行はしていない。

## 1. 形の分類（F0〜F5）と件数

### 1.1 方法（実測と読み）

- **母集団**: closure が到達した `jmp reg` の全件 = fe-lift の `register_jmp_sites`（Blend2D 種なし 99、種 58 個あり 100、dsp_ippv2 `--pdata` 836、縮小 563、QFC 5、FLEngine 20。`forms-scan` の列挙数が全構成で `register_jmp_sites` と**一致**。`res/*.summary.txt`）。`.pdata` の線形走査にあるが closure が到達しなかった `jmp reg`（表のバイトを iced-x86 が `jmp reg` と読んだものを含む）は別に数えた（§1.5）。
- **直前の命令列**: `.pdata` の関数（連鎖ピース全部）の iced-x86 の線形走査（`fe_decode::jumptab::Sweep` と同じ手順）で、`jmp` から**連続して最大 8 命令**遡る（非 fall-through の命令で止める）。`.pdata` の外の site は closure が復号した命令を使う。jmp レジスタ J の最後の書き込みを探し、`add J,X`／`mov J,[…]`／`lea J,[…]`／call の戻り値などで形を分ける（`forms-scan` の `classify`）。
- **N（添字の範囲）の見つけ方を 3 通り記録**: (strict) `recognize_msvc` の規則（`cmp I0,imm; ja` が idiom の直前で、間に `cdqe`／`lea B,[rip+d]`／padding `nop` が最大 2 命令）、(relaxed) 間に fall-through で I0 を書かない命令が最大 6 命令挟まってよい、(edge) idiom のあるブロックへ入る**唯一の辺**が `cmp I0,imm` の直後の条件分岐で、ブロックの先頭から idiom までに I0 を書かない（`ja` の不成立側、`jbe` の成立側などから N を出す）。
- **検出器の自己検査（実測）**: `forms-scan` の F0（msvc_tables）30＋5、F0x＋F3（`msvc_rejected` の全件）が、fe-lift の統計と**全構成で一致**（`summary.txt` の「detector vs fe-decode mismatches: 0」。Blend2D 49＋30、種ありで 50＋30、dsp 6＋5、縮小 4＋3、QFC・FLEngine 0）。

分類の定義（この調査の用語。決定ではない）:

| 記号 | 定義 |
|---|---|
| **F0** | 現在解決される (c1)/(c2)（`msvc_tables`）。 |
| **F0x** | (c1)/(c2) として認識され N も導けるが、別の条件で拒否（`msvc_rejected` の「N not derived」以外）。参考（依頼の分類の外）。 |
| **F1** | 表の値を**正準の読み**（`mov eV,[B+I*4+T]`）で取り、基底 B を足して jmp するが、**load と `add` の間、または `add` と `jmp` の間に別の命令が挟まる**（`recognize_msvc` の「直前」条件に当たらない）。向きが `add rT,B`（B が jmp レジスタでない。**std**）か、`add B,rV`（基底のレジスタが jmp レジスタ。**swapped**。今回の停止点）か。 |
| **F2** | `add rJ,X` が jmp の直前（またはその少し前）で、**表の値の取り方が正準でない**（`add` する相手が load の基底と別のレジスタ、など）。 |
| **F3** | 正準の (c1)/(c2) が `add`／`jmp` の直前にあり、**N が導けない**（`ja` が無い、`cmp idx,imm` が `ja` の直前にない、など。`msvc_rejected` の「N not derived」）。 |
| **F4** | jmp レジスタが**関数ポインタ**（call の戻り値、ポインタの load、`lea J,[rip+d]`、関数先頭から live-in）。 |
| **F5** | その他。dsp_ippv2 では 16 ビット／8 ビットのオフセット表（`movzx eX,word/byte [B+I*k+T]; lea J,[B+X+D]; jmp J`）が全部。 |
| RC／AN／ST | 別の機構で解決済み（`reg_const`／`reg_table`、FLEngine の `Spec`（決定 0036）、selector stub（決定 0079））。MSVC の形ではないので参考。 |

### 1.2 件数（実測。`res/NAME.summary.txt`、`logs/summaries-all.txt`）

| 形 | Blend2D 種なし | Blend2D 種 58 個 | dsp_ippv2 `--pdata` | dsp_ippv2 縮小 | QFC | FLEngine |
|---|---:|---:|---:|---:|---:|---:|
| `register_jmp_sites`（母集団） | 99 | 100 | 836 | 563 | 5 | 20 |
| **F0** 解決済み (c1)/(c2) | 30 | 30 | 5 | 3 | 0 | 0 |
| F0x 認識・N 導出済みで別条件により拒否 | 9 | 9 | 0 | 0 | 0 | 0 |
| **F1** load／`add`／`jmp` が離れる | **13** | **13** | **2** | **1** | **0** | **0** |
| **F2** `add` はあるが表の値の取り方が正準でない | 3 | 3 | 1 | 0 | 0 | 0 |
| **F3** 正準の形だが N が導けない | 40 | 41 | 6 | 4 | 0 | 0 |
| **F4** 関数ポインタ | 4 | 4 | 115 | 10 | 5 | 17 |
| **F5** その他（dsp はオフセット表） | 0 | 0 | 237 | 75 | 0 | 0 |
| RC（`reg_const`／`reg_table`） | 0 | 0 | 470 | 470 | 0 | 0 |
| AN（FLEngine の `Spec`） | 0 | 0 | 0 | 0 | 0 | 3 |
| 合計 | 99 | 100 | 836 | 563 | 5 | 20 |

- Blend2D 種あり（100）と種なし（99）の差は 1 件（`0x478136e`。F3。`.pdata` の外）だけで、F1 の 13 件は同一（実測。`diff`）。
- dsp_ippv2 縮小（563）に F1 が 1 件（`0x37cd3eb`）、`--pdata` 全体（836）に 2 件（`0x37cd3eb` ＋ `0x37ca30f`）。縮小の F0 は 3 件（`0x37cc80b`、`0x37d9670`、`0x37d986a`）、全体は 5 件（＋`0x37b25bd`、`0x37c69eb`）。
- 依頼文の「dsp の 57 件の tail call 風」に直接対応する数は、この分類では出なかった（**未確認**）。F4（115 件。`jmp rbx` 106 件が大半）と F5（237 件）のどちらにも 57 は現れない。`dsp-ippv2-lift-feasibility.md` の「57」は `.text` 内の 57 個の CPU 振り分けの表の数で、別の概念（読み）。
- QFC と FLEngine は、F0〜F3、F5 が 0 件。**F1 に当たる site は QFC・FLEngine に 0 件**（実測）。

### 1.3 F1 の内訳（実測。`res/b2d0.sites.tsv`、`res/dsp.sites.tsv`）

Blend2D の 13 件、dsp_ippv2 の 2 件（`0x37cd3eb`、`0x37ca30f`）。列: load→`add` の間の命令数、`add`→`jmp` の間の命令数、基底 B の定義、ガード（relaxed／edge）、`resolve_msvc`（無改変の library。基底証明＋表の移動先）に合成した `MsvcForm` を渡した結果。

| グループ | site | 形 | 間に挟まる命令 | ガード（N） | library の結果 |
|---|---|---|---|---|---|
| **g-swap**（今回の停止点と同形。swapped） | Blend2D `0x489f8fb`（`f_489f7d0`）、dsp `0x37cd3eb`（`f_37cd2c0`） | `mov eax,[r9+r8*4+T]; add r9,rax; add rcx,r8; mov rax,r11; jmp r9` | `add`→`jmp` に 2 命令（`add rcx,r8`、`mov rax,r11`） | 呼び元ブロックの `cmp r8,0x10; jbe`（成立側）= **N=0x10**。edge で導ける。`recognize_msvc` の `ja` は無い | 基底が jmp レジスタ自身なので、そのまま渡すと「r9 の `add` の定義が届く」で**拒否**（`add r9,rax` が B の書き込みなので）。表の 17 エントリはすべて関数内の命令の先頭（§3） |
| **g-std-A**（`add` は jmp の直前。load と `add` の間に 1 命令） | Blend2D 8 件（`0x48643d4`、`0x4870269`、`0x487c54f`、`0x48883c5`、`0x488ac45`、`0x488d463`、`0x488fc0f`、`0x48923fd`）、dsp `0x37ca30f` | `cmp ecx,0xc; ja; lea r9,[ImageBase]; movaps xmm0,xmm2; mov ecx,[r9+rcx*4+T]; movaps xmm5,xmm2; add rcx,r9; jmp rcx` | load と `add` の間に `movaps`／`movsd`（SSE の move。レジスタ r9・rcx を変えない）1 命令。**`ja` と表引きの間は `lea`＋`movaps` の 2 命令**（`MSVC_SKIP` = 2 の範囲だが `movaps` は中立な命令でない） | `cmp ecx,0xc; ja`（relaxed と edge の両方）= **N=0xc**。dsp は `cmp r9d,0x8; ja` = N=0x8 | Blend2D の 4 件（`0x48643d4`、`0x4870269`、`0x487c54f`、`0x48883c5`）と dsp の 1 件は **OK**（10 エントリ（N=0xc、13 エントリ中 distinct 10）、9 エントリ。移動先は全部関数内で iced の走査の命令の先頭）。残り 4 件（`0x488ac45`、`0x488d463`、`0x488fc0f`、`0x48923fd`）は **REJECT**（他の間接 jmp に非 `lea` の定義が届く。0094 の基底証明の規則）。つまり、この 9 件（Blend2D 8 ＋ dsp 1）のうち、**形を広げると無改変の library が通すのは 5 件**（Blend2D 4 ＋ dsp 1） |
| **g-std-B**（`add` は load のすぐ後、`jmp` との間に `lea`／`mov`） | Blend2D `0x4729320`（c2。`lea rbx,[..]; lea rdi,[..]` が `add`→`jmp` に 2 命令）、`0x4741499`、`0x4741bdc`（`mov r13d,2`／`xor r12b,r12b` が 1 命令） | `mov ecx,[r13+rax*4+T]; add rcx,r13; lea rbx,[...]; lea rdi,[...]; jmp rcx` | `add`→`jmp` に 1〜2 命令（`lea`／`mov`。rcx・r13 を変えない） | `0x4729320`: `cmp eax,0x24; ja`（+`cdqe`、`lea`）= N=0x24（strict でも relaxed でも導ける。`movzx` と `mov` の間に何もない）。`0x4741499`、`0x4741bdc`: `movsxd rcx,eax; lea r8,[ImageBase]` の前に `lea eax,[rcx-1]` で添字を作る形で、**`cmp; ja` が無い**（N は導けない） | `0x4729320` だけ **OK**（7 エントリ）。他の 2 件は N が導けない |
| **g-other** | Blend2D `0x474bb17`（c2。`add rcx,r14` と `jmp` の間に `mov r15d,8`） | ブロックの先頭（`0x474bafd`。外からの分岐が着地する）から idiom が始まり、`ja` が無い。ガードは `jae`（`bt` の後）で `cmp reg,imm` でない | `add`→`jmp` に 1 命令 | N は導けない。基底 `r14` の定義は `lea r14,[0x48c4198]`（**ImageBase でない**） | 解決しない |

- **まとめ（実測と読み）**: F1 の 13 件（Blend2D）のうち、「基底は正しい `lea ImageBase`、N が導け、表が関数内」の条件を満たし、**無改変の library が現行の基底証明で通すもの**は、g-std-A の 4 件（`0x48643d4`、`0x4870269`、`0x487c54f`、`0x48883c5`）と g-std-B の 1 件（`0x4729320`）の**計 5 件**。g-swap の 1 件（`0x489f8fb`）は N が導け、表が関数内で正しく読めるが、基底が jmp レジスタ自身のため現行の基底証明の形（B を jmp までに書かない）に**当たらない**。dsp は `0x37ca30f`（OK）と `0x37cd3eb`（g-swap）。
- g-swap の 2 件（`f_489f7d0` と `f_37cd2c0`。同形）は、同じ関数に他の `jmp reg` を持たない（Blend2D の `f_489f7d0` は `jmp reg` が 1 つだけ。実測: `0x489f8fb` が `register_jmp_sites` の唯一の site）。g-std-A の Blend2D 8 件は 8 つの関数 `f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100`、`f_4889ef0`、`f_488c8c0`、`f_488ef00`、`f_4891880` にあり、うち前半 4 関数（`f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100`）は 0094 が解決済みの site（`0x48654a6`、`0x48712cf`、`0x487d636`、`0x4889443`）を同じ関数に持ち、後半 4 関数（REJECT の 4 件）は 0094 が拒否した site（`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a`）を同じ関数に持つ。つまり**同じ関数に「形違いの `jmp reg`」と switch が同居する関数**（`msvc_rejected` の理由は他の間接 jmp に届く非 `lea` の定義）。

### 1.4 F2、F3、F4、F5 の内訳（実測）

- **F2（Blend2D 3、dsp 1）**: `lea B,[ImageBase]` を**複数回**書き、load の基底と `add` の相手が別のレジスタ。`mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax`（load の基底が jmp レジスタ自身で、`add` の相手が別の `lea`）が 3 件（`0x47513d7`、`0x4746aa7`、dsp `0x37c08f9`）、`mov ecx,[r10+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; mov r10d,..`（load の基底 r10 と `add` の相手 rax が別の `lea ImageBase`）が 1 件（`0x47195fa`。`cmp eax,0xa1; ja` で N=0xa1 が導ける。`rdx*8`／`byte [r10+rdx*8+T]` の c2 風）。**4 件とも基底 B のレジスタが 2 つある**形。
- **F3（Blend2D 40、種ありで 41。dsp 6、縮小 4）**: 理由は `msvc_rejected` のとおり（Blend2D: 「`ja` が無い」31、「closure が直前を復号していない」7、「`cmp idx,imm` が `ja` の直前でない」2。dsp: 5＋1）。`ja` が idiom の直前に無い形の内訳（strict の直前命令）: `mov` 12、`lea` 8、`jmp`（ブロックの先頭）7、`shr`（`f_48a2060` の 3 つ。`jmp r11`）4、`je`／`jne`／`jb` の分岐 5、`movsxd` 2。**添字が `mov`／`shr`／`movsxd`／`lea` で作られるか、分岐の合流で `cmp; ja` が無い**ものが大半。ただし、ブロックの唯一の入り口の辺が `cmp reg,imm; ja` で、idiom までの添字が**レジスタのコピー**（`mov eax,edx`、`mov eax,r8d` など）だけで作られる形では、コピーをたどって N が導ける（`forms-scan` の `edge_guard`。実測）: Blend2D は F3 40 件のうち **11 件**（`0x4701a97`、`0x4722fcd`、`0x4723038`、`0x4723086`、`0x4723238`、`0x4723436`、`0x4756752`、`0x47596c2`、`0x4771e23`、`0x477fe11`、`0x48a3479`。最後の 1 件は `.pdata` の外）、dsp は 6 件のうち **2 件**（`0x37b5cea`、`0x37b6554`）、縮小は 0 件。11 件のうち 10 件（`.pdata` の外の 1 件を除く）で、無改変の `resolve_msvc` に N を渡すと **OK**（5〜24 エントリ。基底証明と表の移動先は通る）。これらは F1 ではない（`add` は `jmp` の直前）。
- **F4（Blend2D 4、dsp 115、縮小 10、QFC 5、FLEngine 17）**: Blend2D の 4 件は `jmp rdx`（`0x489f418`）、`jmp rax`（`0x48a199a`）、`jmp r10`（`0x48a19ac`）、`jmp rax`（`0x48b5b90`。`_pRawDllMain` の `jmp rax`）で、いずれも関数先頭／直前が `mov` の関数ポインタの tail call。QFC・FLEngine の 22 件は**関数の epilogue の `pop` 列の後の `jmp rax`**で、`rax` は同じヘルパの先頭近くの直接 `call` の戻り値（FLEngine `0x4071bf` を `objdump -d` で確認: `push rcx; lea rcx,[0x1976020]; push r8; push r9; sub rsp,0x68; movapd [rsp+..],xmm0..3; call 0x41cfc0; movapd xmm3..0,[rsp+..]; add rsp,0x68; pop r9; pop r8; pop rcx; pop rdx; jmp rax`。引数レジスタを退避して `call` の戻り値（解決した関数のアドレス）へ tail jump する、**遅延 import のヘルパ**の形。読み。`call 0x41cfc0` の中身は読んでいない）。これらは `.pdata` の外（`in_pdata=false`）で、`forms-scan` の 64 命令の窓が `call` の戻り値まで届いて F4 と分類した（FLEngine 17、QFC 5。22 件とも同じ判定）。dsp の 115 件の内訳（実測。`res/dsp.sites.tsv`）: `jmp rbx` 106 件（16 関数。うち 102 件は 64 命令の窓に `rbx` の書き込みなし、4 件は直前が `lea rbx,[rip+d]`）、`jmp r11` 4 件（`mov r11,[reg+disp]` の関数ポインタの load）、`jmp r10` 2 件、`jmp rax` 2 件、`jmp rdx` 1 件（`jmp r10`／`jmp rax`／`jmp rdx` は関数先頭または直前が `mov` の tail call）。`jmp rbx` の 106 件は、関数全体で `rbx` を書く命令が `lea rbx,[rip+d]`（4〜8 個）と `mov rbx,..`（0〜2 個）で、`call` の後に `jmp rbx` が来る形（1 関数 `f_3a97250` の逆アセンブルで確認。**他の 15 関数は書き込みの種類の集計（`forms-scan` の `far` 列）だけを見ており、逆アセンブルは未確認**）。関数内の別の場所へ戻る「継続渡し」に見える（読み）。
- **F5（dsp 237、縮小 75）**: すべて `jmp rax`。`movzx eax,word ptr [B+I*2+T]`（153 件）または `movzx eax,byte ptr [B+I+T]`（84 件）の後 `lea rax,[B+rax+D]; jmp rax`。**`lea` の `D` は「ImageBase + D = jmp の次の命令の VA」**（アンカーが jmp の直後。16 ビット／8 ビットの**関数内相対**のオフセット表）。ガード（edge）は `cmp r,imm; ja` 119、`jbe` 3、添字を idiom と同じブロックで計算するもの 103（`dec r8d`、`sub r13,r12` など）、2 辺合流 12。**Blend2D には 0 件**（実測）。(c1)/(c2) の「RVA の 4 バイト表」ではない別の表形式。

### 1.5 closure が到達しなかった `jmp reg`（実測。`res/*.sweeponly.tsv`）

| 構成 | 件数 | 形（forms-scan） |
|---|---:|---|
| Blend2D 種なし／種あり | 4 | 4 件とも `jmp rax` で、直前に `add rax,B` と正準の load がある std 隣接形（`0x4745efd`、`0x474c382`、`0x474c5b9`、`0x4752623`）。**closure が到達していない**ので解決対象になっていない（`register_jmp_sites` にも `msvc_rejected` にも出ない）。0094 の調査が「`0x4745efd` は closure 未到達」としたものを含む。他の 3 件（`0x474c382` など）は `ja` が idiom の直前にない形（`prev=Lea`） |
| dsp_ippv2 縮小 | 272 | F4 102、F5 164（8 ビット表 53 ＋ 16 ビット表 111）、隣接の std 形 4、F1 1（std）、F2 1。縮小は種が少ないので closure が届かない関数が多い |
| dsp_ippv2 `--pdata` | 2 | F5 2（16 ビット表） |
| QFC、FLEngine | 0 | — |

## 2. 各形の代表例（VA と逆アセンブル。iced-x86 の Intel 形式。`res/NAME.dis.txt`、`doc/examples-*.txt`）

**F0（現在解決される (c1)。Blend2D `0x48a208b`、`f_48a2060`）**

```
  48a2060: mov rax, rcx
  48a2063: lea r10, [0x4700000]
  48a206a: cmp r8, 0xf
  48a206e: ja 0x48a2180
  48a2074: nop [rax+rax]
  48a2080: mov r9d, [r10+r8*4+0x2fd050]
  48a2088: add r9, r10
  48a208b: jmp r9
```

**F1 g-swap（今回の停止点。Blend2D `0x489f8fb`、`f_489f7d0`。dsp `0x37cd3eb` は同形で基底 0x2000000）**

```
  489f8ce: and r8, 0x7f
  489f8d2: jmp short 0x489f870
  489f8d4: nop [rax+rax]
  489f8e0: mov rdx, r9
  489f8e3: lea r9, [0x4700000]
  489f8ea: mov eax, [r9+r8*4+0x2fd000]
  489f8f2: add r9, rax
  489f8f5: add rcx, r8
  489f8f8: mov rax, r11
  489f8fb: jmp r9
```

**F1 g-std-A（Blend2D `0x48643d4`、`f_4859b70`。同形 8 件）**

```
  48643aa: subsd xmm15, xmm12
  48643af: movaps xmm11, xmm2
  48643b3: cmp ecx, 0xc
  48643b6: ja 0x4865e9a
  48643bc: lea r9, [0x4700000]
  48643c3: movaps xmm0, xmm2
  48643c6: mov ecx, [r9+rcx*4+0x165efc]
  48643ce: movaps xmm5, xmm2
  48643d1: add rcx, r9
  48643d4: jmp rcx
```

**F1 g-std-B（Blend2D `0x4729320`、c2。`add` の後に `lea` が 2 つ）**

```
  47292ec: lea r13, [0x4700000]
  47292f3: cmp eax, 0x24
  47292f6: ja 0x47296da
  47292fc: cdqe
  47292fe: movzx eax, byte ptr [rax+r13+0x297c8]
  4729307: mov ecx, [r13+rax*4+0x297ac]
  472930f: add rcx, r13
  4729312: lea rbx, [0x48c4178]
  4729319: lea rdi, [0x48c4198]
  4729320: jmp rcx
```

**F2（Blend2D `0x47513d7`、`f_474ffa0`。基底のレジスタが load と `add` で別。`lea` を 3 回書く）**

```
  47513ac: lea ecx, [r13-0x8]
  47513b0: lea rax, [0x4700000]
  47513b7: movzx eax, byte ptr [rax+rcx+0x53660]
  47513bf: lea rcx, [0x4700000]
  47513c6: mov ecx, [rcx+rax*4+0x535d4]
  47513cd: lea rax, [0x4700000]
  47513d4: add rcx, rax
  47513d7: jmp rcx
```

（第 2 の F2 の形: Blend2D `0x47195fa`。`mov ecx,[r10+rax*4+T]; lea rax,[0x4700000]; add rcx,rax; mov r10d,0x40000000; jmp rcx`。`cmp eax,0xa1; ja` があり N=0xa1。）

**F3（N が導けない。Blend2D `0x48a22d2`、`f_48a2060` の `jmp r11`。添字は `shr` で作る）**

```
  48a22b8: lea r9, [r8+0x1f]
  48a22bc: and r9, 0xffffffffffffffe0
  48a22c0: mov r11, r9
  48a22c3: shr r11, 0x5
  48a22c7: mov r11d, [r10+r11*4+0x2fd090]
  48a22cf: add r11, r10
  48a22d2: jmp r11
```

（別の F3: Blend2D `0x4723038`。`add r8d,-0x207; cmp r8d,0xf5; ja` の後に `mov eax,r8d; lea r8,[ImageBase]; movzx; mov; add; jmp` で、`ja` と `movzx` の間が `mov`＋`lea` の 2 命令（`mov` が中立でない）。`0x4701a97` は `cmp edx,0xd; ja` の後に `mov eax,edx`（索引のコピー。`MSVC_SKIP` の中立な命令でない）。）

**F4（関数ポインタ。QFC `0x406f8f`、FLEngine `0x4071bf`。`jmp rax`。`rax` は呼び先の戻り値）**

```
  40719d: movapd xmm3, [rsp+0x50]
  4071a3: movapd xmm2, [rsp+0x40]
  4071a9: movapd xmm1, [rsp+0x30]
  4071af: movapd xmm0, [rsp+0x20]
  4071b5: add rsp, 0x68
  4071b9: pop r9
  4071bb: pop r8
  4071bd: pop rcx
  4071be: pop rdx
  4071bf: jmp rax
```

（F4 の別の例: dsp `0x3a9774f`、`f_3a97250`。`call` の後の `jmp rbx`。`rbx` は関数内で `lea rbx,[rip+d]` により関数内のアドレスを入れたもの。）

**F5（dsp_ippv2 `0x2740639`。16 ビットのオフセット表。`lea` のアンカーが jmp の次の命令）**

```
  2740608: neg r11
  274060b: cmp r11, r13
  274060e: jl 0x27407c8
  2740614: cmp r8d, 0x1
  2740618: jb 0x2740889
  274061e: dec r8d
  2740621: lea r11, [0x2000000]
  2740628: movzx eax, word ptr [r11+r8*2+0x1ecf010]
  2740631: lea rax, [r11+rax+0x74063b]
  2740639: jmp rax
```

（F5 の 8 ビット版: dsp `0x279135d`。`movzx eax,byte ptr [r14+r13+0x1ecf760]; lea rax,[r14+rax+0x79135f]; jmp rax`。）

## 3. 停止点 `0x489f8fb`（元の VA `0x18019f8fb`、`f_489f7d0`）の詳細

### 3.1 (a) 命令列（実測。`objdump -D -b binary -mi386:x86-64 -M intel --adjust-vma=0x489f7d0`。`logs/objdump-f489f7d0.txt`、`logs/fn-489f7d0.txt`）

- `.pdata` の RUNTIME_FUNCTION は `[0x19f7d0, 0x19f960)`（RVA。VA `0x489f7d0..0x489f960`、長さ 0x190、ピース 1 つ、チェーンなし）。ファイルオフセット 0x19ebd0。呼び出し元は直接 call が 172 か所（`rdef-idump --xrefs` の `logs/xrefs-b2d.txt` を数えた）。関数全体（全 107 命令）は `logs/objdump-f489f7d0.txt`。要点:

```
489f7d0: mov    r11,rcx
489f7d3: movzx  edx,dl
489f7d6: movabs r9,0x101010101010101
489f7e0: imul   r9,rdx                       ; r9 = dl を 8 バイトに複製
489f7e4: cmp    r8,0x10
489f7e8: jbe    0x489f8e0                    ; r8 <= 0x10 は表へ（符号なし）
489f7ee: movq   xmm0,r9 ; punpcklbw xmm0,xmm0
489f7f7: cmp    r8,0x80 ; ja 0x489f810 ; jmp 0x489f870
 ...    (0x489f810.. : movups/movaps/movntps の 16 バイトずつの書き込みのループ。0x489f870.. 末尾の処理)
489f898: mov rax,r11 ; ret
 ...
489f8e0: mov    rdx,r9                       ; ← jbe の行き先。rdx = 複製した 8 バイトの値
489f8e3: lea    r9,[rip-0x19f8ea]            ; r9 = 0x4700000 = ImageBase
489f8ea: mov    eax,[r9+r8*4+0x2fd000]       ; 表の値 (RVA) を eax へ
489f8f2: add    r9,rax                       ; r9 = ImageBase + 表の値（基底のレジスタ自身に足す）
489f8f5: add    rcx,r8                       ; 間の命令 1
489f8f8: mov    rax,r11                      ; 間の命令 2
489f8fb: jmp    r9
489f8fe: xchg   ax,ax                        ; padding
489f900: mov [rcx-0xf],rdx ; mov [rcx-7],edx ; mov [rcx-3],dx ; mov [rcx-1],dl ; ret    ; ← 表の移動先（サイズ 15 以降）
 ...     (0x489f904.. 0x489f958: サイズごとの末尾の書き込みと ret。0x489f940 は サイズ 11)
```

- 命令の読み（読み）: `rcx` = 書き込み先、`dl` = 埋める値、`r8` = サイズ。0x10 以下のサイズは表で、サイズごとの「末尾から遡る重なった書き込み」（`mov [rcx+r8-0xf],rdx`、`[rcx+r8-7],edx` など。`rcx` は `add rcx,r8` で終端に進めてある）で埋めて `ret`。**memset 風**に見えるのは**読み**（実行して確かめていない）。0094 の調査の読みと一致する。

### 3.2 (b) 表の位置と形（実測。`logs/table-489f8fb.txt`、`logs/check-stop-489f8fb.txt`）

- **T = RVA `0x2fd000`**（VA `0x4700000 + 0x2fd000 = 0x49fd000`）。セクション `_RDATA`（RVA `0x2fd000..0x2fd0fc`、`.pdata` の次）。読み取り専用の**32 ビットの RVA の配列**（4 バイト/エントリ）。**基底は ImageBase**（`lea r9,[rip-0x19f8ea]` = `0x4700000`）。エントリ k は `ImageBase + [T + 4k]`。
- 添字は `r8`（`mov eax,[r9+r8*4+0x2fd000]`）。c1（`movzx` の 2 段の表ではない）。(c1) の表の値の取り方 `mov eT,[B+I*4+T2]` に**当たる**（eT = eax、B = r9、I = r8、T2 = 0x2fd000）。当たらないのは `add` の向き（`add r9,rax`。基底が `add` の第 1 オペランド）と `add`→`jmp` の間の 2 命令。
- 表のエントリ 0..16（17 個）:

| k | RVA | VA | 命令（objdump） |
|---:|---|---|---|
| 0 | `0x19f90e` | `0x489f90e` | `ret` |
| 1 | `0x19f90b` | `0x489f90b` | `mov [rcx-1],dl` |
| 2 | `0x19f937` | `0x489f937` | `mov [rcx-2],dx` |
| 3 | `0x19f907` | `0x489f907` | `mov [rcx-3],dx` |
| 4 | `0x19f914` | `0x489f914` | `mov [rcx-4],edx` |
| 5 | `0x19f924` | `0x489f924` | `mov [rcx-5],edx` |
| 6 | `0x19f934` | `0x489f934` | `mov [rcx-6],edx` |
| 7 | `0x19f904` | `0x489f904` | `mov [rcx-7],edx` |
| 8 | `0x19f93c` | `0x489f93c` | `mov [rax],rdx` |
| 9 | `0x19f918` | `0x489f918` | `mov [rcx-9],rdx` |
| 10 | `0x19f950` | `0x489f950` | `mov [rax],rdx`（`mov [rax+8],dx`） |
| 11 | `0x19f940` | `0x489f940` | `mov [rax],rdx` ← **停止時の `r8 = 11` の移動先** |
| 12 | `0x19f910` | `0x489f910` | `mov [rcx-0xc],rdx` |
| 13 | `0x19f920` | `0x489f920` | `mov [rcx-0xd],rdx` |
| 14 | `0x19f930` | `0x489f930` | `mov [rcx-0xe],rdx` |
| 15 | `0x19f900` | `0x489f900` | `mov [rcx-0xf],rdx` |
| 16 | `0x19f958` | `0x489f958` | `mov [rax],rdx`（`mov [rax+8],rdx`） |

### 3.3 (c) 添字のガード（実測）

- `cmp r8,0x10`（`0x489f7e4`）の直後の `jbe 0x489f8e0`（`0x489f7e8`）が**ガード**で、行き先 `0x489f8e0`（idiom のブロックの先頭。`mov rdx,r9`）に入る辺は、(i) この `jbe` の成立側だけ（`0x489f8e0` へ向かう直接分岐は capstone の線形走査で `0x489f7e8` の 1 本だけ。実測。`logs/check-stop-489f8fb.txt`）、(ii) 直前の `0x489f8d4` は `jmp 0x489f870` の後の `nop`（padding）で、fall-through で入る経路はない（`0x489f8d2: jmp short 0x489f870` で終わる）。したがって **N = 0x10**（`r8` は 0..16、17 エントリ）。
- **`ja` ではなく `jbe` の成立側**で、`cmp` は idiom の直前ではなく**別のブロック**（`0x489f7e4`）にある。`recognize_msvc` は idiom から遡って `ja`（`Mnemonic::Ja`）を探し、`cmp idx,imm` を要求する（`MSVC_SKIP` で `cdqe`、`lea`、padding `nop` を最大 2 つ許す）ので、**この形は N が導けない**（`N not derived`）。さらに、`recognize_msvc` は `add` が `jmp` の直前であることを先に要求するので、この site は `msvc_rejected` にも**出ない**（`Ok(None)`。`stats.b2d0.txt` に `0x489f8fb` は無い。実測）。
- **`jbe` の成立側で N を導くには**: `cmp reg,imm; jbe target` の `target`（ブロックの先頭で、入る辺がその 1 本だけ）に idiom が始まる形、すなわち「辺のガード」の理解が要る（この調査の `forms-scan` の `edge_guard` が `N=0x10` を導いた。決定 0094 の規則（`cmp idx,imm; ja` の fall-through、idiom の直前）の範囲外）。`edge_guard` は idiom のブロックの先頭の直前にある padding `nop` を飛ばして predecessor を数える（`0x489f8e0` の前は `0x489f8d4` の padding `nop` で、その前が `0x489f8d2: jmp`）。

### 3.4 (d) 表の長さの導き方（実測と読み）

- N = 0x10 から **17 エントリ**。`read_msvc_targets` と同じ読み方（c1 は `N + 1` 個）で、17 個の RVA はすべて**互いに異なり**、関数 `[0x19f7d0,0x19f960)` の内側。
- **隣の表との距離の確認（実測）**: 次の表（`f_48a2060` の `0x48a208b` の表。RVA `0x2fd050`、16 エントリ）までの距離は 0x50 バイト = 20 dword。**17 エントリ + ゼロの 3 dword**（`+0x44`、`+0x48`、`+0x4c`。RVA 値 0 = `0x4700000`）。つまり「次の表の開始まで」で数えると 20 個で、17 ではない（ゼロのパディングが 3 つ）。`+0x44` からの値 0 は関数内の命令の先頭ではないので、エントリではない（実測）。**表の長さは N から導くのが妥当で、「次の表との距離」では導けない**（パディングがあるため）（読み）。

### 3.5 (e) 移動先がすべて同じ関数内の命令の先頭か（実測。`logs/check-stop-489f8fb.txt`、`lift/out-forms/py/forms_check_stop.py`）

- 17 個すべてが関数 `[0x489f7d0,0x489f960)` の内側。**objdump の線形デコードの命令の先頭に 17/17、capstone の線形デコードの先頭に 17/17、iced-x86 の走査の先頭に 17/17**（`forms-scan` の `targets` 列。`iced_sweep_starts=17`）。distinct 17/17。
- 生成 C（種なしの Blend2D、HEAD の fe-lift）では、この 17 の VA はラベルになっていない（`grep '^L_489f9..:'` が 17 個とも 0）。`0x489f8fb` は `{ fle_dispatch(R[9]); return; }`（`logs/` ではなく `out/b2d0.c` の 391230 行目）。**解決すれば 17 個がラベルになる**（読み。`0x48a20ff` が閉包のラベルになった前例と同じ）。

### 3.6 (f) この形の件数（実測。§1.2、§1.3）

- **F1（swapped。基底のレジスタが jmp レジスタ自身。今回の停止点と同形）**: Blend2D **1 件**（`0x489f8fb`。種なし・種ありとも）、dsp_ippv2 **1 件**（`0x37cd3eb`、`f_37cd2c0`。`--pdata` と縮小のどちらにも出る。バイト列が同形で、表が `0x26150b0`、基底 0x2000000、N も 0x10）。**QFC 0 件、FLEngine 0 件**。
- **F1 全体（load／`add`／`jmp` が離れる形。std も含む）**: Blend2D **13 件**、dsp `--pdata` 2 件（縮小 1 件）、QFC 0、FLEngine 0。
- **「`add` の向きだけが違う形」を F1 の swapped と数えると、停止点の形は Blend2D に 1 件、dsp_ippv2 に 1 件だけ**（実測）。F0 の 30 件と区別される理由は 2 つ: `add` の向き、および `add`→`jmp` の間の 2 命令。

### 3.7 `f_37cd2c0`（dsp_ippv2 の同形。実測）

- `f_37cd2c0` の `0x37cd3eb: jmp r9` は `lea r9,[0x2000000]; mov eax,[r9+r8*4+0x26150b0]; add r9,rax; add rcx,r8; mov rax,r11; jmp r9`。ガード `cmp r8,0x10; jbe` の成立側、17 エントリはすべて関数内（forms-scan: `N=0x10 count=17 distinct=17 in_function=17 iced_sweep_starts=17`）。**実機での挙動は未確認**（実行していない）。

## 4. 停止点の先の見通し（読み。静的。実行していない）

### 4.1 `f_489f7d0` が何をする関数か（読み）

- 引数は `rcx` = 書き込み先、`dl` = 値、`r8` = サイズ。サイズが 0x10 以下は表（17 通り）、0x11〜0x80 は 16 バイト単位の `movups` のループ、0x80 を超えると `test [rip+..],2`（`0x48f1564` のフラグ）を見て非一時書き込み（`movntps`）へ分岐する形で、戻り値は `r11`（= 元の `rcx`）。**memset 風**（`memset(dst, c, n)` の形）。**推測**: ライブラリの `memset`。**ここは読みで、実行して確かめていない**。
- 呼び出し元は直接 call が **172 か所**（Blend2D 全体。`memset` の呼び出しに多い数）。

### 4.2 ATTACH の直線上にあるか（実測と読み）

- 実測（前回の 3 モジュールの測定。`docs/investigations/blend2d-switch-reaching-defs.md` §6.3、`lift/out-rdefs/three/out-1/a4/*-native.txt`）: C のスタックは `#0 f_489f7d0` ← `#1 f_489d880` ← `#2 f_476ed70` ← `#3 f_4899d90` ← `#4 f_4701000` ← `#5 f_48b5b90` ← `#6 f_48a59a8` ← `#7 f_489e2b4` ← `#8 f_489e264` ← `#9 f_489e450`。**`f_489f7d0` は ATTACH の直線上**（スタックにある）。呼んでいる命令は `f_489d880` の `0x489daa7: call 0x489f7d0`（`r8 = rbx - rdi`、`edx = 0`。`xor edx,edx` で値 0。「文字列バッファの拡張部分を 0 で埋める」形に見える（読み））。
- `f_489d880`（`objdump`: `logs/objdump-f489d880.txt`）は「文字列（`ptr, size, capacity` の 16 バイトヘッダ付きの SSO 風の文字列）を `(ptr, len)` で代入する」形に見える（読み。`f_48a2060` の `memcpy` 風でコピー、`f_4771d40` で確保）。呼び出し元の `f_476ed70` は 11 回これを呼び、渡している文字列は `"BMP"`、`"Blend2D"`、`"image/x-bmp"`、`"bmp|ras"`、`"JPEG"`、`"image/jpeg"`、`"PNG"`、`"png"`、`"image/png"`（実測。`0x48dc2d0..0x48dc358` のバイト）。**画像コーデックの名前表の初期化**に見える（読み）。`f_476ed70` は `f_4899d90`（`.xc` の初期化子）の末尾の `0x489b087: call 0x476ed70`。

### 4.3 この F1 の site を解決すると、ATTACH の次はどこで止まりそうか（読み・静的。推測を含む）

静的に、HEAD の fe-lift が作った**生成 C（種なしの Blend2D、`lift/out-forms/out/b2d0.c`。種ありの 3 モジュールの構成と閉包が少し違うので近似）**を、直接 call と `goto` だけで辿った（`logs/static-reach-b2d0.txt`、`logs/static-reach-sites.txt`。**間接 call・vtable は辿らない**ので、到達の下限）。

| 起点 | 直接 call で届く関数 | その中の `jmp reg` の分類（Blend2D の分類。§1） | 届く `fle_imp_*` |
|---|---:|---|---|
| `f_489d880`（停止点の呼び出し元） | 37 | F0 1（`0x48a208b`。0094 で解決済み）、**F1 1（`0x489f8fb`。停止点）**、F3 3（`f_48a2060` の `jmp r11` 3 つ）、F4 1（`f_48b5b90` の `jmp rax`） | 11 名（`EnterCriticalSection`、`FreeLibrary`、`GetLastError`、`GetProcAddress`、`HeapAlloc`、`HeapFree`、`LeaveCriticalSection`、`LoadLibraryExW`、`SetLastError`、`TlsGetValue`、`TlsSetValue`）。ただし `f_489d880` 自身の呼ぶ先（`f_4771d40`、`f_48a2060`、`f_48a26e0`、`f_489f7d0`）の `malloc`／`free` 周辺 |
| `f_4899d90`（`.xc` の初期化子） | 53 | F0 3、F1 1、F3 3、F4 1 | 16 名 |
| `f_489e450`、`f_4899d90`、`f_4701000`、`f_489e2b4`、`f_489e264`、`f_48a59a8`（スタックの `#9..#3` の関数）の和 | 244 | F0 5、F1 1、**F3 4**（`f_48a2060` の 3 つ、`0x48aefcc`（`f_48aeed0`））、F4 1 | — |

- **F1 の site `0x489f8fb` を解決すると**、`f_489d880` の `0x489daa7` の `memset` 風の呼び出しは戻る（`r8 = 11` は `0x489f940` へ）。その先（`f_489d880` → `f_476ed70` の残りの 10 回の `f_489d880` → `f_4899d90` の `.xc` の残り）で、**直接 call で届く範囲の `jmp reg` に残る未解決は F3 の `f_48a2060` の 3 つの `jmp r11`（`0x48a22d2`、`0x48a2402`、`0x48a2576`。`memcpy` のサイズ 0x20 以上の経路。**N が導けない形**。0094 の範囲外）と、`f_48b5b90` の `jmp rax`（F4。関数ポインタ。`_pRawDllMain` に当たる別の用途）**。
- **推測（実行していない）**: `f_489d880` の 11 回の呼び出しで渡される文字列は `"Blend2D"`（7 文字）などの**短い文字列**（長さ 3〜11）。`f_489d880` の `0x489d94b..0x489d96a` の分岐（`cmp rdi,rbx; ja`、`test r14b,1`）で、長さが 0xe 以下で `r14b` のビット 0 が 0 なら**「短い文字列」のインライン格納**（ヒープを使わず `[rsi]` にコピー）に入り、`f_48a2060` の `memcpy` を小サイズで呼ぶ。そうでなければ `f_4771d40` で確保してから `memcpy`。`f_489f7d0`（`memset`）は `0x489daa7` で**「前の確保より小さいとき、残りを 0 で埋める」**形の呼び出し。この呼び出しの `r8` の値は呼び出しごとに違い（今回は 11）、表の 17 エントリのうち別のエントリへ行く呼び出しが後続にあるかもしれないが、**17 エントリはすべて同じ関数内で全部ラベルになる**ので表の範囲内なら止まらない。
- **この site を解決した後に止まりそうな場所の候補（推測）**: (1) `f_48a2060` の `jmp r11`（F3。`memcpy` のサイズが 0x20 以上のとき。これまでの停止では `r8 = 3` と小さいサイズだけ。`f_476ed70` の文字列は最長 11 文字なので、**この 11 回の呼び出しの `memcpy` は `jmp r9`（表の 16 エントリ）の範囲 = N=15 を超えない**。0x10 以上のサイズは `ja 0x48a2180` の先で、`jmp r11` に届く）。つまり **`f_476ed70` の 11 回の呼び出しでは `jmp r11` に届かない見込み**（読み）。(2) `f_4899d90` の `.xc` の初期化の続き。`f_4899d90` は `GetModuleHandleA`、`GetProcAddress`、`GetSystemInfo` などを既に呼んでいて、それらは**これまでの測定の import の列（183 回）に出ている**。(3) その先は DllMain の戻り（`f_489e584` ほか）と `GetProcAddress("AggCreateInstance")`。**どれも読みで、止まる場所は実行しないと分からない**。静的に確かめられるのは、**この F1 の解決だけでは F3 の 3 つ（`jmp r11`）は解決しない**ことと、**F3 の 3 つに届くかは呼び出しのサイズ次第**ということまで。
- **未確認**: 解決後の ATTACH の次の停止（3 モジュールの再測定は依頼の範囲外。実行していない）。`f_489d880` の 11 回の呼び出し以外に、`f_489f7d0` が ATTACH で何回呼ばれるか（172 か所の呼び出し元のうち、ATTACH の直線上にあるものの数）。`0x48f1564` のフラグ（`test [rip+0x51d4d],2`。0x80 を超えるサイズで見る。CPU 機能フラグと読めるが、値は未確認）。

### 4.4 補足: ATTACH の上位の構造（読み。`logs/objdump-f489e2b4.txt`、`objdump-f489e264.txt`）

- スタックの `#7 f_489e2b4`（`0x19e2b4..0x19e3ca`）は CRT の DllMain 本体（`__DllMainCRTStartup` の内側）に読める。`call 0x48a5a0c`（`rcx = 0x48b7338`、`rdx = 0x48b7360`。`_initterm_e` 風）の後に `call 0x48a59a8`（`rcx = 0x48b7320`、`rdx = 0x48b7330`。`_initterm` 風。`f_48a59a8` は `[rcx..rdx)` の関数ポインタを順に呼ぶ 8 バイトずつのループ）。`#6 f_48a59a8` の `call [0x48b7308]`（`0x48a59e5`）が `.xc` の項目 `f_48b5b90`（`jmp rax`。`_pRawDllMain` に当たる）を呼び、そこから `f_4701000` → `f_4899d90`（`.xc` の初期化子）。**停止時、`f_48a59a8` のループは最初の項目の中にいる**（`#5 f_48b5b90` の guest-return が `0x48a59eb` で `0x48a59e5` の次）。`.xc` の項目は 1 つだけ（`f_4701000`。`blend2d-remaining-imports.md` の調査と同じ）に見えるが、表の範囲 `[0x48b7320, 0x48b7330)` は 16 バイト = 2 エントリ（`rax != 0` のものだけ呼ぶ）。**読みなので、2 つ目の項目は未確認**。
- この関数の先（`f_489e2b4` の続き）は `call 0x489e1c0`、`0x489eb5c`、`0x489e128`（`[rbx]` を見て、あれば `call r9` で `[0x48b7308]` を呼ぶ）、そして `DllMain` の戻り（`mov eax,1`）。戻りのスタックの深さが `f_489e450`、`f_489e264`（`switch (reason)` の 4 分岐。`reason == 1`（ATTACH）は `jmp 0x489e2b4`）。

## 5. 拡張の選択肢の整理（事実の表。決めない）

### 5.1 前提（実測と読み）

- 決定 0089 の表の形: (c1) `mov eT,[B+I*4+T2]; add rT,B; jmp rT`、(c2) `movzx eX,byte [B+I0+T1]; mov eT,[B+rax*4+T2]; add rT,B; jmp rT`。いずれも**直前**が `cmp idx,N; ja`（間に `cdqe`、`lea B,[rip+d]`、padding `nop` が最大 `MSVC_SKIP` = 2）。決定 0094 の基底証明は「jmp に届く B の定義がすべて `lea B,[ImageBase]`」で、B は `add rT,B` の相手のレジスタ。
- **数え上げの基になった静的な値**（§1.3。`res/*.sites.tsv` の `lib_resolve` 列。無改変の `fe_decode::jumptab::resolve_msvc` に、F1 の idiom から合成した `MsvcForm` を渡した結果）。ただし**合成した `MsvcForm` が `recognize_msvc` と同じものを返すか**は、実装を変えずにはテストできないので**この数は見積もり**。`closure` が到達する site かどうか（`msvc_tables` に入るか）、`verify_msvc`／`foreign_entries`／`landings_in_idioms` の通過は**見積もりに入れていない**（`foreign_entries` は §5.2 で別に静的に数えた）。

### 5.2 各選択肢の表

| | (A) 何もしない（`Spec` で手で解決） | (B) `recognize_msvc` を広げる（間に別の命令が挟まってよい） | (C) F1 の swapped を別の認識として加える（`add rT,rX` で rX が表の値、rT が基底） |
|---|---|---|---|
| **新しく解決される site（見積もり）** | 手で書いた site だけ。停止点の `0x489f8fb` 1 件（Blend2D）と、dsp の `0x37cd3eb` 1 件を書けば 2 件。`Spec` は `Kind::Rel32 { base }`（`base` は絶対アドレス）で `table`（関数内）を要求する（`resolve` の `in_func(s, s.table)`）。**停止点の表は `_RDATA`（関数の外）にあるので、現行の `Spec` の検査「table is not inside the annotated function」（`jumptab.rs` の 163〜164 行）で拒否される**（読み。ソース）。`Spec` の流儀のままでは書けず、`Spec` の検査を変えるか、新しい `Kind` が要る | **F1 std の 5 件**（Blend2D `0x4729320`、`0x48643d4`、`0x4870269`、`0x487c54f`、`0x48883c5`。dsp `0x37ca30f` の 1 件。無改変の library の `resolve_msvc` が OK を返した数）。残りの F1 std 4 件（`0x488ac45`、`0x488d463`、`0x488fc0f`、`0x48923fd`）は基底証明（0094 の他の間接 jmp のワイルドカード）で拒否、3 件（`0x4741499`、`0x4741bdc`、`0x474bb17`）は N が導けない。**swapped（停止点）は解決しない**（基底が jmp レジスタ自身で、`add` が B の書き込みなので、0094 の基底証明は「B の定義が `add` で `lea` でない」として拒否する。上の `lib_resolve`） | **停止点 `0x489f8fb`（Blend2D）と `0x37cd3eb`（dsp）の 2 件**（F1 swapped）。N は呼び元ブロックの `cmp r8,0x10; jbe`（成立側）から（edge guard）、17 エントリは全部関数内の命令の先頭（§3）。基底証明は「`lea r9,[ImageBase]` の定義が `mov eax,[r9+r8*4+T]` の読みまで届く」ことを `add` の前の `r9` について行う形になる（現行の `prove_base` は jmp に届く B の定義を見るので、**新しい認識では `add` の直前の B の定義を見る**ことになる）。ただし N の導出が `ja` でなく `jbe` の成立側なので、**(C) だけでも `recognize_msvc` の N の導出（0094 の範囲外）の拡張が要る**（それも含めて停止点の解決には 2 つの拡張が要る） |
| **件数の内訳の根拠** | §3、§1.3 | §1.3（g-std-A の 9 件（Blend2D 8 ＋ dsp 1）のうち 5 件（Blend2D 4 ＋ dsp 1）が OK、g-std-B の Blend2D 1 件が OK。合計 Blend2D 5、dsp 1） | §1.3（g-swap の 2 件） |
| **誤認識の危険** | 人の主張（基底、表、N）が入る。`Spec` の `code`（jmp 命令のバイト）と範囲の検査はあるが、**基底と表の中身の正しさは人の主張**。N が間違っても表の外は `fle_jumptab_miss` で止まる（0089 と同じ）が、基底が違うと**別の表を読む**可能性が残る（0036 の `Spec` は基底を `Kind::Rel32 { base }` で人が書く）。決定 0074 の「種の明示リスト」と違い、種は関数の入口で「どこから辿るか」の主張だが、`Spec` は**実行時の値の主張**（索引、基底）で、性格が違う（読み） | **間の命令がレジスタ rT・B・索引を変えないことの検査が要る**。F1 std の実データでは間の命令は `movaps`／`movsd`（SSE の move。GPR を変えない）、`lea rbx/rdi,[rip+d]`（別の GPR）、`mov r13d,2`、`xor r12b,r12b`（別の GPR）、いずれも GPR の rT・B・I を書かない（実測。`between_writes=false` が F1 の 15 件すべて）。**危険**: 間の命令が rT・B・I のどれかを書く形を「書かない」と誤判定すると誤った表を読む（`iced-x86` の `used_registers` で検査できる。0093 の `is_padding_nop` と同じ流儀）。間に**分岐**（`jcc`、`call`）がある形は通さない前提にすれば、基本ブロックの内側に限られる。`verify_msvc` の「idiom の内側に block の先頭がない」検査（`from`〜`jmp` を逐次デコードして先頭が無いこと）は idiom の長さが伸びても同じ趣旨で保てる。`landings_in_idioms` の窓 `MSVC_IDIOM_MAX`（現状 71 バイト。`cmp`＋`ja` から `jmp` まで。`MSVC_SKIP` × 15 を含む）に対し、F1 の load から `jmp` までの長さは実測で **13〜25 バイト**（最長は `0x4729320` の 25 バイト。停止点 `0x489f8fb` は 17 バイト。この長さは load から jmp の先頭までで、`cmp; ja` から load の間は別）なので、窓は伸ばさなくても収まる見込み（読み。**未確認**。窓は `cmp` の位置からの距離なので、`ja`〜load の間の命令も合わせた長さで確かめる必要がある）。ただし**挟んでよい命令の個数・長さの上限を決めることが未決** | **基底でないレジスタを足す形を、表と読む危険**: `add rT,rX` で rX が表の値、rT が基底という認識は、(1) rT が `lea rT,[ImageBase]` の定義だけが届く、(2) rX が `mov eX,[rT+I*4+T]` の読みで、**同じ rT** を基底に使っている、の 2 つが揃えば、`add` の結果 = ImageBase + 表の値、と確実に言える（読み）。(1) か (2) が欠けると別のレジスタを足す形を表と誤認する（例: `mov eax,[r9+r8*4+T]; add r9,rax` の r9 が ImageBase でなければ、r9 + 表の値は意味のあるアドレスにならない。基底証明で ImageBase の `lea` だけを許すので、(1) で防げる）。実装の誤りの危険は (2) の「同じレジスタ」の検査の漏れ（`mov eax,[r10+r8*4+T]; add r9,rax` を誤認する）で、これは F2 の 3 件が**まさにこの形**（load の基底と `add` の相手が別のレジスタ）なので、**F2 は (C) で誤って通してはいけない**（実測。F2 の 4 件を (C) の検査で拒否できることを確認する必要がある。**未確認**） |
| **実装の大きさ** | `jumptab.rs` の `Spec`／`resolve` の検査（表が関数内）を変えるか、`Kind` を足す。fe-decode の変更は小さいが、`FLENGINE` の定数の流儀（`Spec` を `const` で持つ）に Blend2D と dsp の**測定用の値（基底 0x4700000、0x2000000）が入る**ので、DLL ごと・基底ごとの `Spec` が要る（リベースで基底が変わる。**種 58 個・基底は測定用の値**で、`Spec` は決定にできない）。したがって、種のような「測定用の値」の扱いと合わない（読み） | `recognize_msvc`（約 541〜616 行の「`add` の前が `mov eT,[B+I*4+T]`」）の `prev_insn` の鎖に「rT・B・I を変えない命令」を最大 K 個許す。変更箇所は (1) `add` の前（load と `add` の間）、(2) `jmp` の前（`add` と `jmp` の間）の 2 か所。`MsvcForm` に idiom の命令範囲を足す必要が出る（`from`〜`jmp` を `verify_msvc` が使う）。`MSVC_SKIP`／`MSVC_IDIOM_MAX` の意味が変わる（0093 のテストが `MSVC_SKIP == 2` を固定している: `tests/msvc_switch.rs` 611〜612 行。**テストの期待値の変更**が要る）。**N の導出**は変えない（`ja` の fall-through、idiom の直前）。F1 g-std-A の 9 件は、`ja` と load の間に `lea` + `movaps` が挟まる形で、`movaps` は `MSVC_SKIP` の中立な命令にも入っていないので、**(B) は「`ja`〜load の間」にも拡張が要る**（g-std-A の N は relaxed ガードで導いた） | `recognize_msvc` に第 2 の形（swapped）を足す。`prove_base` は「`add B,rV` の B」ではなく「`add` の前の B」の定義を対象にする（`BaseProof::ReachingDefs` の到達定義の「jmp」を「`add` の位置」にして、jmp の B の定義ではなく `add` の B の定義を見る）。`resolve_msvc` の `jmp_va` を `add` の位置にもう 1 つ渡す。N の導出は edge guard（`jbe` の成立側）の拡張が別に要る（0094 の範囲外）。変更が `recognize_msvc`、`prove_base_reaching_defs`、`resolve_msvc`、`verify_msvc`（`add` の後の 2 命令 `add rcx,r8; mov rax,r11` が B を書かないことの検査）に及ぶ（(B) より大きい） |
| **FLEngine の生成 C（SHA-256 `feb5b908…8080f`）が変わるか** | 変わらない（読み）。FLEngine は `Spec` が人手で 3 か所（`0x5e5eb6`、`0x5e9ae8`、`0x5f1499`。`jmp rcx`）を解決済みで、この追加の `Spec` は FLEngine の site に当たらない | **変わらない見込み**（読み）。FLEngine の F1 は **0 件**（実測。§1.2。FLEngine の `register_jmp_sites` 20 件は AN 3、F4 17 で、MSVC の表の形がない）。ただし**実際に生成して SHA を確かめていない**（規則を変えた版は作っていない。**未確認**） | **変わらない見込み**（読み）。FLEngine の F1 は 0 件（実測）。同じく**SHA は未測定** |
| **決定 0089 の本文との関係** | 0089 の本文は変えない。0036 の `Spec` の流儀（`FLENGINE`）に合う。ただし「(c1)/(c2) 以外は扱わない」（0089 の 2'）に**反しない**（`Spec` は別の機構） | **0089 の 1(c)（表の形）の「直前」の意味を変える**。「`mov ...; add rcx,B; jmp rcx` の連続」を、「間に別の命令が挟まってよい」に広げる。0093（`ja` と表引きの間の `nop`）の延長線上にあるが、0093 は「レジスタ・フラグ・メモリを変えない命令のみ」と書き、**`MSVC_SKIP` の上限を変えない**としている。**(B) は上限を変えることになる**（0093 の「上限を緩める必要が出たら緩めずに報告」に当たる） | **0089 の 2'（(c1)(c2) 以外は扱わない。「基底が別の方法で得られる」ものを含む）**に抵触する。(C) は (c1)/(c2) の表の形（`mov eT,[B+I*4+T]` と `add`）を使い、基底の**得方**（`lea`）は同じだが、`add` の向きが違う別の形。2' が「(c1)(c2) の形」を指すなら (C) は範囲外（読み）。範囲外の形を決めるのは PM |

### 5.3 件数の見積もりのまとめ（実測と読み。重複を除く）

- **(B) だけ**: Blend2D **+5**（解決される MSVC switch は 30 → 35 の見込み。**停止点は解決しない**）、dsp_ippv2 **+1**（5 → 6）。
- **(C) だけ**: Blend2D **+1**（停止点 `0x489f8fb`。**N の導出の拡張も一緒に要る**）、dsp_ippv2 **+1**（`0x37cd3eb`）。
- **(B) と (C) の両方**: Blend2D +6、dsp +2（重複なし）。
- **N の導出の拡張**（0094 の範囲外。別の論点。選択肢 A〜C のどれでもない）: F3 の Blend2D 40 件のうち **10 件**（`.pdata` の中。11 件のうち `.pdata` の外の `0x48a3479` を除く）は、「コピーされた添字をたどって、idiom のブロックへの唯一の辺の `cmp reg,imm; ja`」から N が導け、無改変の `resolve_msvc` に N を渡すと OK（実測の見積もり。§1.4）: `0x4701a97`（`cmp edx,0xd; ja`。`mov eax,edx` を 1 つ挟む）、`0x4722fcd`、`0x4723038`、`0x4723086`、`0x4723238`、`0x4723436`（`cmp r8d,imm; ja`）、`0x4756752`、`0x47596c2`、`0x4771e23`、`0x477fe11`。dsp は 2 件（`0x37b5cea`、`0x37b6554`）。**これらは F1 ではない**（`add` は jmp の直前）。この調査は F3 の拡張を選択肢に入れていない（依頼の範囲外。事実として数だけ記す）。**停止点の解決に要る N の導出（`jbe` の成立側）は、F3 のコピーをたどる拡張とは別の拡張**（同じ「ブロックの唯一の入り口の辺のガード」の考え方だが、条件分岐の向きが違う）。
- **dsp_ippv2 の F5**（16/8 ビットのオフセット表。237 件、縮小 75 件）は (A)(B)(C) のどれでも解決しない。別の表形式（`lea J,[B+X+D]`、アンカーが jmp の直後）で、決定 0089 の 2' の範囲外。

### 5.4 意見（作業者の意見。事実ではない。決めない）

- **意見**: 停止点の 1 件だけを解決するなら、(A) の `Spec` は**停止点の表が関数の外（`_RDATA`）にあるため現状の `Spec` では書けない**ので、どの選択肢も `jumptab.rs` の変更が要る。(C) が最も狙いに合う（停止点の形そのもの）が、**N の導出の拡張（`jbe` の成立側）も要る**ので、未決が 2 つある（`add` の向きと N の導出。PM の決定が要る）。
- **意見**: (B) は 0093 の延長として説明できるが、0093 より広い（0093 は「レジスタ・フラグ・メモリを変えない命令」だけ。(B) は「rT・B・I を変えない命令」で、`movaps` や `lea rbx,[..]` のようにフラグは変えないがレジスタを書く命令を含む）。件数は Blend2D 5（g-std-A の 4 ＋ g-std-B の 1）と dsp 1 で、停止点を動かさない。(B) を先にやっても停止点は残る。
- **意見**: 停止点の次の停止が F3 の `f_48a2060` の `jmp r11`（N が導けない）である**かもしれない**（§4.3）ので、(C) だけでなく、F3 の `cmp` のコピー（`mov eax,edx`）を許す拡張が必要になる可能性がある。これは**推測**（`f_476ed70` の 11 回の呼び出しは小さいサイズなので届かない見込みの読みがある。§4.3）。

## 6. 決定が要る点

1. **停止点の形（F1 swapped）を扱うか**。扱うなら (C) の形（`add B,rV`。基底が jmp レジスタ自身）が 0089 の 2'（(c1)(c2) 以外は扱わない）の範囲内かどうか。**0089 の本文の「基底が別の方法で得られる」は基底の得方の話で、`add` の向きの話ではない**（読み）が、範囲外とみなすかは未決。
2. **N の導出の拡張**（`cmp reg,imm; jbe target` の成立側。target に入る辺がその 1 本だけ）を認めるか。停止点は `ja` の fall-through でなく `jbe` の成立側（§3.3）。0094 の決定 4 は「N の導出の拡張はしない」。
3. **(B)（間に別の命令）を認めるか**。認めるなら `MSVC_SKIP` の意味（0093 は「レジスタ・フラグ・メモリを変えない命令」で上限 2。(B) は rT・B・I を変えない命令に広げ、上限を決める）。F1 の std が Blend2D で 5 件、dsp で 1 件増える見込みだが、停止点は動かない。
4. **(A) の `Spec`** を採るなら、`Spec` の表の検査（`table is not inside the annotated function`）を変えるか。測定用の値（基底、種）を `Spec` に入れるのは決定の流儀に合うか。
5. 3 モジュールの再測定（解決後の ATTACH の次の停止）は、この調査では**していない**。(C) を実装した後に測る。

## 7. 限界

- **実行していない**: 3 モジュール、native、gdb、wasm はいずれも実行していない。停止点の先（§4）は**静的な読み**（生成 C の直接 call と `goto`。間接 call、vtable は辿らない）で、止まる場所は**未確認**。F1 の各 site の表の動作（実際に表どおりに飛ぶか）は未確認（0094 は `0x48a208b`、`0x37cc80b`、`0x470a89e` の 3 site だけ native と生成 C を突き合わせた）。
- **分類は 8 命令の窓**（8 命令を超える離れた形は F4／F5／「live-in」に入る）。F4 の「live-in」は 64 命令の窓で再検査し、dsp の 115 件の内訳（`jmp rbx` 106 等）を出したが、`rbx` の出所の分類は 1 関数（`f_3a97250`）の確認に基づく（残りは同じ形と読んだ。**未確認**）。
- **F1 の件数は「分類器の窓」の定義による**。`forms-scan` は fe-decode の `register_jumps`（closure が到達した `jmp reg`）を列挙する。**closure が到達しない `jmp reg`（§1.5）は分類に入れていない**（別に数えた）。種 58 個の違いは Blend2D で F3 の 1 件（`0x478136e`）だけ。
- **`resolve_msvc` に合成した `MsvcForm` を渡した結果は見積もり**。`recognize_msvc` を広げたとき、実際に closure が同じ `MsvcForm` を作るか、`verify_msvc`／`foreign_entries`／`landings_in_idioms` が通すか、`bad_nodes` が 0 のままかは**未確認**（規則を変えた版を作っていない）。`foreign_entries` の静的な検査では、g-std-A の `f_487e100`（`0x48883c5`）に `jcc 0x487e0f5 → 0x487e10e`（他の関数から関数内部への飛び込み）が 1 件あり、**この site は `foreign_entries` で落ちる**（0094 の検証欄で `0x4889443` について調べられたもの。「`0x487e0f5` は直前の関数の RVA 表のバイトを iced-x86 が `loopne` と読んだ偽の分岐で、閉包は持たないので落ちない」とあるので、実装では落ちない可能性が高い。**未確認**）。したがって (B) の「+5」は静的な上限寄りの見積もりで、実装の結果とずれうる。
- **dsp の「57 件の tail call 風」は特定できなかった**（§1.2）。
- 母集団は Blend2D、dsp_ippv2、QFC、FLEngine の 4 つ。同梱のほかの DLL は見ていない。
- 実機の Windows での挙動は未確認（ホストは Linux）。`f_489f7d0` が memset 風であることは読み（実行していない）。

## 8. 作ったファイル

- 新規（追跡対象）: `docs/investigations/msvc-switch-other-forms.md`（この文書）。
- 新規（`.gitignore` の対象の作業物）: `lift/out-forms/`（`tool/`＝`forms-scan`、`py/`、`res/`、`logs/`、`fn/`、`doc/`、`out/b2d0.c`（生成 C。35 MB）、`src/`（`git archive HEAD` の fe-pe、fe-decode、fe-lift の一部）、`target/`（ビルド）、`tmp/`）。
- **追跡ファイルは変更していない**（`git diff --name-only` が 0 件。`git status --short` は、以前からある未追跡 3 件に、この文書 `docs/investigations/msvc-switch-other-forms.md` が加わるだけ）。`lift/out-forms/` は `.gitignore` の `lift/out*/`（10 行目）に当たるので、`git status --short` には出ない（`git status --short --ignored` では `!! lift/out-forms/`。実測）。
