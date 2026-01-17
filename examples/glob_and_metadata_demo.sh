#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          Glob Patterns & Metadata Columns Demo             ║${NC}"
echo -e "${BLUE}║  Query multiple files | __FILE__ | __RAW__ | Threads       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Build the project first
echo -e "${YELLOW}Building lflog...${NC}"
cargo build --release --quiet 2>/dev/null || cargo build --quiet
BINARY=${BINARY:-./target/release/lflog}
if [ ! -f "$BINARY" ]; then
    BINARY=./target/debug/lflog
fi

# Create sample log files
mkdir -p logs

# App server logs
cat > logs/app-server.log << 'EOF'
[2023-01-15 10:00:00] [INFO] Server started on port 8080
[2023-01-15 10:00:05] [DEBUG] Loading configuration
[2023-01-15 10:01:22] [WARN] High memory usage detected
[2023-01-15 10:02:45] [ERROR] Database connection timeout
[2023-01-15 10:03:00] [INFO] Retrying database connection
[2023-01-15 10:03:01] [INFO] Database connected successfully
EOF

# Web server logs
cat > logs/web-server.log << 'EOF'
[2023-01-15 10:00:10] [INFO] Web server initialized
[2023-01-15 10:00:15] [INFO] Request: GET /api/users
[2023-01-15 10:00:16] [DEBUG] Cache hit for /api/users
[2023-01-15 10:01:00] [ERROR] Invalid auth token
[2023-01-15 10:01:30] [WARN] Rate limit approaching for IP 192.168.1.100
[2023-01-15 10:02:00] [INFO] Request: POST /api/orders
EOF

# Worker logs
cat > logs/worker.log << 'EOF'
[2023-01-15 10:00:20] [INFO] Background worker started
[2023-01-15 10:00:30] [INFO] Processing job #1234
[2023-01-15 10:01:00] [ERROR] Job #1234 failed: timeout
[2023-01-15 10:01:01] [INFO] Retrying job #1234
[2023-01-15 10:01:05] [INFO] Job #1234 completed successfully
EOF

echo -e "${GREEN}Created sample logs in ./logs/${NC}"
echo ""

# Demo 1: Glob Patterns
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  1. GLOB PATTERNS - Query multiple files at once${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: lflog 'logs/*.log' --pattern '...' --query \"SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC\""
echo ""
$BINARY "logs/*.log" \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --query "SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC"
echo ""

# Demo 2: __FILE__ metadata column
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  2. __FILE__ COLUMN - Track source files${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: lflog 'logs/*.log' ${YELLOW}--add-file-path${NC} --query \"SELECT level, message, \\\"__FILE__\\\" FROM log WHERE level = 'ERROR'\""
echo ""
$BINARY "logs/*.log" \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --add-file-path \
    --query "SELECT level, message, \"__FILE__\" FROM log WHERE level = 'ERROR'"
echo ""

# Demo 3: __RAW__ metadata column
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  3. __RAW__ COLUMN - Access original log lines${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: lflog logs/app-server.log ${YELLOW}--add-raw${NC} --query \"SELECT level, \\\"__RAW__\\\" FROM log WHERE level = 'WARN' OR level = 'ERROR'\""
echo ""
$BINARY logs/app-server.log \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --add-raw \
    --query "SELECT level, \"__RAW__\" FROM log WHERE level = 'WARN' OR level = 'ERROR'"
echo ""

# Demo 4: Both metadata columns
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  4. BOTH METADATA COLUMNS - Full context${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: lflog 'logs/*.log' ${YELLOW}--add-file-path --add-raw${NC} --query \"SELECT \\\"__FILE__\\\", \\\"__RAW__\\\" FROM log WHERE level = 'ERROR'\""
echo ""
$BINARY "logs/*.log" \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --add-file-path \
    --add-raw \
    --query "SELECT \"__FILE__\", \"__RAW__\" FROM log WHERE level = 'ERROR'"
echo ""

# Demo 5: Thread control
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  5. THREAD CONTROL - Tune parallelism${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: lflog 'logs/*.log' ${YELLOW}--num-threads 2${NC} --query \"SELECT COUNT(*) as total_logs FROM log\""
echo ""
$BINARY "logs/*.log" \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --num-threads 2 \
    --query "SELECT COUNT(*) as total_logs FROM log"
echo ""

# Demo 6: Complex aggregation across files
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  6. ADVANCED - Error analysis across services${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Command: Aggregate errors by source file"
echo ""
$BINARY "logs/*.log" \
    --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
    --add-file-path \
    --query "SELECT \"__FILE__\" as source, COUNT(*) as error_count FROM log WHERE level = 'ERROR' GROUP BY \"__FILE__\" ORDER BY error_count DESC"
echo ""

# Cleanup
rm -rf logs

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              Demo completed successfully! ✓                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
