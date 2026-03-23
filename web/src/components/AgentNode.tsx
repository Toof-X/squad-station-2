import { memo, useState, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { Handle, Position } from '@xyflow/react';
import type { Node, NodeProps } from '@xyflow/react';

export interface AgentNodeData extends Record<string, unknown> {
  name: string;
  role: string;
  status: string;
  model: string | null;
  description: string | null;
  currentTask: string | null;
  isOrchestrator: boolean;
}

export type AgentNodeType = Node<AgentNodeData, 'agent'>;

const statusColors: Record<string, string> = {
  busy: 'bg-green-500',
  idle: 'bg-gray-400',
  dead: 'bg-red-500',
  waiting_confirm: 'bg-yellow-400',
};

function getStatusColor(status: string): string {
  return statusColors[status.toLowerCase()] ?? 'bg-gray-400';
}

export const AgentNode = memo(function AgentNode({ data }: NodeProps<AgentNodeType>) {
  const dotColor = getStatusColor(data.status);
  const icon = data.isOrchestrator ? '★' : '⚙';

  const [showPopup, setShowPopup] = useState(false);
  const [popupPos, setPopupPos] = useState({ x: 0, y: 0 });

  const handleMouseEnter = useCallback((e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setPopupPos({
      x: rect.left + rect.width / 2,
      y: rect.bottom + 8,
    });
    setShowPopup(true);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setShowPopup(false);
  }, []);

  return (
    <div
      className="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 shadow-md min-w-[140px] text-center relative"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* Target handle (hidden for orchestrator — no incoming edges) */}
      <Handle
        type="target"
        position={Position.Top}
        className={data.isOrchestrator ? '!w-0 !h-0 !min-w-0 !min-h-0 !border-0' : ''}
      />

      {/* Status dot */}
      <div className={`absolute top-2 right-2 w-2.5 h-2.5 rounded-full ${dotColor}`} />

      {/* Icon */}
      <div className="text-lg mb-0.5">{icon}</div>

      {/* Name */}
      <div className="text-sm font-semibold text-gray-900 dark:text-gray-100 leading-tight">{data.name}</div>

      {/* Role */}
      <div className="text-xs text-gray-500 dark:text-gray-400 leading-tight mt-0.5">{data.role}</div>

      {/* Source handle */}
      <Handle type="source" position={Position.Bottom} />

      {/* Popup via portal — rendered on document.body top layer */}
      {showPopup && createPortal(
        <div
          style={{
            position: 'fixed',
            left: popupPos.x,
            top: popupPos.y,
            transform: 'translateX(-50%)',
            zIndex: 99999,
          }}
          className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-md p-3 text-xs text-gray-700 dark:text-gray-300 shadow-xl min-w-[200px] max-w-[320px] text-left pointer-events-none break-words"
        >
          <div className="mb-1">
            <span className="text-gray-500">Model: </span>
            {data.model ?? 'unknown'}
          </div>
          <div className="mb-1">
            <span className="text-gray-500">Status: </span>
            {data.status}
          </div>
          {data.currentTask && (
            <div className="mb-1">
              <span className="text-gray-500">Task: </span>
              {data.currentTask}
            </div>
          )}
          {data.description && (
            <div>
              <span className="text-gray-500">Description: </span>
              {data.description}
            </div>
          )}
        </div>,
        document.body
      )}
    </div>
  );
});
