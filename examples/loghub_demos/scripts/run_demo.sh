#!/bin/bash

# Dataset name map (lowercase -> Directory/File prefix)
declare -A DATASETS
DATASETS=(
    ["android"]="Android/Android_2k.log"
    ["apache"]="Apache/Apache_2k.log"
    ["bgl"]="BGL/BGL_2k.log"
    ["hadoop"]="Hadoop/Hadoop_2k.log"
    ["hdfs"]="HDFS/HDFS_2k.log"
    ["healthapp"]="HealthApp/HealthApp_2k.log"
    ["hpc"]="HPC/HPC_2k.log"
    ["linux"]="Linux/Linux_2k.log"
    ["mac"]="Mac/Mac_2k.log"
    ["openssh"]="OpenSSH/OpenSSH_2k.log"
    ["openstack"]="OpenStack/OpenStack_2k.log"
    ["proxifier"]="Proxifier/Proxifier_2k.log"
    ["spark"]="Spark/Spark_2k.log"
    ["thunderbird"]="Thunderbird/Thunderbird_2k.log"
    ["windows"]="Windows/Windows_2k.log"
    ["zookeeper"]="Zookeeper/Zookeeper_2k.log"
)

if [ -z "$1" ]; then
    echo "Usage: $0 <dataset>"
    echo "Available datasets:"
    for key in "${!DATASETS[@]}"; do
        echo "  $key"
    done
    exit 1
fi

DATASET=$1
LOG_FILE="${DATASETS[$DATASET]}"

if [ -z "$LOG_FILE" ]; then
    echo "Error: Unknown dataset '$DATASET'"
    exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
LOG_PATH="$REPO_ROOT/loghub/$LOG_FILE"
CONFIG_PATH="$REPO_ROOT/examples/loghub_demos/config/loghub.toml"
QUERY_PATH="$REPO_ROOT/examples/loghub_demos/queries/$DATASET.sql"

if [ ! -f "$LOG_PATH" ]; then
    echo "Error: Log file not found at $LOG_PATH"
    exit 1
fi

if [ ! -f "$QUERY_PATH" ]; then
    echo "Error: Query file not found at $QUERY_PATH"
    exit 1
fi

# Build lflog if needed (release mode for speed)
# cd "$REPO_ROOT" && cargo build --release --quiet

echo "=== Running Demo: $DATASET ==="
echo "Log File: $LOG_PATH"
echo "Profile:  $DATASET"
echo "Query:    $(cat $QUERY_PATH)"
echo "--------------------------------"

"$REPO_ROOT/target/release/lflog" "$LOG_PATH" \
    --config "$CONFIG_PATH" \
    --profile "$DATASET" \
    --query "$(cat $QUERY_PATH)"
