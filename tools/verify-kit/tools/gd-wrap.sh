#!/bin/bash
exec /snap/bin/geckodriver "$@" --profile-root "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/ffprof"
