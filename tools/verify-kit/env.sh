# the kit's environment (source it). Firefox runs need DISPLAY=:77 (an Xvfb: without it Firefox has no WebGL and the page draws with the 2D presenter; decision 0271)
K=/home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit
REPO=/home/ubuntu/work/signature-studio-develop
export DISPLAY=:77 TMPDIR=$K/tmp FF_PROFILE_ROOT=$K/ffprof GECKODRIVER=$K/tools/gd-wrap.sh APP_WORK=$K/appwork KIT_VITE_CACHE=$K/vite-cache
NEWPORT=${NEWPORT:-5040}; PREVPORT=${PREVPORT:-5041}
mkdir -p $K/tmp $K/ffprof $K/appwork $K/bundles $K/out
io() { head -n 1 /proc/pressure/io | cut -d' ' -f2-4; }
