# 外部入力と取得手順

冒頭: (1) 下の sha256 は旧作業ツリーでの計算と、2026-11-10 の再計算([実測: `sha256sum -c`])。(2) FL の DLL・インストーラ・bundle は git に入れない(公開される。`.gitignore` 済み)。(3) ホスト名・アカウント・トークンはここに書かない(R2 の資源名は `docs/ops/environment.md`)。

## 1 入力の一覧(正本は `artifacts.sha256`〔リポジトリ直下。2 列 10 行: bundle 9 + `binaries/win64/FLEngine_x64.dll` 1〕と `ci/external-inputs.sha256`〔6 行。パスは `~/ダウンロード/` からの相対名〕)

| 入力 | 置き場 | サイズ B | sha256 先頭 | 区分 |
|---|---|---:|---|---|
| FL インストーラ 5653 | `~/ダウンロード/flstudio_win64_26.1.7.5653.exe`(依頼者が配置。外部) | 1,106,033,456 | `04393e79…` | サイズ [実測: `stat`]。sha256 は旧記録と一致 [文書] |
| FL インストーラ 5654 | `~/ダウンロード/flstudio_win64_26.1.7.5654.exe` | 1,106,029,776 | `6ddf9f23…` | サイズ [実測]。旧 lock に無い別版。基準プロジェクトは 5654 で作られた [文書] |
| Serum 2 / OTT / Vital | `~/ダウンロード/Install_Xfer_Serum2_2.1.5.exe` / `Install_Xfer_OTT_137.exe` / `VitalInstaller.exe` | 1,313,084,056 / 8,397,928 / 41,808,960 | `507b726d…` / `d4e1d3f6…` / `5bf1b8c7…` | サイズ [実測]。OTT・Vital の先頭 16 桁は旧記録と一致 [実測: `sha256sum`]。Serum 2 は未再計算 |
| 基準プロジェクト | `~/ダウンロード/serina1-serum2.zip`(P2/P3 の入力) | 53,489,451 | `25e8aa91…` | [実測]。zip は 49 ファイル、音声 48(wav 43・mp3 4・ogg 1)・FLP 1 |
| `FLEngine_x64.dll` | `binaries/win64/`(git 外) | 22,227,016 | `b2c64da4…9496d` | [実測: 提案ファイルの 10 行を `sha256sum -c` が全て OK、2026-11-10。DLL の中身は解析していない] |
| E13 bundle(9 ファイル) | `artifacts/bundles/fl-engine-ci-0363b050/`(git 外) | 計約 260 MiB | wasm `58ad4a6a…ccbf`(143,087,706 B)、glue `07e7646f…d075`、seed pack `f7726113…8984`、DLL pack `2f494419…7d39` | [実測: 同上、9 行 OK]。**実行は未確認**(R0 の前) |

## 2 照合(毎回、最初に)

```bash
cd ~/work/signature-studio && sha256sum -c artifacts.sha256        # 全行 OK(artifacts/・binaries/ の 10 行)
(cd ~/ダウンロード && sha256sum -c ~/work/signature-studio/ci/external-inputs.sha256)   # 全行 OK(外部入力 6 行)
```
2 つとも 2026-11-10 に実行し、全行 OK([実測]。bundle・DLL の実行と DLL の中身の解析はしていない)。大きいので `background: true` で実行する(外部入力だけで約 3.6 GB を読む)。
不一致なら**判断で押し通さず**、`docs/BLOCKERS.md` にも `STATE.md` にも書かず、worker にコマンド出力をそのまま報告させる。

## 3 DLL の抽出(インストーラ → 同梱 DLL 8 本)

| 手順 | コマンド | 確かめ方 |
|---|---|---|
| 1 | `python3 tools/extract_nsis2.py <5653.exe> --hash FLEngine_x64.dll` | `22227016 b2c64da4…9496d` |
| 2 | `python3 ci/lib/extract_fl.py <5653.exe> artifacts/host --dlls ci/extract-dlls.tsv --seed lift/harness/seeds/vfs-seed-idle1.tsv` | 各ファイルが `extract-dlls.tsv` の sha256 と一致(不一致は exit 2) |
| 3 | 抽出先は **`artifacts/host/` の 1 か所に統一**(旧 `extract-dlls.tsv` は 6 か所の `lift/out-*` を指し、`oggio`・`elastique` の置き場は旧木に無い [実測])。この書き換えは R0-4 の入力 | `ls artifacts/host` |

対象 DLL: `FLEngine_x64`・`dsp_ippv2_x64`・`Blend2D`・`QuickFontCache_x64`・`freetype_x64`・`ILTools_x64`・`oggio_x64`・`elastique_x64`。`ci/artifacts.lock` はインストーラ 4 本の size・sha256・ETag(旧 R2 前提のヘッダは書き直す)。

## 4 ビルド入力(上流 Blend2D)

`lift/harness/b2d1/build_upstream.sh` が GitHub の固定コミットを取得し、tarball の sha256 を照合して no-JIT で作る(`TARGET=wasm OPT=-O2`、`EMCC_CFLAGS="-matomics -mbulk-memory"`、パッチ `b2d3-compop.patch`)。出力の `build-wasm/libblend2d.a` と `src/src` を `B2D_UPSTREAM_LIB`・`B2D_UPSTREAM_SRC` に渡す(**必須**。無いと `build-engine-bundle.sh` が停止)。ネットワーク到達は [実測: R1 で npm・GitHub が HTTP 200]。

## 5 置き場の規則

- `artifacts/`・`binaries/`・`artifacts/host/`・`artifacts/work/` は `.gitignore`。ビルド中間は数 GiB〜(旧 `lift/out-hyb1` 1 つで 18 GiB)。**作業場所は `/` 側**(`source scripts/env.sh`。`/tmp` は tmpfs 3.6 GiB)。作ったら掃除する(`ls -la` の後、自分が作った物だけ)。
- 旧 R2 の `fl-web-assets` の bundle は取得手順に含めない(旧本番。既定の入口の差し替えは ROADMAP R6)。
- 旧作業ツリーから複写する物は `docs/ops/lessons.md` の場所から、読み取りで 1 回だけ。複写後は `sha256sum -c`。
