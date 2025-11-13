#!/bin/bash
# Auto-generated migration runner for dashboard

set -e

echo "Running dashboard migrations..."

psql $DATABASE_URL < 001_create_events_table.sql

echo "✓ Migrations complete!"
