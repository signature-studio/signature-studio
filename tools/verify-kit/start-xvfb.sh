#!/bin/bash
# start-xvfb.sh: the X server for Firefox (run it in the background with exec). Without DISPLAY=:77 Firefox has no WebGL (decision 0271).
exec Xvfb :77 -screen 0 2560x1600x24 -nolisten tcp
