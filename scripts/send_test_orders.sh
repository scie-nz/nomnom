#!/bin/bash

# Script to send test orders with random data to the ingestion API
# Usage: ./send_test_orders.sh [number_of_orders] [api_url]

set -e

NUM_ORDERS=${1:-5}
API_URL=${2:-"http://localhost:8888/ingest/message"}

echo "Sending $NUM_ORDERS test orders to $API_URL"
echo "=========================================="

# Arrays for random data generation
CUSTOMERS=("Alice Johnson" "Bob Smith" "Charlie Brown" "Diana Prince" "Eve Davis" "Frank Miller" "Grace Lee" "Henry Ford" "Iris West" "Jack Ryan")
STATUSES=("pending" "processing" "shipped" "delivered" "cancelled")
PRODUCTS=("Widget A" "Gadget B" "Doohickey C" "Thingamajig D" "Whatchamacallit E" "Gizmo F" "Contraption G" "Device H")

# Function to generate random float between min and max
random_float() {
    local min=$1
    local max=$2
    awk -v min="$min" -v max="$max" 'BEGIN{srand(); printf "%.2f\n", min+rand()*(max-min)}'
}

# Function to generate random int between min and max
random_int() {
    local min=$1
    local max=$2
    echo $(( RANDOM % (max - min + 1) + min ))
}

# Function to get random element from array
random_element() {
    local arr=("$@")
    local idx=$(random_int 0 $((${#arr[@]} - 1)))
    echo "${arr[$idx]}"
}

# Generate and send orders
for i in $(seq 1 $NUM_ORDERS); do
    ORDER_ID=$(uuidgen)
    CUSTOMER=$(random_element "${CUSTOMERS[@]}")
    STATUS=$(random_element "${STATUSES[@]}")
    ORDER_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # Generate random number of line items (2-5)
    NUM_LINE_ITEMS=$(random_int 2 5)

    # Build line items JSON array
    LINE_ITEMS_JSON="["
    TOTAL_AMOUNT=0

    for j in $(seq 1 $NUM_LINE_ITEMS); do
        PART_KEY="P$(random_int 1000 9999)"  # Generate part key like P1234
        QUANTITY=$(random_int 1 10)
        UNIT_PRICE=$(random_float 10.00 500.00)
        EXTENDED_PRICE=$(awk -v q="$QUANTITY" -v p="$UNIT_PRICE" 'BEGIN{printf "%.2f", q*p}')
        TOTAL_AMOUNT=$(awk -v t="$TOTAL_AMOUNT" -v s="$EXTENDED_PRICE" 'BEGIN{printf "%.2f", t+s}')

        if [ $j -gt 1 ]; then
            LINE_ITEMS_JSON+=","
        fi

        # TPC-H schema: line_number, part_key, quantity, extended_price
        LINE_ITEMS_JSON+="{\"line_number\":$j,\"part_key\":\"$PART_KEY\",\"quantity\":$QUANTITY,\"extended_price\":$EXTENDED_PRICE}"
    done

    LINE_ITEMS_JSON+="]"

    # Build the complete order JSON (raw Order format - no envelope)
    # Using TPC-H schema field names: order_key, customer_key, order_status, total_price
    ORDER_JSON=$(cat <<EOF
{
    "order_key": "$ORDER_ID",
    "customer_key": "$CUSTOMER",
    "order_date": "$ORDER_DATE",
    "order_status": "$STATUS",
    "total_price": $TOTAL_AMOUNT,
    "line_items": $LINE_ITEMS_JSON
}
EOF
)
echo $ORDER_JSON | jq
    echo ""
    echo "Order $i/$NUM_ORDERS:"
    echo "  Customer: $CUSTOMER"
    echo "  Status: $STATUS"
    echo "  Line Items: $NUM_LINE_ITEMS"
    echo "  Total: \$$TOTAL_AMOUNT"

    # Send the order
    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL" \
        -H "Content-Type: application/json" \
        -d "$ORDER_JSON")

    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | sed '$d')

    if [ "$HTTP_CODE" = "200" ]; then
        echo "  ✓ Sent successfully"
    else
        echo "  ✗ Failed (HTTP $HTTP_CODE)"
        echo "  Response: $BODY"
    fi

    # Small delay between requests
    sleep 0.5
done

echo ""
echo "=========================================="
echo "Finished sending $NUM_ORDERS orders!"
echo ""
echo "Check the dashboard at http://localhost:8888 to see the real-time updates"
