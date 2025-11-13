# Auto-generated FastAPI backend for real-time dashboard
# Database type: postgresql

import asyncio
import os
from fastapi import FastAPI, WebSocket
from fastapi.middleware.cors import CORSMiddleware
from databases import Database
from config import ENTITIES

app = FastAPI(title="Real-Time Dashboard API")

# CORS middleware for frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify frontend URL
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Database connection
DATABASE_URL = os.getenv("DATABASE_URL")
database = Database(DATABASE_URL)

# Global state
last_event_id = 0
connected_clients = set()

# Configuration
POLL_INTERVAL = 0.5  # seconds
MAX_EVENTS_PER_POLL = 100

async def poll_events():
    """Background task that polls db_events table"""
    global last_event_id

    while True:
        try:
            # Query for new events since last poll
            query = """
                SELECT id, entity, event_type, payload, created_at
                FROM db_events
                WHERE id > :last_id
                ORDER BY id ASC
                LIMIT :limit
            """

            new_events = await database.fetch_all(
                query,
                {"last_id": last_event_id, "limit": MAX_EVENTS_PER_POLL}
            )

            # Broadcast new events to all connected WebSocket clients
            if new_events:
                for event in new_events:
                    message = {
                        "entity": event["entity"],
                        "event_type": event["event_type"],
                        "data": event["payload"],
                        "timestamp": event["created_at"].isoformat(),
                    }

                    # Broadcast to all connected clients
                    disconnected = set()
                    for client in connected_clients:
                        try:
                            await client.send_json(message)
                        except Exception:
                            disconnected.add(client)

                    # Remove disconnected clients
                    connected_clients.difference_update(disconnected)

                # Update last_event_id to the highest ID we've seen
                last_event_id = new_events[-1]["id"]

            await asyncio.sleep(POLL_INTERVAL)

        except Exception as e:
            print(f"Polling error: {e}")
            await asyncio.sleep(POLL_INTERVAL)


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    connected_clients.add(websocket)

    try:
        # Keep connection alive and handle client messages
        while True:
            # Wait for client messages (ping/pong, filters, etc.)
            data = await websocket.receive_text()
            # Handle client commands if needed
            # For now, just echo back
            # await websocket.send_text(f"Echo: {data}")
    except Exception as e:
        print(f"WebSocket error: {e}")
    finally:
        connected_clients.discard(websocket)


@app.get("/api/stats")
async def get_stats():
    """Get statistics for all entities"""
    stats = {}
    for entity in ENTITIES:
        count = await database.fetch_val(
            f"SELECT COUNT(*) FROM {entity['table']}"
        )
        stats[entity['name']] = {"total": count}
    return stats

@app.get("/api/entities")
async def get_entities():
    """Return entity metadata for frontend"""
    return ENTITIES

@app.get("/api/health")
async def health_check():
    return {"status": "ok", "connected_clients": len(connected_clients)}

@app.on_event("startup")
async def startup():
    await database.connect()
    asyncio.create_task(poll_events())

@app.on_event("shutdown")
async def shutdown():
    await database.disconnect()
