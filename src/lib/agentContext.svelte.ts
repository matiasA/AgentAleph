// Shared state for context attached to the next agent turn.
// Edited by AgentPanel and consumed by AgentView when sending.

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

/** Text block with attached context to prepend to the turn input. */
export function buildContextBlock(): string {
  if (!agentContext.items.length) return "";
  let s = "User-attached context. Consider it when working on the task:\n";
  for (const it of agentContext.items) {
    const tag = it.kind === "file" ? `file: ${it.label}` : it.label;
    const trunc = it.truncated ? "\n[... truncated ...]" : "";
    s += `\n----- ${tag} -----\n${it.content}${trunc}\n`;
  }
  return s + "\n";
}
