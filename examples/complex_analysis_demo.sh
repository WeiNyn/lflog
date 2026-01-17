#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${MAGENTA}╔══════════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║                        lflog Complex Analysis Demo                           ║${NC}"
echo -e "${MAGENTA}║           Demonstrating Advanced Log Analysis with SQL + Macros              ║${NC}"
echo -e "${MAGENTA}╚══════════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Build lflog
echo -e "${YELLOW}Building lflog...${NC}"
cargo build --release --quiet 2>/dev/null || cargo build --quiet
LFLOG=${LFLOG:-./target/release/lflog}
if [ ! -f "$LFLOG" ]; then
    LFLOG=./target/debug/lflog
fi

CONFIG_FILE="examples/loghub_demos/config/loghub.toml"

# Helper function to display queries in a formatted box
show_query() {
    local query="$1"
    echo -e "${BOLD}Query:${NC}"
    echo -e "${CYAN}┌──────────────────────────────────────────────────────────────────────────────┐${NC}"
    # Print each line of the query with proper indentation
    echo "$query" | while IFS= read -r line; do
        printf "${CYAN}│${NC} %-76s ${CYAN}│${NC}\n" "$line"
    done
    echo -e "${CYAN}└──────────────────────────────────────────────────────────────────────────────┘${NC}"
    echo ""
}

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 1: Multi-Log Source Analysis with __FILE__ Tracking
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 1: Multi-Source Log Analysis                                            ${NC}"
echo -e "${CYAN}  Analyze distributed system logs (HDFS, Hadoop, Spark) in a single query     ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} You have logs from HDFS, Hadoop, and Spark. You want to:"
echo -e "  1. Count log entries by level across ALL systems"
echo -e "  2. See which system generates the most logs"
echo -e "  3. Track the source file for each entry"
echo ""

# We'll use separate table registrations and UNION them
echo -e "${GREEN}Running lflog with three log sources and advanced SQL...${NC}"
echo ""

echo -e "${YELLOW}Query 1: Overview of all distributed system logs${NC}"
echo -e "Pattern macros make parsing each format trivial - just reference the profile name!"
echo ""

# For HDFS
echo -e "${BLUE}─── HDFS Log Analysis ───${NC}"
QUERY="SELECT 
    'HDFS' as source,
    level,
    component,
    COUNT(*) as count
FROM log
GROUP BY level, component
ORDER BY count DESC
LIMIT 5"
show_query "$QUERY"
$LFLOG loghub/HDFS/HDFS_2k.log \
    --config "$CONFIG_FILE" \
    --profile hdfs \
    --add-file-path \
    --query "$QUERY"
echo ""

echo -e "${BLUE}─── Hadoop MapReduce Log Analysis ───${NC}"
QUERY="SELECT 
    'Hadoop' as source,
    level,
    component,
    COUNT(*) as count
FROM log
GROUP BY level, component
ORDER BY count DESC
LIMIT 5"
show_query "$QUERY"
$LFLOG loghub/Hadoop/Hadoop_2k.log \
    --config "$CONFIG_FILE" \
    --profile hadoop \
    --add-file-path \
    --query "$QUERY"
echo ""

echo -e "${BLUE}─── Spark Log Analysis ───${NC}"
QUERY="SELECT 
    'Spark' as source,
    level,
    component,
    COUNT(*) as count
FROM log
GROUP BY level, component
ORDER BY count DESC
LIMIT 5"
show_query "$QUERY"
$LFLOG loghub/Spark/Spark_2k.log \
    --config "$CONFIG_FILE" \
    --profile spark \
    --add-file-path \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 2: Advanced Aggregations and String Analysis
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 2: Advanced Aggregations - Finding Patterns in BGL Supercomputer Logs  ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} Blue Gene/L supercomputer logs contain critical system events."
echo -e "  - Find the most common error types"
echo -e "  - Identify which nodes have the most issues"
echo -e "  - Analyze message patterns"
echo ""

echo -e "${YELLOW}Query 2a: Nodes with most log entries (potential hotspots)${NC}"
QUERY="SELECT 
    node,
    level,
    COUNT(*) as event_count,
    COUNT(DISTINCT component) as unique_components
FROM log
GROUP BY node, level
HAVING COUNT(*) > 10
ORDER BY event_count DESC
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/BGL/BGL_2k.log \
    --config "$CONFIG_FILE" \
    --profile bgl \
    --query "$QUERY"
echo ""

echo -e "${YELLOW}Query 2b: Error pattern analysis - what types of issues occur?${NC}"
QUERY="SELECT 
    level,
    component,
    -- Extract first few words of message to find patterns
    SUBSTR(message, 1, 50) as message_preview,
    COUNT(*) as occurrences
FROM log
WHERE level != 'INFO'
GROUP BY level, component, SUBSTR(message, 1, 50)
ORDER BY occurrences DESC
LIMIT 8"
show_query "$QUERY"
$LFLOG loghub/BGL/BGL_2k.log \
    --config "$CONFIG_FILE" \
    --profile bgl \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 3: Multi-file Glob Pattern + Metadata Columns
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 3: Glob Patterns with __FILE__ and __RAW__ Metadata                    ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} Query multiple Apache-style logs at once and track their source."
echo ""

