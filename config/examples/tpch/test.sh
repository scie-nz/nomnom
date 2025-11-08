#!/bin/bash
# Test script for TPC-H record_parser binary
# Generates random test data and pipes it through the parser with various flags

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if parser binary exists
PARSER_BIN="./target/debug/record_parser"
if [ ! -f "$PARSER_BIN" ]; then
    echo "Error: Parser binary not found at $PARSER_BIN"
    echo "Run ./build.sh first to build the project"
    exit 1
fi

# Check for --with-db flag
WITH_DB=false
if [[ "$1" == "--with-db" ]]; then
    WITH_DB=true
    shift  # Remove --with-db from arguments
fi

# Parse remaining command line arguments (forward all flags to parser)
PARSER_FLAGS="$@"
DEFAULT_COUNT=3

echo -e "${BLUE}=== TPC-H Parser Test ===${NC}"
echo

# If --with-db flag is set, check database and load data
if [ "$WITH_DB" = true ]; then
    echo -e "${YELLOW}Database mode enabled${NC}"
    echo

    # Check if database is running
    if ! docker compose ps | grep -q "Up"; then
        echo -e "${YELLOW}Database not running. Starting database...${NC}"
        ./db.sh start
        echo
    fi

    # Load environment variables (filter out comments and empty lines)
    export $(cat .env.test | grep -v '^#' | grep -v '^$' | xargs)

    # Generate test data and pipe to SQL file
    echo -e "${BLUE}Generating SQL from test data...${NC}"
    python3 generate_test_data.py --count 5 --seed 42 | "$PARSER_BIN" --sql-only > /tmp/tpch_test.sql
    echo -e "${GREEN}✓ SQL generated at /tmp/tpch_test.sql${NC}"
    echo

    # Show first few lines of SQL
    echo -e "${BLUE}Sample SQL output:${NC}"
    head -n 20 /tmp/tpch_test.sql
    echo "..."
    echo

    # Execute SQL against database (filter to only INSERT statements)
    echo -e "${BLUE}Executing SQL against database...${NC}"
    # Extract only INSERT statements from the SQL file (INSERT + VALUES + data line + semicolon)
    grep -A 3 "^INSERT INTO" /tmp/tpch_test.sql | grep -v "^--$" > /tmp/tpch_test_inserts.sql
    docker compose exec -T postgres psql -U tpch_user -d tpch_db < /tmp/tpch_test_inserts.sql > /dev/null 2>&1

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Data loaded into database${NC}"
    else
        echo -e "${YELLOW}⚠ Some inserts may have failed (duplicate keys are OK)${NC}"
    fi
    echo

    # Query database to verify data
    echo -e "${BLUE}Verifying data in database:${NC}"
    docker compose exec -T postgres psql -U tpch_user -d tpch_db -c "SELECT COUNT(*) as order_count FROM orders;"
    docker compose exec -T postgres psql -U tpch_user -d tpch_db -c "SELECT COUNT(*) as line_item_count FROM order_line_items;"
    echo

    echo -e "${GREEN}✓ Database test complete${NC}"
    echo -e "${BLUE}To query the database: ./db.sh shell${NC}"
    exit 0
fi

# If no flags provided, run all test modes
if [ -z "$PARSER_FLAGS" ]; then
    echo -e "${GREEN}Running all test modes...${NC}"
    echo

    # Test 1: JSON output only
    echo -e "${BLUE}1. JSON Output (--json-only):${NC}"
    python3 generate_test_data.py --count 2 --seed 123 | "$PARSER_BIN" --json-only
    echo

    # Test 2: SQL output only
    echo -e "${BLUE}2. SQL Output (--sql-only):${NC}"
    python3 generate_test_data.py --count 1 --seed 456 | "$PARSER_BIN" --sql-only
    echo

    # Test 3: Lineage visualization
    echo -e "${BLUE}3. Lineage Tree (--show-lineage):${NC}"
    python3 generate_test_data.py --count 1 --seed 789 | "$PARSER_BIN" --show-lineage
    echo

    # Test 4: JSON with lineage metadata
    echo -e "${BLUE}4. JSON with Lineage (--json-only --lineage):${NC}"
    python3 generate_test_data.py --count 1 --seed 111 | "$PARSER_BIN" --json-only --lineage
    echo

else
    # Run with user-provided flags
    echo -e "${GREEN}Running parser with flags: ${PARSER_FLAGS}${NC}"
    echo
    python3 generate_test_data.py --count "$DEFAULT_COUNT" | "$PARSER_BIN" $PARSER_FLAGS
fi

echo
echo -e "${GREEN}✓ Test complete${NC}"
