import type { ConnectionState } from '../hooks/useSquadWebSocket';

export function ConnectionStatus({ status }: { status: ConnectionState }) {
  const dotColor =
    status === 'connected'
      ? 'bg-green-500'
      : status === 'disconnected'
        ? 'bg-red-500'
        : 'bg-yellow-400';

  const label =
    status === 'connected'
      ? 'Connected'
      : status === 'disconnected'
        ? 'Disconnected'
        : 'Connecting...';

  return (
    <div className="flex items-center gap-2 text-sm text-gray-300">
      <span className={`inline-block w-2 h-2 rounded-full ${dotColor}`} />
      <span>{label}</span>
    </div>
  );
}
