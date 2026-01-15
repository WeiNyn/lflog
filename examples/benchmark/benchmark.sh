#!/bin/bash

# Configuration
LOG_FILE="Apache_large.log"
CONFIG_FILE="../loghub_demos/config/loghub.toml"
LFLOG_BIN="../../target/release/lflog"

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file $LOG_FILE not found. Running generator..."
    ./generate_data.sh
fi

echo "========================================================"
echo "BENCHMARK: lflog vs awk vs gawk vs mawk"
echo "Dataset: Apache Log ($(du -h $LOG_FILE | cut -f1))"
echo "========================================================"

# Helper function to time a command
time_cmd() {
    local name="$1"
    local cmd="$2"
    
    echo -n "Running $name... "
    
    # Use standard 'time' and capture real time
    # Format: Real time in seconds
    start=$(date +%s.%N)
    eval "$cmd" > /dev/null
    end=$(date +%s.%N)
    
    duration=$(echo "$end - $start" | bc)
    printf "%.3f s\n" "$duration"
}

echo
echo "--- TASK 1: Count Filter (level = 'error') ---"
echo "Query: Count lines where 6th field is '[error]'"
echo

# 1. lflog
# Note: Using optimized regex profile 'apache' from loghub.toml
LFLOG_QUERY="SELECT COUNT(*) FROM log WHERE level = 'error'"
CMD="$LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
time_cmd "lflog" "$CMD"

# 2. mawk (usually fastest awk)
# AWK uses default whitespace splitting. Field 6 is '[error]'
MAWK_CMD="mawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "mawk" "$MAWK_CMD"

# 3. gawk (GNU awk)
GAWK_CMD="gawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "gawk" "$GAWK_CMD"

# 4. awk (System default)
AWK_CMD="awk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
time_cmd "awk " "$AWK_CMD"


echo
echo "--- TASK 2: Aggregation (Group by level) ---"
echo "Query: Count occurrences of each log level"
echo

# 1. lflog
LFLOG_QUERY="SELECT level, COUNT(*) FROM log GROUP BY level ORDER BY count(*)"
CMD="$LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
time_cmd "lflog" "$CMD"

# 2. mawk
MAWK_CMD="mawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "mawk" "$MAWK_CMD"

# 3. gawk
GAWK_CMD="gawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "gawk" "$GAWK_CMD"

# 4. awk
AWK_CMD="awk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
time_cmd "awk " "$AWK_CMD"

echo
echo "========================================================"
echo "Note: "
echo "  - lflog parses the full line using Regex (more robust)."
echo "  - awk/mawk splits by whitespace (faster but brittle)."
echo "========================================================"
