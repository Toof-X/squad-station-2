import { useEffect, useState } from 'react';

interface StatusData {
  project: string;
  agents: number;
  uptime_secs: number;
  version: string;
}

function formatUptime(secs: number): string {
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  return `${minutes}m ${seconds}s`;
}

export function StatusBar() {
  const [status, setStatus] = useState<StatusData | null>(null);

  useEffect(() => {
    function fetchStatus() {
      fetch('/api/status')
        .then((res) => res.json())
        .then((data: StatusData) => setStatus(data))
        .catch(() => {
          // silently ignore fetch errors — server may not be ready yet
        });
    }

    fetchStatus();
    const interval = setInterval(fetchStatus, 10000);

    return () => clearInterval(interval);
  }, []);

  if (!status) {
    return (
      <div className="flex items-center px-4 py-2 bg-gray-800 text-gray-400 text-sm">
        Loading...
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-4 py-2 bg-gray-800 text-white text-sm">
      <div className="flex items-center gap-6">
        <span className="font-semibold text-gray-100">{status.project}</span>
        <span className="text-gray-400">
          {status.agents} agent{status.agents !== 1 ? 's' : ''}
        </span>
        <span className="text-gray-400">up {formatUptime(status.uptime_secs)}</span>
      </div>
      <span className="text-gray-500 text-xs">v{status.version}</span>
    </div>
  );
}
