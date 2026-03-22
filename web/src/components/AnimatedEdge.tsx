import { useState } from 'react';
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath } from '@xyflow/react';
import type { Edge, EdgeProps } from '@xyflow/react';

type AnimatedEdgeData = {
  animated: boolean;
  task?: string;
  priority?: string;
  timestamp?: string;
};

type MessageEdge = Edge<AnimatedEdgeData, 'animated'>;

const priorityColors: Record<string, string> = {
  urgent: 'bg-red-500 text-white',
  high: 'bg-orange-500 text-white',
  normal: 'bg-gray-500 text-white',
};

function getPriorityClass(priority: string | undefined): string {
  if (!priority) return 'bg-gray-500 text-white';
  return priorityColors[priority.toLowerCase()] ?? 'bg-gray-500 text-white';
}

function formatRelativeTime(timestamp: string | undefined): string | null {
  if (!timestamp) return null;
  const date = new Date(timestamp);
  if (isNaN(date.getTime())) return null;
  const diffMs = Date.now() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  if (diffMins < 1) return 'just now';
  if (diffHours < 1) return `${diffMins}m ago`;
  return `${diffHours}h ago`;
}

export function AnimatedEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
}: EdgeProps<MessageEdge>) {
  const [hovered, setHovered] = useState(false);

  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const isAnimated = data?.animated === true;
  const hasLabel = Boolean(data?.task) && hovered;
  const taskText = data?.task ? (data.task.length > 30 ? data.task.slice(0, 30) + '…' : data.task) : '';
  const priorityClass = getPriorityClass(data?.priority);
  const relativeTime = formatRelativeTime(data?.timestamp);

  return (
    <>
      <BaseEdge id={id} path={edgePath} />

      {/* Invisible wider hit area for hover detection */}
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={20}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={{ pointerEvents: 'stroke' }}
      />

      {/* Crawling dots animation — only when animated, no pointer events */}
      {isAnimated && (
        <g style={{ pointerEvents: 'none' }}>
          <circle r="3" fill="#3b82f6">
            <animateMotion dur="2s" repeatCount="indefinite" path={edgePath} begin="0s" />
          </circle>
          <circle r="3" fill="#3b82f6">
            <animateMotion dur="2s" repeatCount="indefinite" path={edgePath} begin="0.66s" />
          </circle>
          <circle r="3" fill="#3b82f6">
            <animateMotion dur="2s" repeatCount="indefinite" path={edgePath} begin="1.33s" />
          </circle>
        </g>
      )}

      {/* Edge label — only on hover when in-flight task exists */}
      {hasLabel && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: 'all',
            }}
            className="nodrag nopan bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-xs shadow-md"
          >
            <div className="text-gray-800 dark:text-gray-200 font-medium leading-tight">{taskText}</div>
            <div className="flex items-center gap-1 mt-0.5">
              {data?.priority && (
                <span className={`px-1 rounded text-[10px] font-medium ${priorityClass}`}>
                  {data.priority}
                </span>
              )}
              {relativeTime && (
                <span className="text-gray-500 dark:text-gray-400 text-[10px]">{relativeTime}</span>
              )}
            </div>
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
