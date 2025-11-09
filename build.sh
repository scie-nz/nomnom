#!/bin/bash
# Build script for nomnom library
# Handles PostgreSQL library linking for macOS and Linux

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Building nomnom library${NC}"
echo -e "${BLUE}================================${NC}"
echo ""

# Detect OS and set PostgreSQL library path
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    if [ -d "/opt/homebrew/opt/postgresql@17/lib" ]; then
        # Apple Silicon (M1/M2) - PostgreSQL 17
        PG_LIB_PATH="/opt/homebrew/opt/postgresql@17/lib"
        echo -e "${GREEN}✓${NC} Detected macOS (Apple Silicon - PostgreSQL 17)"
    elif [ -d "/opt/homebrew/opt/postgresql/lib" ]; then
        # Apple Silicon (M1/M2)
        PG_LIB_PATH="/opt/homebrew/opt/postgresql/lib"
        echo -e "${GREEN}✓${NC} Detected macOS (Apple Silicon)"
    elif [ -d "/usr/local/opt/postgresql/lib" ]; then
        # Intel Mac
        PG_LIB_PATH="/usr/local/opt/postgresql/lib"
        echo -e "${GREEN}✓${NC} Detected macOS (Intel)"
    elif [ -d "/opt/homebrew/opt/libpq/lib" ]; then
        # libpq only (Apple Silicon)
        PG_LIB_PATH="/opt/homebrew/opt/libpq/lib"
        echo -e "${GREEN}✓${NC} Detected macOS (Apple Silicon - libpq)"
    elif [ -d "/usr/local/opt/libpq/lib" ]; then
        # libpq only (Intel)
        PG_LIB_PATH="/usr/local/opt/libpq/lib"
        echo -e "${GREEN}✓${NC} Detected macOS (Intel - libpq)"
    else
        echo -e "${YELLOW}⚠${NC}  PostgreSQL library not found"
        echo -e "${YELLOW}⚠${NC}  Install with: brew install postgresql"
        PG_LIB_PATH=""
    fi
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    if [ -d "/usr/lib/x86_64-linux-gnu" ]; then
        PG_LIB_PATH="/usr/lib/x86_64-linux-gnu"
        echo -e "${GREEN}✓${NC} Detected Linux (x86_64)"
    elif [ -d "/usr/lib64" ]; then
        PG_LIB_PATH="/usr/lib64"
        echo -e "${GREEN}✓${NC} Detected Linux (lib64)"
    else
        echo -e "${YELLOW}⚠${NC}  PostgreSQL library not found"
        echo -e "${YELLOW}⚠${NC}  Install with: sudo apt-get install libpq-dev"
        PG_LIB_PATH=""
    fi
else
    echo -e "${YELLOW}⚠${NC}  Unknown OS: $OSTYPE"
    PG_LIB_PATH=""
fi

# Set RUSTFLAGS if PostgreSQL library path was found
if [ -n "$PG_LIB_PATH" ]; then
    export RUSTFLAGS="-L $PG_LIB_PATH"
    echo -e "${GREEN}✓${NC} Set RUSTFLAGS=\"-L $PG_LIB_PATH\""
else
    echo -e "${YELLOW}⚠${NC}  Building without PostgreSQL support"
fi

echo ""

# Parse command line arguments
BUILD_TYPE="lib"
RELEASE_FLAG=""
VERBOSE_FLAG=""
CLEAN_FIRST=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release|-r)
            RELEASE_FLAG="--release"
            echo -e "${BLUE}→${NC} Building in release mode"
            shift
            ;;
        --verbose|-v)
            VERBOSE_FLAG="--verbose"
            echo -e "${BLUE}→${NC} Verbose output enabled"
            shift
            ;;
        --clean|-c)
            CLEAN_FIRST=true
            echo -e "${BLUE}→${NC} Will clean before building"
            shift
            ;;
        --all|-a)
            BUILD_TYPE="all"
            echo -e "${BLUE}→${NC} Building library and tests"
            shift
            ;;
        --test|-t)
            BUILD_TYPE="test"
            echo -e "${BLUE}→${NC} Running tests"
            shift
            ;;
        --help|-h)
            echo "Usage: ./build.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -r, --release    Build in release mode"
            echo "  -v, --verbose    Enable verbose output"
            echo "  -c, --clean      Clean before building"
            echo "  -a, --all        Build library and all targets"
            echo "  -t, --test       Run tests after building"
            echo "  -h, --help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./build.sh                  # Build library in debug mode"
            echo "  ./build.sh --release        # Build library in release mode"
            echo "  ./build.sh --test           # Build and run tests"
            echo "  ./build.sh -r -t            # Release build with tests"
            echo "  ./build.sh --clean --all    # Clean, then build everything"
            exit 0
            ;;
        *)
            echo -e "${RED}✗${NC} Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo ""

# Clean if requested
if [ "$CLEAN_FIRST" = true ]; then
    echo -e "${YELLOW}→${NC} Cleaning build artifacts..."
    cargo clean
    echo -e "${GREEN}✓${NC} Clean complete"
    echo ""
fi

# Build based on type
case $BUILD_TYPE in
    lib)
        echo -e "${YELLOW}→${NC} Building library..."
        if cargo build --lib $RELEASE_FLAG $VERBOSE_FLAG; then
            echo -e "${GREEN}✓${NC} Library build successful"
        else
            echo -e "${RED}✗${NC} Library build failed"
            exit 1
        fi
        ;;
    all)
        echo -e "${YELLOW}→${NC} Building all targets..."
        if cargo build --all-targets $RELEASE_FLAG $VERBOSE_FLAG; then
            echo -e "${GREEN}✓${NC} Build successful"
        else
            echo -e "${RED}✗${NC} Build failed"
            exit 1
        fi
        ;;
    test)
        echo -e "${YELLOW}→${NC} Building and running tests..."
        if cargo test $RELEASE_FLAG $VERBOSE_FLAG; then
            echo -e "${GREEN}✓${NC} All tests passed"
        else
            echo -e "${RED}✗${NC} Tests failed"
            exit 1
        fi
        ;;
esac

echo ""
echo -e "${BLUE}================================${NC}"
echo -e "${GREEN}✓ Build completed successfully${NC}"
echo -e "${BLUE}================================${NC}"

# Show build artifacts location
if [ "$RELEASE_FLAG" = "--release" ]; then
    BUILD_DIR="target/release"
else
    BUILD_DIR="target/debug"
fi

echo ""
echo -e "${BLUE}Build artifacts:${NC} $BUILD_DIR"
echo ""
