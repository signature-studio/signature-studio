| variant | f_b34aca0 ms | f_b34eaf0 ms | sum | mean sum |
|---|---|---|---|---|
| default (3 runs) | [1571, 1439, 1354] | [1278, 1196, 1172] | [2849, 2635, 2526] | 2670 |
| branch hints (3 runs, --experimental-wasm-branch-hinting) | [1352, 1177, 1223] | [1075, 1007, 1025] | [2427, 2184, 2248] | 2286 |
| locals recoloured (2 runs) | [1415, 1368] | [1161, 1190] | [2576, 2558] | 2567 |
| window checks removed (1 run, output not x86-exact) | [422] | [301] | [723] | 723 |
| Liftoff only (1 run) | [2808] | [2373] | [5181] | 5181 |
