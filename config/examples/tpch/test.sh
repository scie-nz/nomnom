#!/bin/bash
# Test script for TPC-H record_parser binary
# Generates random test data and pipes it through the parser with various flags

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if parser binary exists
PARSER_BIN="./target/debug/record_parser"
if [ ! -f "$PARSER_BIN" ]; then
    echo "Error: Parser binary not found at $PARSER_BIN"
    echo "Run ./build.sh first to build the project"
    exit 1
fi

# Parse command line arguments (forward all flags to parser)
PARSER_FLAGS="$@"
DEFAULT_COUNT=3

echo -e "${BLUE}=== TPC-H Parser Test ===${NC}"
echo

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
