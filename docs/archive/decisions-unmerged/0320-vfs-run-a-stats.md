### streams
| id | 流れ | 符号化 | chunk | ファイル数 | 論理 MiB | 保存 MiB | 率 | chunk 数 |
|---|---|---|---|---|---|---|---|---|
| 0 | large-audio-compressed | identity | 4 MiB | 130 | 72.6 | 72.6 | 1.000 | 19 |
| 1 | large-bin | brotli | 4 MiB | 245 | 1014.8 | 316.3 | 0.312 | 254 |
| 2 | large-data | brotli | 4 MiB | 278 | 184.8 | 34.9 | 0.189 | 47 |
| 3 | large-image-compressed | identity | 4 MiB | 31 | 31.3 | 31.3 | 1.000 | 8 |
| 4 | large-image-raw | brotli | 4 MiB | 45 | 53.9 | 7.9 | 0.147 | 14 |
| 5 | large-other | brotli | 4 MiB | 36 | 24.1 | 12.2 | 0.506 | 7 |
| 6 | large-pack | brotli | 4 MiB | 31 | 104.7 | 47.9 | 0.458 | 27 |
| 7 | large-pcm | brotli | 4 MiB | 131 | 83.8 | 73.0 | 0.871 | 21 |
| 8 | small-audio-compressed | identity | 1 MiB | 2450 | 108.3 | 108.3 | 1.000 | 109 |
| 9 | small-bin | brotli | 1 MiB | 141 | 8.4 | 2.9 | 0.347 | 9 |
| 10 | small-data | brotli | 1 MiB | 12718 | 114.5 | 27.6 | 0.241 | 115 |
| 11 | small-image-compressed | identity | 1 MiB | 1121 | 47.0 | 47.0 | 1.000 | 48 |
| 12 | small-image-raw | brotli | 1 MiB | 242 | 7.4 | 1.8 | 0.250 | 8 |
| 13 | small-other | brotli | 1 MiB | 5952 | 21.2 | 4.9 | 0.232 | 22 |
| 14 | small-pack | brotli | 1 MiB | 392 | 10.4 | 3.8 | 0.361 | 11 |
| 15 | small-pcm | brotli | 1 MiB | 3879 | 165.9 | 144.6 | 0.872 | 166 |

### kind groups
| 種別の組 | 符号化 | ファイル数 | 論理 MiB | 保存 MiB | 率 | chunk 数 |
|---|---|---|---|---|---|---|
| bin | brotli | 386 | 1023.3 | 319.3 | 0.312 | 263 |
| data | brotli | 12996 | 299.3 | 62.5 | 0.209 | 162 |
| pcm | brotli | 4010 | 249.7 | 217.6 | 0.871 | 187 |
| audio-compressed | identity | 2580 | 180.9 | 180.9 | 1.000 | 128 |
| pack | brotli | 423 | 115.1 | 51.7 | 0.449 | 38 |
| image-compressed | identity | 1152 | 78.4 | 78.4 | 1.000 | 56 |
| image-raw | brotli | 287 | 61.3 | 9.8 | 0.159 | 22 |
| other | brotli | 5988 | 45.2 | 17.1 | 0.378 | 29 |
| **合計** | | 27822 | 2053.3 | 937.2 | 0.4565 | 885 |

small streams: files 26895, chunks 488, logical 483.1 MiB, stored 341.0 MiB; large: files 927, chunks 397, logical 1570.1 MiB, stored 596.2 MiB
{"bytes": {"logical": 2153002077, "stored_all_chunks": 982747629, "stored_unique_objects": 982747629}, "counts": {"chunks_total": 885, "dirs": 1522, "entries": 29344, "files": 27822, "objects_unique": 885, "streams": 16}, "files_lt_4KiB": {"bytes": 10781929, "chunks_touched": 115, "files": 15223, "stored_bytes_of_those_chunks": 62624836}, "files_zero_length": 0, "index": {"br_q11": 215576, "name": "index.9fde21c39f5d.br", "raw": 667223, "sha256_of_index_bin": "9fde21c39f5de145a75d109328acd5c20ae6d0df678bbd1537e4f7c27a625826", "variant_with_file_sha_br_q11": 1055298, "variant_with_file_sha_raw": 1557527}}

### fetch cost of typical directories (distinct chunks touched, stored bytes)
| Data/Patches/Packs/Drums (ModeAudio)/Kicks/ | 349 | 6.9 MiB | 7 | 7.0 MiB |
| Artwork/Themes/ | 39 | 17.1 MiB | 8 | 22.2 MiB |
| System/Languages/ | 15 | 6.0 MiB | 3 | 4.0 MiB |
| Data/Templates/ | 70 | 29.4 MiB | 11 | 6.5 MiB |
| Plugins/Fruity/Effects/Fruity Balance/ | 2 | 3.5 MiB | 3 | 2.5 MiB |
| Plugins/Fruity/Effects/Fruity Filter/ | 2 | 3.6 MiB | 2 | 1.4 MiB |
| Data/Patches/Packs/FLEX/ | 16 | 89.9 MiB | 24 | 42.6 MiB |
| Data/Patches/Plugin presets/Generators/ | 7022 | 134.3 MiB | 99 | 45.7 MiB |
| (現在の seed の 218 ファイル) | 218 | 5.4 MiB | 22 | 19.4 MiB |
files spanning more than one chunk: 773 of 27822
max chunks of one file: 13
stored object size: min 3372 median 1048576 max 4194320
