import { useEffect, useState } from 'react';

export interface Agent {
  id: string;
  name: string;
  tool: string;
  role: string;
  status: string;
  status_updated_at: string;
  model: string | null;
  description: string | null;
  current_task: string | null;
  routing_hints: string | null;
}

export interface WsMessage {
  id: string;
  agent_name: string;
  from_agent: string | null;
  to_agent: string | null;
  msg_type: string;
  task: string;
  status: string;
  priority: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  thread_id: string | null;
}

export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export function useSquadWebSocket() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [messages, setMessages] = useState<WsMessage[]>([]);
  const [status, setStatus] = useState<ConnectionState>('connecting');

  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      setStatus('connecting');
      ws = new WebSocket(`ws://${window.location.host}/ws`);

      ws.onopen = () => setStatus('connected');

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data as string);
          switch (data.type) {
            case 'snapshot':
              setAgents(data.agents as Agent[]);
              setMessages(data.messages as WsMessage[]);
              break;
            case 'agent_update':
              setAgents(data.agents as Agent[]);
              break;
            case 'message_update':
              setMessages(data.messages as WsMessage[]);
              break;
          }
        } catch {
          // Ignore malformed messages
        }
      };

      ws.onclose = () => {
        setStatus('disconnected');
        // Wipe state on disconnect — on reconnect, fresh snapshot replaces all state
        setAgents([]);
        setMessages([]);
        reconnectTimer = setTimeout(connect, 3000);
      };

      ws.onerror = () => ws?.close();
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

  return { agents, messages, status };
}
