// Estado compartido del contexto adjunto al turno del agente.
// Lo edita el panel derecho (AgentPanel) y lo consume el envío del turno (AgentView).

export interface ContextItem {
  id: string;
  kind: "file" | "text";
  label: string;
  content: string;
  truncated?: boolean;
}

export const agentContext = $state<{ items: ContextItem[] }>({ items: [] });

export function addContextItem(item: ContextItem) {
  agentContext.items.push(item);
}

export function removeContextItem(id: string) {
  agentContext.items = agentContext.items.filter((i) => i.id !== id);
}

export function clearContext() {
  agentContext.items = [];
}

/** Bloque de texto con el contexto adjunto, para anteponer al input del turno. */
export function buildContextBlock(): string {
  if (!agentContext.items.length) return "";
  let s = "Contexto adjunto por el usuario (tenelo en cuenta para la tarea):\n";
  for (const it of agentContext.items) {
    const tag = it.kind === "file" ? `archivo: ${it.label}` : it.label;
    const trunc = it.truncated ? "\n[… truncado …]" : "";
    s += `\n----- ${tag} -----\n${it.content}${trunc}\n`;
  }
  return s + "\n";
}
