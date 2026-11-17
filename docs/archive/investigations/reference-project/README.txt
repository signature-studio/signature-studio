proj1-results: static analysis of the reference project  ~/ダウンロード/serina1-serum2.zip
=====================================================================================

THIS IS A STATIC ANALYSIS. NO REAL ENGINE (FLEngine_x64.dll) AND NO FL STUDIO WAS RUN.
IT IS NOT A PERFORMANCE MEASUREMENT AND NOT PROGRESS ON THE G LADDER.
Date 2026-11-03, base HEAD 45402a1. Written by an investigation sub-agent for the PM session.

Start with reference-project.md (Japanese; the first section is the summary, the top-3 risks and the Serum 2 answer).

Layout
  reference-project.md   the report (legend: 実測 = counted/read, 読み = interpretation, 未確認, 推測)
  tools/                 own parser and scripts (Python standard library only; run with PYTHONDONTWRITEBYTECODE=1)
    flp_model.py           FLP reader + model (event framing, channels, patterns, playlist, mixer, plugin params)
    flp_report.py          writes the tables from the FLP
    audio_probe.py         header-only probe of wav / mp3 / ogg
    selfcheck.py           17 invariants tying the model to the raw event stream (all OK, see tables/selfcheck.txt)
    budget.py              collects the numbers a performance budget needs (counts only)
  tables/                TSV / JSON outputs
    channels.tsv  plugins.tsv  mixer.tsv  patterns.tsv  automation_clips.tsv
    playlist_items.tsv (3,565 rows)  playlist_tracks.tsv  timeline_intervals.tsv (3,901 intervals)
    audio_files.tsv  event_bytes.tsv  budget_inputs.tsv  requirements.tsv (engine requirements vs state)
    summary.json  selfcheck.txt

NOT included (the client's data): the FLP and the 48 audio files. They were unpacked to lift/out-proj1/x/ (62.7 MB, not for git).

Reproduce (from the repository root):
  mkdir -p lift/out-proj1 && unzip -q ~/ダウンロード/serina1-serum2.zip -d lift/out-proj1/x
  R=lift/out-git/proj1-results; X=lift/out-proj1/x
  PYTHONDONTWRITEBYTECODE=1 python3 $R/tools/flp_report.py $X/serina1-serum2.flp $X $R/tables ~/ダウンロード/FL_STUDIO_.Plugins.ini docs/investigations/g8-project-load/plugins.tsv
  PYTHONDONTWRITEBYTECODE=1 python3 $R/tools/audio_probe.py $X $R/tables/audio_files.tsv $R/tables/channels.tsv
  PYTHONDONTWRITEBYTECODE=1 python3 $R/tools/budget.py $R/tables
  PYTHONDONTWRITEBYTECODE=1 python3 $R/tools/selfcheck.py $X/serina1-serum2.flp
(the engine-side reading in reference-project.md section 3.2 / 4.3 uses docs/investigations/goal-ladder/fle_index.py and
 docs/investigations/g8-project-load/fle_query.py, fle_disasm.py on binaries/win64/FLEngine_x64.dll)

One empirical finding worth knowing: in FL 25.2 / 26.1 files event id 172 has a 3-byte value (the generic 4-byte rule desynchronises
and only re-synchronises by luck). The engine's own reader was not checked (未確認).
