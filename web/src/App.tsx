import { useCallback } from 'react';
import { ReactFlow } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { ConnectionStatus } from './components/ConnectionStatus';
import { StatusBar } from './components/StatusBar';
import { AgentNode } from './components/AgentNode';
import { AnimatedEdge } from './components/AnimatedEdge';
import { ThemeToggle } from './components/ThemeToggle';
import { useSquadWebSocket } from './hooks/useSquadWebSocket';
import { useGraphLayout } from './hooks/useGraphLayout';
import { useTheme } from './hooks/useTheme';

// Defined at module level — CRITICAL: avoids React Flow re-mounting nodes/edges on every render
const nodeTypes = { agent: AgentNode };
const edgeTypes = { animated: AnimatedEdge };

export default function App() {
  const { agents, messages, status } = useSquadWebSocket();
  const { nodes, edges } = useGraphLayout(agents, messages);
  const { theme, toggleTheme } = useTheme();

  const handleContinueAll = useCallback(() => {
    fetch('/api/continue-all', { method: 'POST' })
      .then((res) => res.json())
      .catch((err) => console.error('Continue all failed:', err));
  }, []);

  return (
    <div className="h-screen flex flex-col bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      {/* Top bar: status info + theme toggle + connection indicator */}
      <div className="flex items-center justify-between border-b border-gray-200 dark:border-gray-700">
        <div className="flex-1">
          <StatusBar agentCount={agents.length} agents={agents} onContinueAll={handleContinueAll} />
        </div>
        <div className="px-2 py-2 bg-gray-100 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700">
          <ThemeToggle theme={theme} onToggle={toggleTheme} />
        </div>
        <div className="px-4 py-2 bg-gray-100 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700">
          <ConnectionStatus status={status} />
        </div>
      </div>

      {/* Main area: React Flow graph */}
      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          colorMode={theme}
          fitView
          proOptions={{ hideAttribution: false }}
        />
      </div>
    </div>
  );
}
