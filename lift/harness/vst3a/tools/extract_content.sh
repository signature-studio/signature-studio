#!/bin/bash
# Decision 0165 (vst3a): the part of Serum 2's content (1.41 GB in the installer) that the five measured states reference, extracted with 7z (NSIS-3 installer, per-file deflate) into OUTDIR (105 MB).
# usage: extract_content.sh ~/ダウンロード/Install_Xfer_Serum2_2.1.5.exe OUTDIR
set -e
7z x "$1" -o"$2" -y 'Tables/S2 Tables/Default Shapes.wav' 'Tables/Analog/BSOD_Square.wav' 'Tables/Analog/Basic Shapes.wav' 'Tables/Analog/DS Saw and Tri.wav' \
   'Samples/Factory Non-Tonal/Noises/Organics/AC hum1.wav' 'Samples/Factory Non-Tonal/Noises/Analog/BrightWhite.wav' 'Samples/Factory Non-Tonal/Noises/Analog/SID noise.wav' \
   'Samples/Factory Non-Tonal/Noises/Attacks_Misc/icon_kick a.wav' '-ir!Multisamples/Factory/Keys/Baby Grand Samples/*'
# 'Tables/Adventure Kid/raw.wav' (state 5) is not in the installer (a user table); that state loads without it.
