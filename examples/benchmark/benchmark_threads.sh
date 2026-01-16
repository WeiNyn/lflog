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
echo "PARALLEL SCALING BENCHMARK"
echo "Dataset: Apache Log (10 Million Lines)"
echo "Size: $(du -h $LOG_FILE | cut -f1)"
echo "Iterations per config: $ITERATIONS (taking average)"
echo "========================================================"

# Warmup
echo "Warming up page cache..."
cat "$LOG_FILE" > /dev/null
echo "Warmup done."

# Helper function to time a command
run_benchmark() {
    local threads="$1"
    local query="$2"
    local task_name="$3"
    
    echo -n "Threads: $threads | Task: $task_name"
    
    total_duration=0
    
    for ((i=1; i<=ITERATIONS; i++)); do
        start=$(date +%s.%N)
        
        LFLOGTHREADS=$threads $LFLOG_BIN "$LOG_FILE" \
            --config "$CONFIG_FILE" \
            --profile apache \
            --query "$query" > /dev/null
            
        end=$(date +%s.%N)
        duration=$(echo "$end - $start" | bc)
        total_duration=$(echo "$total_duration + $duration" | bc)
        echo -n "."
    done
    
    avg_duration=$(echo "scale=3; $total_duration / $ITERATIONS" | bc)
    printf " Avg: %.3f s\n" "$avg_duration"
}

# Task 1: Count Filter
QUERY1="SELECT COUNT(*) FROM log WHERE level = 'error'"

# Task 2: Aggregation
QUERY2="SELECT level, COUNT(*) FROM log GROUP BY level ORDER BY count(*)"

echo
echo "--- TASK 1: Count Filter ---"
for t in 1 2 4 8; do
    run_benchmark "$t" "$QUERY1" "Filter"
done

echo
echo "--- TASK 2: Aggregation ---"
for t in 1 2 4 8; do
    run_benchmark "$t" "$QUERY2" "Group By"
done
