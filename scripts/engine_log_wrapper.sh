#!/usr/bin/env bash
# Wraps an engine and logs all UCI I/O.
# Usage: engine_log_wrapper.sh <log_prefix> <engine_exe>
# Produces <log_prefix>.in.log, <log_prefix>.out.log, <log_prefix>.err.log
LOGPREFIX="$1"; shift
tee "${LOGPREFIX}.in.log" | "$@" 2>"${LOGPREFIX}.err.log" | tee "${LOGPREFIX}.out.log"
