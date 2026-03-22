import { useEffect, useState } from 'react';

type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export function ConnectionStatus() {
  const [status, setStatus] = useState<ConnectionState>('connecting');

  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      setStatus('connecting');
      const url = `ws://${window.location.host}/ws`;
      ws = new WebSocket(url);

      ws.onopen = () => {
        setStatus('connected');
      };

      ws.onclose = () => {
        setStatus('disconnected');
        reconnectTimer = setTimeout(connect, 3000);
      };

      ws.onerror = () => {
        setStatus('disconnected');
        ws?.close();
      };
    }

    connect();

    return () => {
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (ws) {
        ws.onclose = null;
        ws.close();
      }
    };
  }, []);

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
