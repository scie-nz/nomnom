// Auto-generated EntityCard component
import { Entity } from '../generated/entities';
import { EntityRecord } from '../hooks/useRealtimeData';

interface EntityCardProps {
  entity: Entity;
  records: EntityRecord[];
}

export function EntityCard({ entity, records }: EntityCardProps) {
  return (
    <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
      <div className="flex items-center gap-3 mb-4">
        <span className="text-3xl">{entity.icon}</span>
        <div>
          <h2 className="text-2xl font-bold">{entity.name}</h2>
          <p className="text-sm text-gray-500">Table: {entity.table}</p>
        </div>
        <div className="ml-auto">
          <span className="text-sm font-semibold text-gray-700">
            {records.length} records
          </span>
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                Time
              </th>
              {entity.fields.map(field => (
                <th
                  key={field}
                  className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase"
                >
                  {field}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {records.slice(0, 10).map((record, idx) => (
              <tr
                key={idx}
                className="hover:bg-gray-50 transition-colors"
                style={{
                  borderLeft: `4px solid ${entity.color}`,
                }}
              >
                <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-500">
                  {new Date(record.timestamp).toLocaleTimeString()}
                </td>
                {entity.fields.map(field => (
                  <td
                    key={field}
                    className="px-4 py-3 whitespace-nowrap text-sm text-gray-900"
                  >
                    {String(record.data[field] ?? '-')}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {records.length === 0 && (
          <div className="text-center py-8 text-gray-500">
            No records yet. Waiting for data...
          </div>
        )}
      </div>
    </div>
  );
}
