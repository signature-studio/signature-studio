'use strict';
// The layout of the SharedArrayBuffer ring between the engine's Worker (producer) and the AudioWorklet (consumer), decision 0139 (PROVISIONAL). Version 1. Plain script (no module syntax): it is loaded by importScripts in the
// Worker, by a <script> in the page and by addModule in the AudioWorkletGlobalScope, where it defines the global FLE_AUDIO_SAB. All counters are Int32 words read and written with Atomics; frame counters are modulo 2^32.
//   words (Int32 index):  0 RD  frames read by the worklet since the ring was made (advances 128 per quantum while STATE == PLAYING, data or silence: it is the output timeline)
//                         1 WR  frames published by the engine (the next free frame); a frame is published when the C device hands it over (fle_audio_ring_append)
//                         2 STATE 0 closed, 1 open (a device exists, nothing plays), 2 playing      3 RATE (Hz)      4 CAP (frames, a power of two)      5 MAGIC 0x52414C46 ("FLAR")      6 VERSION
//                         8 Q_TOTAL quanta processed    9 Q_PLAYING quanta processed while STATE == PLAYING    10 UR_QUANTA quanta with missing frames while playing    11 UR_FRAMES missing frames
//                        12 FILL_MIN (frames, while playing)   13 FILL_MAX   14 PUBLISHES (calls of publish)   15 PUBLISHED_FRAMES   16 SKIPPED_FRAMES (frames the engine could not hand over in time: they were played as silence)
//   data: Float32, interleaved L R, CAP frames, from byte offset FLE_AUDIO_DATA_OFFSET.
var FLE_AUDIO_SAB = { RD: 0, WR: 1, STATE: 2, RATE: 3, CAP: 4, MAGIC: 5, VERSION: 6, Q_TOTAL: 8, Q_PLAYING: 9, UR_QUANTA: 10, UR_FRAMES: 11, FILL_MIN: 12, FILL_MAX: 13, PUBLISHES: 14, PUBLISHED_FRAMES: 15, SKIPPED_FRAMES: 16,
  WORDS: 32, DATA_OFFSET: 256, MAGIC_VALUE: 0x52414C46, VERSION_VALUE: 1,
  bytes: function (capFrames) { return 256 + capFrames * 2 * 4; },
  make: function (capFrames, rate) {
    var sab = new SharedArrayBuffer(FLE_AUDIO_SAB.bytes(capFrames)), h = new Int32Array(sab, 0, 32);
    h[FLE_AUDIO_SAB.CAP] = capFrames; h[FLE_AUDIO_SAB.RATE] = rate; h[FLE_AUDIO_SAB.MAGIC] = FLE_AUDIO_SAB.MAGIC_VALUE; h[FLE_AUDIO_SAB.VERSION] = FLE_AUDIO_SAB.VERSION_VALUE; h[FLE_AUDIO_SAB.FILL_MIN] = 0x7fffffff;
    return sab;
  } };
