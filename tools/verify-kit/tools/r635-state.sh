#!/usr/bin/env bash
# R6-35: the machine state read before a run (read only; no argument). Prints the 1-minute load, free -m, vmstat 1 3, the I/O pressure, and the browser processes (--remote-debugging in the command line and no --type=, i.e. one per automated browser) with their pid and age. The command lines themselves are not printed.
cat /proc/loadavg
free -m
vmstat 1 3
head -1 /proc/pressure/io
n=0
for p in /proc/[0-9]*; do
  c=$(tr '\0' ' ' 2>/dev/null < "$p/cmdline") || continue
  case "$c" in *--remote-debugging*) ;; *) continue ;; esac
  case "$c" in *--type=*) continue ;; esac
  n=$((n + 1))
  echo "browser pid=${p#/proc/} age=$(ps -o etime= -p "${p#/proc/}" | tr -d ' ') comm=$(cat "$p/comm" 2>/dev/null)"
done
echo "browser processes (not --type=): $n"
