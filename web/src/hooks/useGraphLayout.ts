import { useMemo } from 'react';
import dagre from '@dagrejs/dagre';
import { Position } from '@xyflow/react';
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
      targetPosition: Position.Top,
      sourcePosition: Position.Bottom,
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
        type: 'animated',
      }));
  }, [agents, orchestrator]);

  // Run dagre layout — only re-runs when agent names change (layoutKey)
  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    return getLayoutedElements(rawNodes, structuralEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layoutKey, structuralEdges]);

  // Update edge animation data from in-flight and recently completed messages
  const edges = useMemo((): Edge[] => {
    const orchName = orchestrator?.name;
    const now = Date.now();
    // Recently completed threshold: show reverse animation for 5 seconds after completion
    const RECENT_MS = 5000;

    return layoutedEdges.map((edge) => {
      // Helper: check if a message matches this edge (orchestrator → worker direction)
      const matchesEdge = (m: WsMessage) => {
        const from = m.from_agent;
        const to = m.to_agent;
        const matchesFrom =
          from === edge.source ||
          (from === 'orchestrator' && edge.source === orchName);
        const matchesTo =
          to === edge.target ||
          (to === 'orchestrator' && edge.target === orchName);
        const matchesReverse =
          (from === edge.target ||
            (from === 'orchestrator' && edge.target === orchName)) &&
          (to === edge.source ||
            (to === 'orchestrator' && edge.source === orchName));
        return (matchesFrom && matchesTo) || matchesReverse;
      };

      // 1. Check for processing message (orchestrator → agent, forward direction)
      const activeMsg = messages.find(
        (m) => m.status === 'processing' && matchesEdge(m),
      );
      if (activeMsg) {
        return {
          ...edge,
          animated: true,
          data: {
            animated: true,
            direction: 'forward' as const,
            task: activeMsg.task,
            priority: activeMsg.priority,
            timestamp: activeMsg.updated_at,
          },
        };
      }

      // 2. Check for recently completed message (agent → orchestrator, reverse direction)
      const recentCompleted = messages.find((m) => {
        if (m.status !== 'completed' || !m.completed_at) return false;
        const completedAt = new Date(m.completed_at).getTime();
        if (isNaN(completedAt)) return false;
        return now - completedAt < RECENT_MS && matchesEdge(m);
      });
      if (recentCompleted) {
        return {
          ...edge,
          animated: true,
          data: {
            animated: true,
            direction: 'reverse' as const,
            task: `✓ ${recentCompleted.task}`,
            priority: recentCompleted.priority,
            timestamp: recentCompleted.completed_at ?? recentCompleted.updated_at,
          },
        };
      }

      return { ...edge, animated: false };
    });
  }, [layoutedEdges, messages, orchestrator]);

  return { nodes: layoutedNodes, edges };
}
