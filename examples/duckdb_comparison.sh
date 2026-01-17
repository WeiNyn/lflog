#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              lflog vs DuckDB Comparison Demo                       ║${NC}"
echo -e "${BLUE}║        Querying Log Files: Macros vs Raw Regex                     ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check for duckdb
if ! command -v duckdb &> /dev/null; then
    echo -e "${RED}Error: duckdb is not installed. Please install it first.${NC}"
    exit 1
fi

# Build lflog
echo -e "${YELLOW}Building lflog...${NC}"
cargo build --release --quiet 2>/dev/null || cargo build --quiet
LFLOG=${LFLOG:-./target/release/lflog}
if [ ! -f "$LFLOG" ]; then
    LFLOG=./target/debug/lflog
fi

# Use the Apache log sample
LOG_FILE="loghub/Apache/Apache_2k.log"
if [ ! -f "$LOG_FILE" ]; then
    echo -e "${RED}Error: $LOG_FILE not found. Please ensure loghub dataset is available.${NC}"
    exit 1
fi

echo -e "${GREEN}Using log file: $LOG_FILE${NC}"
echo -e "${GREEN}Sample line:${NC}"
head -1 "$LOG_FILE"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# COMPARISON 1: Basic Query - Count by log level
# ═══════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  COMPARISON 1: Count logs by level${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}┌─────────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│ lflog (with macros)                                                 │${NC}"
echo -e "${BLUE}└─────────────────────────────────────────────────────────────────────┘${NC}"
LFLOG_PATTERN='^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$'
echo -e "${YELLOW}Pattern:${NC} $LFLOG_PATTERN"
echo -e "${YELLOW}Query:${NC}   SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC"
echo ""
time $LFLOG "$LOG_FILE" \
    --pattern "$LFLOG_PATTERN" \
    --query "SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC"
echo ""

echo -e "${RED}┌─────────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${RED}│ DuckDB (with raw regex)                                             │${NC}"
echo -e "${RED}└─────────────────────────────────────────────────────────────────────┘${NC}"
echo -e "${YELLOW}Regex:${NC}   ^\[[^\]]+\] \[([^\]]+)\]"
echo -e "${YELLOW}Query:${NC}   (see below - requires regexp_extract for each field)"
echo ""
time duckdb -c "
SELECT 
    regexp_extract(line, '\[[^\]]+\] \[([^\]]+)\]', 1) as level,
    COUNT(*) as count
FROM read_csv('$LOG_FILE', columns={'line': 'VARCHAR'}, header=false, delim=E'\x1F')
WHERE line LIKE '[%] [%]%'
GROUP BY level
ORDER BY count DESC;
"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# COMPARISON 2: Filter and extract fields
# ═══════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  COMPARISON 2: Filter errors and show messages${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}┌─────────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│ lflog (with macros)                                                 │${NC}"
echo -e "${BLUE}└─────────────────────────────────────────────────────────────────────┘${NC}"
echo -e "${YELLOW}Query:${NC} SELECT time, message FROM log WHERE level = 'error' LIMIT 5"
echo ""
time $LFLOG "$LOG_FILE" \
    --pattern "$LFLOG_PATTERN" \
    --query "SELECT time, message FROM log WHERE level = 'error' LIMIT 5"
echo ""

echo -e "${RED}┌─────────────────────────────────────────────────────────────────────┐${NC}"
echo -e "${RED}│ DuckDB (with raw regex)                                             │${NC}"
echo -e "${RED}└─────────────────────────────────────────────────────────────────────┘${NC}"
echo -e "${YELLOW}Query:${NC} (requires multiple regexp_extract calls)"
echo ""
time duckdb -c "
SELECT 
    regexp_extract(line, '\[([^\]]+)\]', 1) as time,
    regexp_extract(line, '\[[^\]]+\] \[[^\]]+\] (.*)', 1) as message
FROM read_csv('$LOG_FILE', columns={'line': 'VARCHAR'}, header=false, delim=E'\x1F')
WHERE regexp_extract(line, '\[[^\]]+\] \[([^\]]+)\]', 1) = 'error'
LIMIT 5;
"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# COMPARISON 3: Code readability
# ═══════════════════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  COMPARISON 3: Pattern Readability${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}lflog pattern (intuitive macros):${NC}"
echo '  ^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$'
echo ""
echo -e "${RED}DuckDB regex (raw regex, repeated for each field):${NC}"
echo "  time:    regexp_extract(line, '\[([^\]]+)\]', 1)"
echo "  level:   regexp_extract(line, '\[[^\]]+\] \[([^\]]+)\]', 1)"
echo "  message: regexp_extract(line, '\[[^\]]+\] \[[^\]]+\] (.*)', 1)"
echo ""

echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  Key Differences${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${BLUE}lflog:${NC}"
echo -e "    ✓ Named fields directly in pattern: {{level:var_name}}"
echo -e "    ✓ Readable macros: datetime(\"%Y-%m-%d\"), number, ip, uuid"
echo -e "    ✓ Automatic schema generation with type inference"
echo -e "    ✓ Fields available as columns: SELECT level, time FROM log"
echo -e "    ✓ Glob patterns for multiple files: 'logs/*.log'"
echo -e "    ✓ Metadata columns: __FILE__, __RAW__"
echo ""
echo -e "  ${RED}DuckDB:${NC}"
echo -e "    ✗ Raw regex with capture groups: ([^\]]+)"
echo -e "    ✗ Manual field extraction: regexp_extract(line, regex, group_num)"
echo -e "    ✗ Repeat regex for each field extraction"
echo -e "    ✗ No automatic type inference from pattern"
echo -e "    ✗ More verbose queries"
echo ""

echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              Comparison completed successfully! ✓                  ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════╝${NC}"
