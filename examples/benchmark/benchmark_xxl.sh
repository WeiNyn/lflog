#!/bin/bash

# Configuration
LOG_FILE="Apache_10M.log"
CONFIG_FILE="../loghub_demos/config/loghub.toml"
LFLOG_BIN="../../target/release/lflog"
ITERATIONS=5

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file $LOG_FILE not found. Running generator..."
    ./generate_xxl.sh
fi

echo "========================================================"
echo "XXL BENCHMARK: lflog vs awk vs gawk vs mawk"
echo "Dataset: Apache Log (10 Million Lines)"
echo "Size: $(du -h $LOG_FILE | cut -f1)"
echo "Iterations per tool: $ITERATIONS (taking average)"
echo "========================================================"

# Helper function to time a command
run_benchmark() {
    local name="$1"
    local cmd="$2"
    
    echo -n "Running $name... "
    
    total_duration=0
    
    for ((i=1; i<=ITERATIONS; i++)); do
        start=$(date +%s.%N)
        eval "$cmd" > /dev/null
        end=$(date +%s.%N)
        
        duration=$(echo "$end - $start" | bc)
        total_duration=$(echo "$total_duration + $duration" | bc)
        echo -n "."
    done
    
    avg_duration=$(echo "scale=3; $total_duration / $ITERATIONS" | bc)
    printf " Avg: %.3f s\n" "$avg_duration"
}

echo
echo "--- TASK 1: Count Filter (level = 'error') ---"

# 1. lflog (Optimal Threads=8)
LFLOG_QUERY="SELECT COUNT(*) FROM log WHERE level = 'error'"
CMD="LFLOGTHREADS=8 $LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
run_benchmark "lflog (8t)" "$CMD"

# 2. mawk
MAWK_CMD="mawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
run_benchmark "mawk      " "$MAWK_CMD"

# 3. gawk
GAWK_CMD="gawk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
run_benchmark "gawk      " "$GAWK_CMD"

# 4. awk
AWK_CMD="awk '\$6 == \"[error]\" {n++} END {print n}' \"$LOG_FILE\""
run_benchmark "awk       " "$AWK_CMD"


echo
echo "--- TASK 2: Aggregation (Group by level) ---"

# 1. lflog
LFLOG_QUERY="SELECT level, COUNT(*) FROM log GROUP BY level ORDER BY count(*)"
CMD="LFLOGTHREADS=8 $LFLOG_BIN \"$LOG_FILE\" --config \"$CONFIG_FILE\" --profile apache --query \"$LFLOG_QUERY\""
run_benchmark "lflog (8t)" "$CMD"

# 2. mawk
MAWK_CMD="mawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
run_benchmark "mawk      " "$MAWK_CMD"

# 3. gawk
GAWK_CMD="gawk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
run_benchmark "gawk      " "$GAWK_CMD"

# 4. awk
AWK_CMD="awk '{a[\$6]++} END {for (k in a) print k, a[k]}' \"$LOG_FILE\""
run_benchmark "awk       " "$AWK_CMD"
