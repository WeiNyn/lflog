#!/bin/bash
set -e

SOURCE_LOG="../../loghub/Apache/Apache_2k.log"
TARGET_LOG="Apache_large.log"
TARGET_SIZE_MB=100

if [ ! -f "$SOURCE_LOG" ]; then
    echo "Error: Source log $SOURCE_LOG not found."
    exit 1
fi

echo "Generating $TARGET_LOG (approx $TARGET_SIZE_MB MB)..."

# Clear existing file
> "$TARGET_LOG"

# Get source size
SOURCE_SIZE=$(du -k "$SOURCE_LOG" | cut -f1)
# Calculate iterations needed (approximate)
ITERATIONS=$(( (TARGET_SIZE_MB * 1024) / SOURCE_SIZE ))

echo "Appending source file $ITERATIONS times..."

for ((i=1; i<=ITERATIONS; i++)); do
    cat "$SOURCE_LOG" >> "$TARGET_LOG"
    if (( i % 50 == 0 )); then
        echo -n "."
    fi
done

echo
echo "Done. Final size:"
du -h "$TARGET_LOG"
