import { ReactFlow } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { ConnectionStatus } from './components/ConnectionStatus';
import { StatusBar } from './components/StatusBar';

const initialNodes: Node[] = [
  { id: '1', position: { x: 250, y: 0 }, data: { label: 'Orchestrator' }, type: 'default' },
  { id: '2', position: { x: 100, y: 150 }, data: { label: 'Worker 1' }, type: 'default' },
  { id: '3', position: { x: 400, y: 150 }, data: { label: 'Worker 2' }, type: 'default' },
];

const initialEdges: Edge[] = [
  { id: 'e1-2', source: '1', target: '2', animated: true },
  { id: 'e1-3', source: '1', target: '3' },
];

export default function App() {
  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      {/* Top bar: status info + connection indicator */}
      <div className="flex items-center justify-between border-b border-gray-700">
        <div className="flex-1">
          <StatusBar />
        </div>
        <div className="px-4 py-2 bg-gray-800 border-l border-gray-700">
          <ConnectionStatus />
        </div>
      </div>

      {/* Main area: React Flow graph */}
      <div className="flex-1">
        <ReactFlow nodes={initialNodes} edges={initialEdges} fitView />
      </div>
    </div>
  );
}
