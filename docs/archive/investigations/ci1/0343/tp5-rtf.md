## RT-F: File clicks n=12, UI worker tid 2975166, other Workers 7
- click 1: C span 61.7 ms; (program) total 15.7 ms in 23 runs, runs >= 1 ms: @7.3+1.3, @33.7+1.5, @51.1+1.2, @54.5+1.8, @56.8+1.0
    2 ms bins [P=(program) w=wasm i=idle j=js]: PwwPwwwwPwwwwwwwwwwwwwwPwPwPPjw
- click 2: C span 57.3 ms; (program) total 9.9 ms in 12 runs, runs >= 1 ms: @4.3+1.7, @21.1+2.6
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwPwwwwwwPPPwwwPwwwwwwPwwwwwP
- click 3: C span 61.4 ms; (program) total 17.0 ms in 25 runs, runs >= 1 ms: @36.8+1.8, @41.0+1.2, @43.1+1.5, @47.1+1.3, @53.8+1.1
    2 ms bins [P=(program) w=wasm i=idle j=js]: jwwwwwwwwwwwwwwwwwPwPPwPwwwPwww
- click 4: C span 44.8 ms; (program) total 12.3 ms in 20 runs, runs >= 1 ms: @16.3+1.1, @17.3+1.2, @26.7+1.4
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwwPwwPwPwPwPwwwww
- click 5: C span 48.9 ms; (program) total 10.7 ms in 20 runs, runs >= 1 ms: @35.6+2.1, @40.4+1.1
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwPwwwwwwwwwwPwwPwPww
- click 6: C span 44.9 ms; (program) total 7.9 ms in 14 runs, runs >= 1 ms: @16.8+1.5, @39.1+1.1
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwwPwwwwwwwwwwPwww
- click 7: C span 46.2 ms; (program) total 11.2 ms in 21 runs, runs >= 1 ms: @36.3+1.4, @40.3+1.1
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwPwwwwwwwwwwwwwwPwPwwj
- click 8: C span 37.0 ms; (program) total 2.3 ms in 6 runs, runs >= 1 ms: 
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwwwwwwwwwwwww
- click 9: C span 36.8 ms; (program) total 6.8 ms in 12 runs, runs >= 1 ms: @26.7+1.2
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwwwwPwwPwwwww
- click 10: C span 48.4 ms; (program) total 11.3 ms in 18 runs, runs >= 1 ms: @14.4+1.3, @33.8+1.5
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwPPPwwwwwwPwwwwwwwj
- click 11: C span 39.3 ms; (program) total 7.8 ms in 12 runs, runs >= 1 ms: @1.7+1.2, @3.7+1.2, @9.6+1.5
    2 ms bins [P=(program) w=wasm i=idle j=js]: wPwwwPwwwwwwwwPwwwwP
- click 12: C span 47.5 ms; (program) total 9.4 ms in 13 runs, runs >= 1 ms: @16.2+1.0, @17.9+1.2, @35.5+2.9
    2 ms bins [P=(program) w=wasm i=idle j=js]: wwwwwwwwPwwwwwwwwwPwwwww

### (program) runs >= 1 ms on the UI worker in phase C: 30 runs over 12 clicks; length median 1.3 ms, max 2.9 ms, total 3.6 ms per click
- other Workers during these runs, mean share wasm: 15%
- other Workers during these runs, mean share program: 85%
- other Workers during these runs, mean share idle: 0%
- other Workers during these runs, mean share js: 0%
