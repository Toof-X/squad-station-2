import { ReactFlow } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

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
    <div style={{ width: '100vw', height: '100vh' }}>
      <ReactFlow nodes={initialNodes} edges={initialEdges} fitView />
    </div>
  );
}
