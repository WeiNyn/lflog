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
echo "PARALLEL SCALING BENCHMARK"
echo "Dataset: Apache Log (10 Million Lines)"
echo "Size: $(du -h $LOG_FILE | cut -f1)"
echo "========================================================"

# Warmup
echo "Warming up page cache..."
cat "$LOG_FILE" > /dev/null
echo "Warmup done."

# Helper function to time a command
time_cmd() {
    local threads="$1"
    local query="$2"
    local name="$3"
    
    echo -n "Threads: $threads | Task: $name... "
    
    # Use standard 'time' and capture real time
    start=$(date +%s.%N)
    
    LFLOGTHREADS=$threads $LFLOG_BIN "$LOG_FILE" \
        --config "$CONFIG_FILE" \
        --profile apache \
        --query "$query" > /dev/null
        
    end=$(date +%s.%N)
    
    duration=$(echo "$end - $start" | bc)
    printf "%.3f s\n" "$duration"
}

# Task 1: Count Filter
QUERY1="SELECT COUNT(*) FROM log WHERE level = 'error'"

# Task 2: Aggregation
QUERY2="SELECT level, COUNT(*) FROM log GROUP BY level ORDER BY count(*)"

echo
echo "--- TASK 1: Count Filter ---"
for t in 2 4 8 16; do
    time_cmd "$t" "$QUERY1" "Filter"
done

echo
echo "--- TASK 2: Aggregation ---"
for t in 2 4 8 16; do
    time_cmd "$t" "$QUERY2" "Group By"
done