echo -e "${YELLOW}Query 3: Show entries with their source file and raw content${NC}"
QUERY="SELECT 
    level,
    \"__FILE__\" as source_file,
    SUBSTR(\"__RAW__\", 1, 80) as original_line
FROM log
WHERE level = 'error'
LIMIT 5"
show_query "$QUERY"
$LFLOG loghub/Apache/Apache_2k.log \
    --config "$CONFIG_FILE" \
    --profile apache \
    --add-file-path \
    --add-raw \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 4: Real-World Analysis - OpenStack Cloud Debugging
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 4: OpenStack Cloud Log Analysis - Finding API Request Patterns         ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} Debug OpenStack API issues by analyzing log patterns."
echo ""

echo -e "${YELLOW}Query 4a: Component activity breakdown${NC}"
QUERY="SELECT 
    filename,
    level,
    COUNT(*) as log_count
FROM log
GROUP BY filename, level
ORDER BY log_count DESC
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/OpenStack/OpenStack_2k.log \
    --config "$CONFIG_FILE" \
    --profile openstack \
    --query "$QUERY"
echo ""

echo -e "${YELLOW}Query 4b: Request tracking - find related log entries${NC}"
QUERY="SELECT 
    req_id,
    level,
    component,
    SUBSTR(message, 1, 60) as message_preview
FROM log
WHERE req_id != '-'
ORDER BY req_id
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/OpenStack/OpenStack_2k.log \
    --config "$CONFIG_FILE" \
    --profile openstack \
    --add-raw \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 5: Security Analysis - SSH Login Attempts
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 5: Security Analysis - SSH Authentication Patterns                     ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} Analyze SSH logs for security insights."
echo ""

echo -e "${YELLOW}Query 5a: Component activity in SSH logs${NC}"
QUERY="SELECT 
    hostname,
    component,
    COUNT(*) as events
FROM log
GROUP BY hostname, component
ORDER BY events DESC
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/OpenSSH/OpenSSH_2k.log \
    --config "$CONFIG_FILE" \
    --profile openssh \
    --query "$QUERY"
echo ""

echo -e "${YELLOW}Query 5b: Search for authentication-related messages${NC}"
QUERY="SELECT 
    timestamp,
    hostname,
    SUBSTR(message, 1, 70) as message
FROM log
WHERE LOWER(message) LIKE '%failed%' 
   OR LOWER(message) LIKE '%invalid%'
   OR LOWER(message) LIKE '%break-in%'
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/OpenSSH/OpenSSH_2k.log \
    --config "$CONFIG_FILE" \
    --profile openssh \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# DEMO 6: Mobile System Analysis - Android Logcat
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DEMO 6: Android App Debugging - Process & Thread Analysis                   ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Scenario:${NC} Debug Android app issues by analyzing logcat output."
echo ""

echo -e "${YELLOW}Query 6a: Activity by process ID (identify noisy processes)${NC}"
QUERY="SELECT 
    pid,
    level,
    COUNT(*) as log_count,
    COUNT(DISTINCT component) as component_count
FROM log
GROUP BY pid, level
ORDER BY log_count DESC
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/Android/Android_2k.log \
    --config "$CONFIG_FILE" \
    --profile android \
    --query "$QUERY"
echo ""

echo -e "${YELLOW}Query 6b: Warning and Error analysis by component${NC}"
QUERY="SELECT 
    level,
    component,
    SUBSTR(message, 1, 50) as message_snippet,
    COUNT(*) as count
FROM log
WHERE level IN ('W', 'E')
GROUP BY level, component, SUBSTR(message, 1, 50)
ORDER BY count DESC
LIMIT 10"
show_query "$QUERY"
$LFLOG loghub/Android/Android_2k.log \
    --config "$CONFIG_FILE" \
    --profile android \
    --query "$QUERY"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════════
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}  Summary: Why lflog Excels at Complex Log Analysis                           ${NC}"
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}Key Benefits Demonstrated:${NC}"
echo ""
echo -e "  ${BOLD}1. Intuitive Pattern Macros${NC}"
echo -e "     Instead of: regexp_extract(line, '\\[([^\\]]+)\\]', 1)"
echo -e "     Use:        {{level:var_name}} or {{timestamp:datetime(\"%Y-%m-%d\")}}"
echo ""
echo -e "  ${BOLD}2. Profile-based Configuration${NC}"
echo -e "     Define patterns once in config.toml, reuse across queries:"
echo -e "     --profile hdfs, --profile spark, --profile android"
echo ""
echo -e "  ${BOLD}3. Full SQL Power${NC}"
echo -e "     GROUP BY, HAVING, ORDER BY, LIMIT, LIKE, SUBSTR, COUNT, DISTINCT"
echo -e "     All powered by Apache DataFusion's optimized query engine"
echo ""
echo -e "  ${BOLD}4. Metadata Columns${NC}"
echo -e "     --add-file-path: Track which file each log came from (__FILE__)"
echo -e "     --add-raw: Access the original unparsed log line (__RAW__)"
echo ""
echo -e "  ${BOLD}5. Multi-format Support${NC}"
echo -e "     Parse Android logcat, Apache errors, HDFS, Hadoop, Spark, OpenStack,"
echo -e "     SSH, and 10+ other log formats with the same tool"
echo ""
echo -e "  ${BOLD}6. Zero-Copy Performance${NC}"
echo -e "     Memory-mapped file parsing, pre-calculated regex indices,"
echo -e "     and DataFusion's vectorized execution for fast analysis"
echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Complex Analysis Demo Complete! ✓                         ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════════════════════╝${NC}"
