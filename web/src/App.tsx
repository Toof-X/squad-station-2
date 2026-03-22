import { ReactFlow } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { ConnectionStatus } from './components/ConnectionStatus';
import { StatusBar } from './components/StatusBar';
import { AgentNode } from './components/AgentNode';
import { useSquadWebSocket } from './hooks/useSquadWebSocket';
import { useGraphLayout } from './hooks/useGraphLayout';

// Defined at module level — CRITICAL: avoids React Flow re-mounting nodes on every render
const nodeTypes = { agent: AgentNode };

export default function App() {
  const { agents, messages, status } = useSquadWebSocket();
  const { nodes, edges } = useGraphLayout(agents, messages);

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      {/* Top bar: status info + connection indicator */}
      <div className="flex items-center justify-between border-b border-gray-700">
        <div className="flex-1">
          <StatusBar agentCount={agents.length} />
        </div>
        <div className="px-4 py-2 bg-gray-800 border-l border-gray-700">
          <ConnectionStatus status={status} />
        </div>
      </div>

      {/* Main area: React Flow graph */}
      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          proOptions={{ hideAttribution: false }}
        />
      </div>
    </div>
  );
}
