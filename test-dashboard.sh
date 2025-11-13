#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Nomnom Dashboard Test Script${NC}"
echo -e "${BLUE}========================================${NC}\n"

# Configuration
ENTITIES_DIR="${1:-config/examples/tpch/entities}"
OUTPUT_DIR="${2:-/tmp/nomnom-dashboard-test}"
DATABASE_TYPE="${3:-postgresql}"

echo -e "${YELLOW}Configuration:${NC}"
echo -e "  Entities: ${ENTITIES_DIR}"
echo -e "  Output:   ${OUTPUT_DIR}"
echo -e "  Database: ${DATABASE_TYPE}\n"

# Step 1: Generate Dashboard
echo -e "${GREEN}[1/6] Generating dashboard...${NC}"
./target/debug/nomnom generate-dashboard \
  --entities "${ENTITIES_DIR}" \
  --output "${OUTPUT_DIR}" \
  --database "${DATABASE_TYPE}"

# Step 2: Set up database connection
echo -e "\n${GREEN}[2/6] Setting up database connection...${NC}"

if [ ! -f "${OUTPUT_DIR}/backend/.env" ]; then
  echo -e "${YELLOW}Please enter your database connection details (press Enter for defaults):${NC}"

  if [ "${DATABASE_TYPE}" = "postgresql" ] || [ "${DATABASE_TYPE}" = "postgres" ] || [ "${DATABASE_TYPE}" = "pg" ]; then
    read -p "PostgreSQL host [localhost]: " DB_HOST
    DB_HOST=${DB_HOST:-localhost}
    read -p "PostgreSQL port [5432]: " DB_PORT
    DB_PORT=${DB_PORT:-5432}
    read -p "PostgreSQL database name [nomnom]: " DB_NAME
    DB_NAME=${DB_NAME:-nomnom}
    read -p "PostgreSQL user [postgres]: " DB_USER
    DB_USER=${DB_USER:-postgres}
    read -sp "PostgreSQL password [postgres]: " DB_PASS
    DB_PASS=${DB_PASS:-postgres}
    echo

    echo "DATABASE_URL=postgresql://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}" > "${OUTPUT_DIR}/backend/.env"
  else
    read -p "MySQL host [localhost]: " DB_HOST
    DB_HOST=${DB_HOST:-localhost}
    read -p "MySQL port [3306]: " DB_PORT
    DB_PORT=${DB_PORT:-3306}
    read -p "MySQL database name [nomnom]: " DB_NAME
    DB_NAME=${DB_NAME:-nomnom}
    read -p "MySQL user [root]: " DB_USER
    DB_USER=${DB_USER:-root}
    read -sp "MySQL password [password]: " DB_PASS
    DB_PASS=${DB_PASS:-password}
    echo

    echo "DATABASE_URL=mysql://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}" > "${OUTPUT_DIR}/backend/.env"
  fi

  echo -e "${GREEN}✓ Database configuration saved${NC}"
else
  echo -e "${YELLOW}Using existing .env file${NC}"
fi

# Step 3: Check database and run migrations
echo -e "\n${GREEN}[3/6] Checking database and running migrations...${NC}"

