#!/bin/bash
# alpha1: the scenario set on one native build with the recorder. usage: run-all.sh TAG   (work/n1-TAG -> runs/<scenario>-TAG)
set -u
A="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/../../..")"/lift/out-alpha1; T=$1; B=$A/work/n1-$T
cd $A
export A1_BBSIZE="206x279;113x108" A1_BBPIX="30,262;40,97" A1_PIX="34,298;99,133;100,133" A1_CROP="4,36,208,333;59,36,345,231"
bash tools/run-n1.sh $B scripts/pull1.g3 runs/pull1-$T runs/pull1-$T/dump
export A1_BBSIZE="1920x1080" A1_BBPIX="10,10;500,300;960,540;1500,900;1900,1070;300,60" A1_PIX="10,10;500,300;960,540;1500,900;1900,1070;300,60" A1_CROP=""
bash tools/run-n1.sh $B scripts/f8.g3 runs/f8-$T runs/f8-$T/dump
export A1_BBSIZE="" A1_BBPIX="" A1_PIX="" A1_CROP="140,30,240,100"
bash tools/run-n1.sh $B scripts/sel1.g3 runs/sel1-$T runs/sel1-$T/dump
export A1_CROP="470,100,100,60"
bash tools/run-n1.sh $B scripts/dd1.g3 runs/dd1-$T runs/dd1-$T/dump
export A1_CROP=""
bash tools/run-n1.sh $B scripts/cmp-short.g3 runs/cmp-$T
grep -a -E "^g3-(snap|surface|wintable)|^fruity: (== stop|runtime message)" runs/cmp-$T/stdout.txt | cut -c1-240 > runs/cmp-$T/g3-lines.txt
echo done $T
for d in runs/*-$T/; do rm -rf $d/dump $d/wintable; done
