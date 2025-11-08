#!/bin/bash
# Database management script for TPC-H example
# Manages Docker Compose PostgreSQL database lifecycle

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Load database URL from .env.test (filter out comments and empty lines)
export $(cat .env.test | grep -v '^#' | grep -v '^$' | xargs)

function show_usage() {
    echo "Usage: $0 <command>"
    echo
    echo "Commands:"
    echo "  start       Start PostgreSQL container"
    echo "  stop        Stop PostgreSQL container"
    echo "  restart     Restart PostgreSQL container"
    echo "  reset       Stop, remove volumes, and start fresh"
    echo "  migrate     Run database migrations"
    echo "  status      Check database status"
    echo "  logs        Show database logs"
    echo "  shell       Open psql shell"
    echo
}

function start_db() {
    echo -e "${BLUE}Starting PostgreSQL database...${NC}"

    # Check if Docker is running
    if ! docker info > /dev/null 2>&1; then
        echo -e "${RED}✗ Docker daemon is not running${NC}"
        echo -e "${YELLOW}Please start Docker Desktop and try again${NC}"
        return 1
    fi

    docker compose up -d

    echo -e "${YELLOW}Waiting for database to be ready...${NC}"
    for i in {1..30}; do
        if docker compose exec -T postgres pg_isready -U tpch_user -d tpch_db > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Database is ready${NC}"
            return 0
        fi
        sleep 1
    done

    echo -e "${RED}✗ Database failed to start${NC}"
    return 1
}

function stop_db() {
    echo -e "${BLUE}Stopping PostgreSQL database...${NC}"
    docker compose down
    echo -e "${GREEN}✓ Database stopped${NC}"
}

function restart_db() {
    stop_db
    start_db
}

function reset_db() {
    echo -e "${YELLOW}Resetting database (removing all data)...${NC}"
    docker compose down -v
    start_db
    migrate_db
}

function migrate_db() {
    echo -e "${BLUE}Running database migrations...${NC}"

    # Check if diesel CLI is installed
    if ! command -v diesel &> /dev/null; then
        echo -e "${YELLOW}Diesel CLI not found. Installing...${NC}"
        cargo install diesel_cli --no-default-features --features postgres
    fi

    # Run migrations
    diesel migration run --database-url="$DATABASE_URL"
    echo -e "${GREEN}✓ Migrations complete${NC}"
}

function status_db() {
    echo -e "${BLUE}Database status:${NC}"
    if docker compose ps | grep -q "Up"; then
        echo -e "${GREEN}✓ Database is running${NC}"
        docker compose ps

        # Try to connect
        if docker compose exec -T postgres pg_isready -U tpch_user -d tpch_db > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Database is accepting connections${NC}"
        else
            echo -e "${RED}✗ Database is not accepting connections${NC}"
        fi
    else
        echo -e "${RED}✗ Database is not running${NC}"
    fi
}

function logs_db() {
    docker compose logs -f postgres
}

function shell_db() {
    echo -e "${BLUE}Opening psql shell...${NC}"
    echo -e "${YELLOW}(Use \\q to exit)${NC}"
    docker compose exec postgres psql -U tpch_user -d tpch_db
}

# Main command dispatcher
case "${1:-}" in
    start)
        start_db
        ;;
    stop)
        stop_db
        ;;
    restart)
        restart_db
        ;;
    reset)
        reset_db
        ;;
    migrate)
        migrate_db
        ;;
    status)
        status_db
        ;;
    logs)
        logs_db
        ;;
    shell)
        shell_db
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