# Check if database server is accessible
if [ "${DATABASE_TYPE}" = "postgresql" ] || [ "${DATABASE_TYPE}" = "postgres" ] || [ "${DATABASE_TYPE}" = "pg" ]; then
  # Extract connection details from .env
  if [ -f "${OUTPUT_DIR}/backend/.env" ]; then
    DB_URL=$(grep DATABASE_URL "${OUTPUT_DIR}/backend/.env" | cut -d= -f2)

    # Try to connect to PostgreSQL
    if ! psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
      echo -e "${YELLOW}⚠ Cannot connect to PostgreSQL server${NC}"

      # Check if we can start PostgreSQL with Docker
      if command -v docker &> /dev/null; then
        echo -e "${YELLOW}Attempting to start PostgreSQL with Docker...${NC}"

        # Check if postgres container already exists
        if docker ps -a --format '{{.Names}}' | grep -q "^postgres$"; then
          echo "PostgreSQL container exists, starting it..."
          docker start postgres > /dev/null 2>&1
        else
          echo "Creating new PostgreSQL container..."
          docker run -d \
            -p 5432:5432 \
            -e POSTGRES_PASSWORD=postgres \
            -e POSTGRES_USER=postgres \
            -e POSTGRES_DB=nomnom \
            --name postgres \
            postgres:14 > /dev/null 2>&1
        fi

        # Wait for PostgreSQL to be ready
        echo "Waiting for PostgreSQL to start..."
        for i in {1..30}; do
          if psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ PostgreSQL started successfully${NC}"
            break
          fi
          sleep 1
        done

        # Update .env file with correct Docker credentials if we started PostgreSQL
        echo "DATABASE_URL=postgresql://postgres:postgres@localhost:5432/nomnom" > "${OUTPUT_DIR}/backend/.env"
        DB_URL="postgresql://postgres:postgres@localhost:5432/nomnom"

        # Check if connection succeeded
        if ! psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
          echo -e "${RED}Failed to start PostgreSQL with Docker${NC}"
          read -p "Skip migrations and continue? (y/n) [y]: " SKIP_MIGRATIONS
          SKIP_MIGRATIONS=${SKIP_MIGRATIONS:-y}

          if [ "$SKIP_MIGRATIONS" != "y" ]; then
            exit 1
          fi
        fi
      else
        echo -e "${YELLOW}Docker not found. To start PostgreSQL:${NC}"
        echo -e "  macOS (Homebrew): brew services start postgresql@14"
        echo -e "  Ubuntu:           sudo systemctl start postgresql"
        echo -e "  Docker:           docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres --name postgres postgres:14"
        echo
        read -p "Skip migrations and continue? (y/n) [y]: " SKIP_MIGRATIONS
        SKIP_MIGRATIONS=${SKIP_MIGRATIONS:-y}

        if [ "$SKIP_MIGRATIONS" != "y" ]; then
          echo -e "${RED}Exiting. Please start PostgreSQL and run: ${OUTPUT_DIR}/migrations/run.sh${NC}"
          exit 1
        else
          echo -e "${YELLOW}Skipping migrations. Run manually later: ${OUTPUT_DIR}/migrations/run.sh${NC}"
        fi
      fi
    fi

    # Run migrations if we can connect
    if psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
      cd "${OUTPUT_DIR}/migrations"
      chmod +x run.sh
      ./run.sh
      cd - > /dev/null
    fi
  fi
else
  # MySQL/MariaDB
  if [ -f "${OUTPUT_DIR}/backend/.env" ]; then
    # Simple check - try to connect
    cd "${OUTPUT_DIR}/migrations"
    chmod +x run.sh
    if ! ./run.sh 2>&1; then
      echo -e "${YELLOW}⚠ Cannot connect to MySQL server${NC}"
      echo -e "${YELLOW}To start MySQL:${NC}"
      echo -e "  macOS (Homebrew): brew services start mysql"
      echo -e "  Ubuntu:           sudo systemctl start mysql"
      echo -e "  Docker:           docker run -d -p 3306:3306 -e MYSQL_ROOT_PASSWORD=password --name mysql mysql:8"
      echo
      read -p "Continue anyway? (y/n) [y]: " CONTINUE
      CONTINUE=${CONTINUE:-y}

      if [ "$CONTINUE" != "y" ]; then
        echo -e "${RED}Exiting. Please start MySQL and run: ${OUTPUT_DIR}/migrations/run.sh${NC}"
        exit 1
      fi
    fi
    cd - > /dev/null
  fi
fi

# Step 4: Install frontend dependencies
echo -e "\n${GREEN}[4/6] Installing frontend dependencies...${NC}"
cd "${OUTPUT_DIR}/frontend"
npm install --silent
cd - > /dev/null

# Step 5: Install backend dependencies
echo -e "\n${GREEN}[5/6] Installing backend dependencies...${NC}"

# Check for pip/pip3
PIP_CMD=""
if command -v pip3 &> /dev/null; then
  PIP_CMD="pip3"
elif command -v pip &> /dev/null; then
  PIP_CMD="pip"
else
  echo -e "${YELLOW}⚠ pip not found${NC}"
  echo -e "${YELLOW}Please install Python 3 and pip:${NC}"
  echo -e "  macOS:   brew install python3"
  echo -e "  Ubuntu:  sudo apt-get install python3-pip"
  echo -e "  pyenv:   pyenv install 3.11.0 && pyenv global 3.11.0"
  echo
  read -p "Skip backend setup and continue? (y/n) [y]: " SKIP_BACKEND
  SKIP_BACKEND=${SKIP_BACKEND:-y}

  if [ "$SKIP_BACKEND" != "y" ]; then
    echo -e "${RED}Exiting. Please install Python/pip first.${NC}"
    exit 1
  else
    echo -e "${YELLOW}Skipping backend setup. Install manually: cd ${OUTPUT_DIR}/backend && pip install -r requirements.txt${NC}"
  fi
fi

