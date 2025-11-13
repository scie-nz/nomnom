// Auto-generated WebSocket hook for real-time data
import { useState, useEffect, useRef } from 'react';
import { ENTITIES } from '../generated/entities';

export interface EntityRecord {
  entity: string;
  data: Record<string, any>;
  timestamp: string;
}

export interface RealtimeData {
  records: Map<string, EntityRecord[]>;
  connected: boolean;
  error: string | null;
}

const BACKEND_URL = 'ws://localhost:3000/ws';

export function useRealtimeData(): RealtimeData {
  const [records, setRecords] = useState<Map<string, EntityRecord[]>>(new Map());
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Initialize records map with empty arrays for each entity
    const initialRecords = new Map<string, EntityRecord[]>();
    ENTITIES.forEach(entity => {
      initialRecords.set(entity.name, []);
    });
    setRecords(initialRecords);

    // Connect to WebSocket
    const ws = new WebSocket(BACKEND_URL);

    ws.onopen = () => {
      console.log('WebSocket connected');
      setConnected(true);
      setError(null);
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        const { entity, data, timestamp } = message;

        // Add new record to the appropriate entity
        setRecords(prev => {
          const newRecords = new Map(prev);
          const entityRecords = newRecords.get(entity) || [];
          const entityConfig = ENTITIES.find(e => e.name === entity);

          // Add new record at the beginning (most recent first)
          const updatedRecords = [{ entity, data, timestamp }, ...entityRecords];

          // Cap at maxRecords
          const maxRecords = entityConfig?.maxRecords || 500;
          const cappedRecords = updatedRecords.slice(0, maxRecords);

          newRecords.set(entity, cappedRecords);
          return newRecords;
        });
      } catch (err) {
        console.error('Failed to parse WebSocket message:', err);
      }
    };

    ws.onerror = (event) => {
      console.error('WebSocket error:', event);
      setError('WebSocket connection error');
      setConnected(false);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setConnected(false);
    };

    wsRef.current = ws;

    // Cleanup on unmount
    return () => {
      ws.close();
    };
  }, []);

  return { records, connected, error };
}
