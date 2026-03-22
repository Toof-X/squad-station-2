import { useMemo } from 'react';
import dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';
import type { Agent, WsMessage } from './useSquadWebSocket';
import type { AgentNodeData } from '../components/AgentNode';

const NODE_WIDTH = 150;
const NODE_HEIGHT = 60;

export type { AgentNodeData };

function getLayoutedElements(
  nodes: Node<AgentNodeData, 'agent'>[],
  edges: Edge[],
): { nodes: Node<AgentNodeData, 'agent'>[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: 'TB', ranksep: 120, nodesep: 80 });

  for (const node of nodes) {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }

  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  dagre.layout(g);

  const layoutedNodes = nodes.map((node) => {
    const pos = g.node(node.id);
    return {
      ...node,
      position: { x: pos.x - NODE_WIDTH / 2, y: pos.y - NODE_HEIGHT / 2 },
      targetPosition: 'top' as const,
      sourcePosition: 'bottom' as const,
    };
  });

  return { nodes: layoutedNodes, edges };
}

function detectOrchestrator(agents: Agent[], messages: WsMessage[]): Agent | undefined {
  // Primary: role contains "orchestrator" (case-insensitive)
  const byRole = agents.find((a) => a.role.toLowerCase().includes('orchestrator'));
  if (byRole) return byRole;

  // Fallback: agent appearing most as from_agent in messages
  if (messages.length > 0) {
    const counts: Record<string, number> = {};
    for (const msg of messages) {
      if (msg.from_agent) {
        counts[msg.from_agent] = (counts[msg.from_agent] ?? 0) + 1;
      }
    }
    let maxName: string | undefined;
    let maxCount = 0;
    for (const [name, count] of Object.entries(counts)) {
      if (count > maxCount) {
        maxCount = count;
        maxName = name;
      }
    }
    if (maxName) {
      const byFrequency = agents.find((a) => a.name === maxName);
      if (byFrequency) return byFrequency;
    }
  }

  // Final fallback: first agent
  return agents[0];
}

export function useGraphLayout(
  agents: Agent[],
  messages: WsMessage[],
): { nodes: Node<AgentNodeData, 'agent'>[]; edges: Edge[] } {
  const orchestrator = useMemo(
    () => detectOrchestrator(agents, messages),
    // Recompute orchestrator detection only when agents or message count changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [agents, messages.length],
  );

  // Key for layout: only agent names — avoid re-layout on status-only changes
  const layoutKey = useMemo(
    () => agents.map((a) => a.name).sort().join(','),
    [agents],
  );

  // Build nodes (without layout) — keyed on full agent data for status updates
  const rawNodes = useMemo((): Node<AgentNodeData, 'agent'>[] => {
    return agents.map((agent) => ({
      id: agent.name,
      type: 'agent' as const,
      position: { x: 0, y: 0 }, // will be overwritten by layout
      data: {
        name: agent.name,
        role: agent.role,
        status: agent.status,
        model: agent.model,
        description: agent.description,
        currentTask: agent.current_task,
        isOrchestrator: orchestrator?.name === agent.name,
      },
    }));
  }, [agents, orchestrator]);

  // Build structural edges (orchestrator -> each worker)
  const structuralEdges = useMemo((): Edge[] => {
    if (!orchestrator) return [];
    return agents
      .filter((a) => a.name !== orchestrator.name)
      .map((worker) => ({
        id: `e-${orchestrator.name}-${worker.name}`,
        source: orchestrator.name,
        target: worker.name,
        type: 'default',
      }));
  }, [agents, orchestrator]);

  // Run dagre layout — only re-runs when agent names change (layoutKey)
  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    return getLayoutedElements(rawNodes, structuralEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layoutKey, structuralEdges]);

  // Update edge animation data from in-flight messages (separate memo — no re-layout)
  const edges = useMemo((): Edge[] => {
    return layoutedEdges.map((edge) => {
      // Find a processing message between source and target (in either direction)
      const activeMsg = messages.find(
        (m) =>
          m.status === 'processing' &&
          ((m.from_agent === edge.source && m.to_agent === edge.target) ||
            (m.from_agent === edge.target && m.to_agent === edge.source)),
      );
      if (activeMsg) {
        return {
          ...edge,
          animated: true,
          data: {
            animated: true,
            task: activeMsg.task,
            priority: activeMsg.priority,
            timestamp: activeMsg.updated_at,
          },
        };
      }
      return { ...edge, animated: false };
    });
  }, [layoutedEdges, messages]);

  return { nodes: layoutedNodes, edges };
}