if [ -n "$PIP_CMD" ]; then
  cd "${OUTPUT_DIR}/backend"

  # Create virtual environment if it doesn't exist
  if [ ! -d "venv" ]; then
    echo "Creating Python virtual environment..."
    python3 -m venv venv
  fi

  # Install dependencies in virtual environment
  echo "Installing Python dependencies in virtual environment..."
  source venv/bin/activate
  pip install -q -r requirements.txt
  deactivate

  cd - > /dev/null
fi

# Step 6: Start services
echo -e "\n${GREEN}[6/6] Starting dashboard services...${NC}"
echo -e "${YELLOW}Starting backend and frontend servers...${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}\n"

# Function to kill background processes on exit
cleanup() {
  echo -e "\n\n${YELLOW}Shutting down services...${NC}"
  [ -n "$BACKEND_PID" ] && kill $BACKEND_PID 2>/dev/null && wait $BACKEND_PID 2>/dev/null
  [ -n "$FRONTEND_PID" ] && kill $FRONTEND_PID 2>/dev/null && wait $FRONTEND_PID 2>/dev/null
  echo -e "${GREEN}✓ Services stopped${NC}"
}
trap cleanup EXIT INT TERM

# Start backend
if [ -n "$PIP_CMD" ]; then
  echo -e "${BLUE}Starting backend on http://localhost:8000...${NC}"
  cd "${OUTPUT_DIR}/backend"

  # Start uvicorn using virtual environment
  if [ -d "venv" ]; then
    source venv/bin/activate
    python -m uvicorn main:app --host 0.0.0.0 --port 8000 > /tmp/dashboard-backend.log 2>&1 &
    BACKEND_PID=$!
    deactivate
  elif command -v python3 &> /dev/null; then
    python3 -m uvicorn main:app --host 0.0.0.0 --port 8000 > /tmp/dashboard-backend.log 2>&1 &
    BACKEND_PID=$!
  fi

  cd - > /dev/null

  # Wait for backend to start and check if it's running
  sleep 3

  if [ -n "$BACKEND_PID" ] && kill -0 $BACKEND_PID 2>/dev/null; then
    echo -e "${GREEN}✓ Backend started successfully${NC}"
  else
    echo -e "${RED}✗ Backend failed to start. Check logs: tail -f /tmp/dashboard-backend.log${NC}"
  fi
else
  echo -e "${YELLOW}⚠ Skipping backend startup (Python not configured)${NC}"
  BACKEND_PID=""
fi

# Start frontend
echo -e "${BLUE}Starting frontend on http://localhost:5173...${NC}"
cd "${OUTPUT_DIR}/frontend"
npm run dev > /tmp/dashboard-frontend.log 2>&1 &
FRONTEND_PID=$!
cd - > /dev/null

# Wait for frontend to start
sleep 3

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}  Dashboard is running!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${BLUE}Frontend:${NC}     http://localhost:5173"
if [ -n "$BACKEND_PID" ] && kill -0 $BACKEND_PID 2>/dev/null; then
  echo -e "${BLUE}Backend API:${NC}  http://localhost:8000/docs"
  echo -e "${BLUE}WebSocket:${NC}   ws://localhost:8000/ws"
else
  echo -e "${RED}Backend:${NC}      Not running (check logs below)"
fi
echo -e ""
echo -e "${YELLOW}Logs:${NC}"
echo -e "  Backend:  tail -f /tmp/dashboard-backend.log"
echo -e "  Frontend: tail -f /tmp/dashboard-frontend.log"
echo -e ""

# Check for common backend errors
if [ -f "/tmp/dashboard-backend.log" ]; then
  if grep -q "ModuleNotFoundError\|ImportError\|Error" /tmp/dashboard-backend.log 2>/dev/null; then
    echo -e "${RED}⚠ Backend errors detected:${NC}"
    tail -5 /tmp/dashboard-backend.log | sed 's/^/  /'
    echo -e ""
  fi
fi

echo -e "${YELLOW}To see real-time updates, insert data into your database:${NC}"
if [ "${DATABASE_TYPE}" = "postgresql" ] || [ "${DATABASE_TYPE}" = "postgres" ] || [ "${DATABASE_TYPE}" = "pg" ]; then
  echo -e "  psql \"$DATABASE_URL\" -c \"INSERT INTO orders (order_key, customer_key, order_status, total_price, order_date) VALUES (1, 100, 'O', 123.45, '2025-01-01');\""
else
  echo -e "  mysql -D \$DB_NAME -e \"INSERT INTO orders (order_key, customer_key, order_status, total_price, order_date) VALUES (1, 100, 'O', 123.45, '2025-01-01');\""
fi
echo -e "\n${YELLOW}Press Ctrl+C to stop...${NC}\n"

# Wait for user interrupt
wait
