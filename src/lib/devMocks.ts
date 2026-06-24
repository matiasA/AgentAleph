// Arnés de mocks SOLO para inspección visual de la UI en un navegador normal (sin Tauri).
// No se carga nunca dentro de la app Tauri real (ver guarda en main.ts): cuando corre Tauri,
// `window.__TAURI_INTERNALS__` existe y este módulo no se instala.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Settings, StoredSession } from "./types";

const settings: Settings = {
  temperature: 0.7,
  max_tokens: 2048,
  top_p: 0.9,
  repeat_penalty: 1.1,
  system_prompt: "You are a helpful assistant.",
  context_size: 4096,
  n_gpu_layers: 99,
  threads: 8,
  device: "auto",
  enable_thinking: false,
  gpu_layers_auto: true,
  cache_type_k: "f16",
  cache_type_v: "f16",
  n_batch: 2048,
  use_mmap: true,
  use_mlock: false,
  extra_model_dirs: ["/home/matias/.local/share/agent-aleph/models"],
  tool_calling: "auto",
};

// Sesión nativa de ejemplo: ejercita el branch de tool_calls del asistente + rol `tool`
// en `AgentView.reconstruct`.
const nativeSession: StoredSession = {
  id: "demo-native",
  title: "Demo nativa (mock)",
  working_dir: "/proj",
  mode: "build",
  created: "2026-06-24T00:00:00Z",
  updated: "2026-06-24T00:00:00Z",
  messages: [
    { role: "system", content: "Eres un agente de programación." },
    { role: "user", content: "¿Qué dice la primera línea del README.md?" },
    {
      role: "assistant",
      content: "",
      tool_calls: [{ id: "call_0", name: "read_file", args: { path: "README.md" } }],
    },
    {
      role: "tool",
      tool_name: "read_file",
      tool_call_id: "call_0",
      content: "# Agent Aleph\nUn agente de codificación 100% local.",
    },
    {
      role: "assistant",
      content: "La primera línea del README.md es: «# Agent Aleph».",
    },
  ],
};

let skills = [
  { slug: "revisor-rust", name: "Revisor Rust", description: "Checklist y convenciones para revisar código Rust.", enabled: true },
  { slug: "convenciones-svelte", name: "Convenciones Svelte 5", description: "Runes, $state/$props y patrones del proyecto.", enabled: false },
];

export function installDevMocks() {
  mockWindows("main");
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "get_settings":
        return settings;
      case "save_settings":
        Object.assign(settings, (args as any)?.settings ?? {});
        return;
      case "model_status":
        return { loaded: true, model: "/models/qwen.gguf", model_name: "Qwen3.5-0.8B", port: 8099 };
      case "get_app_info":
        return {
          version: "0.1.0",
          models_dir: "/home/matias/.local/share/agent-aleph/models",
          llama_binary: "/bin/llama-server",
          os: "linux",
          arch: "x86_64",
        };
      case "list_gpus":
        return [];
      case "list_agent_sessions":
        return [
          { id: "demo-native", title: "Demo nativa (mock)", updated: "2026-06-24T00:00:00Z", working_dir: "/proj" },
        ];
      case "load_agent_session":
        return nativeSession;
      case "delete_agent_session":
        return;
      case "list_local_models":
      case "list_catalog_models":
      case "browse_hf":
      case "search_hf":
      case "list_model_files":
        return [];
      case "list_model_dirs":
        return ["/home/matias/.local/share/agent-aleph/models"];
      case "list_skills":
        return skills;
      case "set_skill_enabled": {
        const a = args as any;
        const sk = skills.find((s) => s.slug === a.slug);
        if (sk) sk.enabled = a.enabled;
        return;
      }
      case "create_skill": {
        const a = args as any;
        const sk = { slug: `nueva-${skills.length}`, name: a.name, description: a.description, enabled: false };
        skills.push(sk);
        return sk;
      }
      case "delete_skill": {
        const a = args as any;
        skills = skills.filter((s) => s.slug !== a.slug);
        return;
      }
      case "read_context_file":
        return { name: "ejemplo.txt", content: "contenido de ejemplo del archivo adjunto", truncated: false };
      default:
        // Plugins de eventos/ventana de Tauri: devolver algo inocuo para no romper listen().
        if (cmd.startsWith("plugin:")) return 0;
        return undefined;
    }
  });
  // eslint-disable-next-line no-console
  console.log("[devMocks] IPC de Tauri mockeado para inspección en navegador");
}
