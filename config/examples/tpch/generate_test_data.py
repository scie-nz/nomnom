#!/usr/bin/env python3
"""
Generate random TPC-H Order records for testing the record_parser binary.

Usage:
    python generate_test_data.py | ./target/debug/record_parser --json-only
    python generate_test_data.py --count 10 | ./target/debug/record_parser --dry-run
"""

import json
import random
import argparse
from datetime import datetime, timedelta

# TPC-H reference data
ORDER_STATUSES = ["O", "F", "P"]  # Open, Fulfilled, Pending
ORDER_PRIORITIES = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"]
RETURN_FLAGS = ["R", "A", "N"]  # Returned, Accepted, None
LINE_STATUSES = ["O", "F"]  # Open, Fulfilled
MARKET_SEGMENTS = ["AUTOMOBILE", "BUILDING", "FURNITURE", "MACHINERY", "HOUSEHOLD"]

def random_date(start_year=2020, end_year=2024):
    """Generate a random date in YYYY-MM-DD format."""
    start = datetime(start_year, 1, 1)
    end = datetime(end_year, 12, 31)
    delta = end - start
    random_days = random.randint(0, delta.days)
    return (start + timedelta(days=random_days)).strftime("%Y-%m-%d")

def generate_line_item(line_number):
    """Generate a single line item."""
    quantity = random.randint(1, 50)
    unit_price = round(random.uniform(10.0, 1000.0), 2)
    extended_price = round(quantity * unit_price, 2)

    return {
        "line_number": line_number,
        "part_key": f"PART-{random.randint(1, 10000)}",
        "supplier_key": f"SUPP-{random.randint(1, 1000)}" if random.random() > 0.1 else None,
        "quantity": quantity,
        "extended_price": extended_price,
        "discount": round(random.uniform(0.0, 0.10), 2) if random.random() > 0.3 else None,
        "tax": round(random.uniform(0.0, 0.08), 2) if random.random() > 0.3 else None,
        "return_flag": random.choice(RETURN_FLAGS) if random.random() > 0.2 else None,
        "line_status": random.choice(LINE_STATUSES) if random.random() > 0.2 else None,
        "ship_date": random_date() if random.random() > 0.2 else None,
        "commit_date": random_date() if random.random() > 0.3 else None,
        "receipt_date": random_date() if random.random() > 0.3 else None,
    }

def generate_order(order_id):
    """Generate a complete order with line items."""
    num_line_items = random.randint(1, 7)
    line_items = [generate_line_item(i + 1) for i in range(num_line_items)]

    # Calculate total price from line items
    total_price = sum(
        item["extended_price"] * (1 - (item.get("discount") or 0)) * (1 + (item.get("tax") or 0))
        for item in line_items
    )

    order = {
        "order_key": f"ORDER-{order_id:06d}",
        "customer_key": f"CUST-{random.randint(1, 1000):04d}",
        "order_status": random.choice(ORDER_STATUSES),
        "total_price": round(total_price, 2),
        "order_date": random_date(),
        "order_priority": random.choice(ORDER_PRIORITIES) if random.random() > 0.2 else None,
        "clerk": f"Clerk#{random.randint(1, 100):03d}" if random.random() > 0.2 else None,
        "ship_priority": random.randint(0, 1) if random.random() > 0.5 else None,
        "comment": f"Order comment {order_id}" if random.random() > 0.5 else None,
        "line_items": line_items,
    }

    return order

def main():
    parser = argparse.ArgumentParser(
        description="Generate random TPC-H Order records for testing"
    )
    parser.add_argument(
        "--count",
        type=int,
        default=5,
        help="Number of orders to generate (default: 5)"
    )
    parser.add_argument(
        "--seed",
        type=int,
        help="Random seed for reproducible output"
    )
    parser.add_argument(
        "--pretty",
        action="store_true",
        help="Pretty-print JSON output"
    )

    args = parser.parse_args()

    if args.seed is not None:
        random.seed(args.seed)

    for i in range(args.count):
        order = generate_order(i + 1)
        if args.pretty:
            print(json.dumps(order, indent=2))
        else:
            print(json.dumps(order))

if __name__ == "__main__":
    main()
