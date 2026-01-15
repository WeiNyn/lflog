#!/bin/bash
set -e

SOURCE_LOG="../../loghub/Apache/Apache_2k.log"
TARGET_LOG="Apache_10M.log"
TARGET_LINES=10000000

if [ ! -f "$SOURCE_LOG" ]; then
    echo "Error: Source log $SOURCE_LOG not found."
    exit 1
fi

SOURCE_LINES=$(wc -l < "$SOURCE_LOG")
ITERATIONS=$(( TARGET_LINES / SOURCE_LINES ))

echo "Generating $TARGET_LOG with approximately $TARGET_LINES lines..."
echo "Source: $SOURCE_LOG ($SOURCE_LINES lines)"
echo "Iterations: $ITERATIONS"

# Clear existing file
> "$TARGET_LOG"

# Efficient way to repeat file: read once, write multiple times using a loop in a subshell or temporary file
# But cat in a loop is simple enough for this scale (5000 iterations is fast)

start=$(date +%s)

# Create a temporary 100k line file first to speed up (50 iterations)
TEMP_CHUNK="temp_chunk.log"
> "$TEMP_CHUNK"
for ((i=1; i<=50; i++)); do
    cat "$SOURCE_LOG" >> "$TEMP_CHUNK"
done

CHUNK_LINES=$(wc -l < "$TEMP_CHUNK")
CHUNK_ITERATIONS=$(( TARGET_LINES / CHUNK_LINES ))

echo "Created chunk of $CHUNK_LINES lines. Appending $CHUNK_ITERATIONS times..."

for ((i=1; i<=CHUNK_ITERATIONS; i++)); do
    cat "$TEMP_CHUNK" >> "$TARGET_LOG"
    if (( i % 10 == 0 )); then
        echo -n "."
    fi
done

rm "$TEMP_CHUNK"

end=$(date +%s)
duration=$((end - start))

echo
echo "Done in $duration seconds."
echo "Final stats:"
wc -l "$TARGET_LOG"
du -h "$TARGET_LOG"
