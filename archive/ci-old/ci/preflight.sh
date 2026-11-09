#!/bin/bash
# ci/preflight.sh - what the runner has, and whether the build fits. The build job runs on the standard GitHub-hosted ubuntu-24.04 (private repository: 2 vCPU, 8 GB RAM, 14 GB SSD:
# https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Measured on this build (2026-11-04, see docs/decisions/0217-ci-deploy.md section 4b): one `emcc -Oz` unit peaks at 1.43 GB
# (2 compile jobs on 2 vCPU: 3 GB), `wasm-opt -Oz` at 5.2 GB (GNU time, 1 thread; it runs alone, after the compiles), the work directory 2 GB, emscripten 1.9 GB, the FL installer 1.1 GB (deleted after
# the extraction), cargo/rustup/npm about 1 GB: about 7 GB of disk at the peak.
#   On a GitHub-hosted runner (GITHUB_ACTIONS=true) it adds a swap file when RAM + swap is below CI_NEED_MEM_MB (default 10240), puts the work directory on /mnt (only when /mnt is another file system than the root disk; on the 2026-11-03 image it is not)
#   when the root disk has less than CI_NEED_DISK_MB free (default 10240), and as a last resort removes preinstalled toolchains this build does not use (.NET, Android, GHC, CodeQL, Swift).
#   It stops with the reason when the numbers are still short. Anywhere else it only prints.
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
NEED_MEM_MB=${CI_NEED_MEM_MB:-10240}; NEED_DISK_MB=${CI_NEED_DISK_MB:-10240}
mem_mb()  { awk '/MemTotal/ {print int($2/1024)}' /proc/meminfo; }
swap_mb() { awk '/SwapTotal/ {print int($2/1024)}' /proc/meminfo; }
free_mb() { df -Pm "$1" 2>/dev/null | awk 'NR==2 {print $4}'; }
other_fs() { [ "$(stat -c %d "$1" 2>/dev/null)" != "$(stat -c %d "$2" 2>/dev/null)" ]; }
on_runner() { [ "${GITHUB_ACTIONS:-}" = true ] && command -v sudo >/dev/null; }
report() {
  echo "ci: nproc $(nproc); RAM $(mem_mb) MB; swap $(swap_mb) MB; load $(cut -d' ' -f1 /proc/loadavg)"
  df -h -x tmpfs -x devtmpfs -x squashfs -x overlay 2>/dev/null | sed 's/^/ci:   /'
  echo "ci:   work directory $CI_WORK: $(free_mb "$CI_WORK") MB free; /mnt is $([ -d /mnt ] && { other_fs /mnt / && echo 'another file system' || echo 'on the root file system'; } || echo 'absent')"
}
if [ "${1:-}" = end ]; then
  # at the end of the build job: what the whole job used (the cgroup of this step and its parents have memory.peak when the controller is on), the disk, and whether the kernel killed anything
  echo "ci: resources now ($(date -u +%H:%M:%SZ)); memory.peak is the peak since that cgroup began: $(cut -d: -f3 /proc/self/cgroup)"
  cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
  for _ in 1 2 3 4; do
    for f in memory.peak memory.swap.peak memory.current memory.max; do [ -r "$cg/$f" ] && echo "ci:   $cg/$f = $(cat "$cg/$f")"; done
    cg=$(dirname "$cg"); [ "$cg" = /sys/fs ] && break
  done
  free -m | sed 's/^/ci:   /'; swapon --show 2>/dev/null | sed 's/^/ci:   /'
  df -h -x tmpfs -x devtmpfs -x squashfs -x overlay 2>/dev/null | sed 's/^/ci:   /'
  du -sm "$CI_WORK" 2>/dev/null | sed 's/^/ci:   work directory MB: /'
  echo "ci:   pressure: $(tr '\n' ' ' < /proc/pressure/memory 2>/dev/null)"
  { sudo -n dmesg 2>/dev/null || dmesg 2>/dev/null; } | grep -i -E 'out of memory|oom-kill|killed process' | tail -5 | sed 's/^/ci:   kernel: /'
  exit 0
fi
echo "ci: preflight (need RAM+swap >= $NEED_MEM_MB MB, free disk for the work directory >= $NEED_DISK_MB MB)"
mkdir -p "$CI_WORK"
report
if on_runner; then
  # 1. memory: a swap file for the wasm-opt peak. The swap goes to /mnt only when that is another file system; on the root disk it eats the room the work directory needs (step 2 sees that).
  have=$(( $(mem_mb) + $(swap_mb) ))
  if [ "$have" -lt "$NEED_MEM_MB" ]; then
    add=$(( NEED_MEM_MB - have )); [ "$add" -lt 4096 ] && add=4096
    where=/; { [ -d /mnt ] && other_fs /mnt / && [ "$(free_mb /mnt)" -gt $(( add + 2048 )) ]; } && where=/mnt
    log "RAM + swap is $have MB: adding a ${add} MB swap file on $where"
    sf="${where%/}/ci-swapfile"; sudo fallocate -l "${add}M" "$sf" && sudo chmod 600 "$sf" && sudo mkswap "$sf" >/dev/null && sudo swapon "$sf" || log "WARNING: the swap file could not be created"
  fi
  # 2. disk: the work directory on /mnt when that is another file system with more room
  if [ "$(free_mb "$CI_WORK")" -lt "$NEED_DISK_MB" ] && [ -d /mnt ] && other_fs /mnt "$CI_WORK" && [ "$(free_mb /mnt)" -gt "$(free_mb "$CI_WORK")" ]; then
    log "root disk has $(free_mb "$CI_WORK") MB free: putting $CI_WORK on /mnt ($(free_mb /mnt) MB free)"
    sudo mkdir -p /mnt/ci-work && sudo chown "$(id -u):$(id -g)" /mnt/ci-work && sudo mount --bind /mnt/ci-work "$CI_WORK" || log "WARNING: the bind mount failed"
  fi
  # 3. disk: remove preinstalled toolchains that this build does not use
  if [ "$(free_mb "$CI_WORK")" -lt "$NEED_DISK_MB" ]; then
    log "still $(free_mb "$CI_WORK") MB free: removing preinstalled .NET, Android, GHC, CodeQL, Swift"
    sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc /usr/local/.ghcup /opt/hostedtoolcache/CodeQL /usr/share/swift 2>/dev/null || true
  fi
  report
fi
bad=""
[ "$(( $(mem_mb) + $(swap_mb) ))" -ge "$NEED_MEM_MB" ] || bad="$bad RAM+swap $(( $(mem_mb) + $(swap_mb) )) MB < $NEED_MEM_MB MB (wasm-opt needs 5.2 GB on top of the system);"
[ "$(free_mb "$CI_WORK")" -ge "$NEED_DISK_MB" ] || bad="$bad free disk $(free_mb "$CI_WORK") MB < $NEED_DISK_MB MB (work directory 2 GB + emscripten 1.9 GB + installer 1.1 GB + cargo/npm ~1 GB at the peak ~7 GB);"
summary "### Runner resources" "" "nproc $(nproc), RAM $(mem_mb) MB, swap $(swap_mb) MB, free disk of the work directory $(free_mb "$CI_WORK") MB. Compile jobs: $(jobs_default)."
if [ -n "$bad" ]; then
  echo "ci: ERROR: this runner is too small for the build:$bad" >&2
  summary "**The runner is too small:**$bad A larger runner would be the owner's decision (see the issue); nothing was changed."
  exit 1
fi
log "preflight ok; compile jobs: $(jobs_default)"
