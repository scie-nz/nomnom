// Auto-generated Dashboard component
import { ENTITIES } from '../generated/entities';
import { useRealtimeData } from '../hooks/useRealtimeData';
import { EntityCard } from './EntityCard';

export function Dashboard() {
  const { records, connected, error } = useRealtimeData();

  return (
    <div className="min-h-screen bg-gray-100">
      <header className="bg-white shadow-md">
        <div className="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between">
            <h1 className="text-3xl font-bold text-gray-900">
              Real-Time Dashboard
            </h1>
            <div className="flex items-center gap-2">
              <div
                className={`w-3 h-3 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'}`}
              />
              <span className="text-sm font-medium text-gray-700">
                {connected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
        {error && (
          <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error}
          </div>
        )}

        {ENTITIES.map(entity => (
          <EntityCard
            key={entity.name}
            entity={entity}
            records={records.get(entity.name) || []}
          />
        ))}
      </main>
    </div>
  );
}
