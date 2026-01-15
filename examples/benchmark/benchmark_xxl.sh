#!/bin/bash

# Configuration
LOG_FILE="Apache_10M.log"
CONFIG_FILE="../loghub_demos/config/loghub.toml"
LFLOG_BIN="../../target/release/lflog"

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file $LOG_FILE not found. Running generator..."
    ./generate_xxl.sh
fi

echo "========================================================"
echo "XXL BENCHMARK: lflog vs awk vs gawk vs mawk"
echo "Dataset: Apache Log (10 Million Lines)"
echo "Size: $(du -h $LOG_FILE | cut -f1)"
echo "========================================================"

# Helper function to time a command
time_cmd() {
    local name="$1"
    local cmd="$2"
    
    echo -n "Running $name... "
    
    # Use standard 'time' and capture real time
    start=$(date +%s.%N)
    eval "$cmd" > /dev/null
    end=$(date +%s.%N)
    
    duration=$(echo "$end - $start" | bc)
    printf "%.3f s\n" "$duration"
}

echo
echo "--- TASK 1: Count Filter (level = 'error') ---"
LFLOG_QUERY="SELECT COUNT(*) FROM log WHERE level = 'error'"
CMD="$LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
time_cmd "lflog" "$CMD"

MAWK_CMD="mawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "mawk " "$MAWK_CMD"

GAWK_CMD="gawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "gawk " "$GAWK_CMD"

# Skip standard awk if it's just gawk/mawk symlink, but running it for completeness
AWK_CMD="awk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "awk  " "$AWK_CMD"


echo
echo "--- TASK 2: Aggregation (Group by level) ---"
LFLOG_QUERY="SELECT level, COUNT(*) FROM log GROUP BY level ORDER BY count(*)"
CMD="$LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
time_cmd "lflog" "$CMD"

MAWK_CMD="mawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "mawk " "$MAWK_CMD"

GAWK_CMD="gawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "gawk " "$GAWK_CMD"

AWK_CMD="awk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "awk  " "$AWK_CMD"
