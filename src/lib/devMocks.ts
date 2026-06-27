// Mock harness ONLY for visual UI inspection in a normal browser without Tauri.
// It never loads inside the real Tauri app: when Tauri runs, `window.__TAURI_INTERNALS__`
// exists and this module is not installed.
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

// Example native session: exercises assistant tool_calls + tool role in AgentView.reconstruct.
const nativeSession: StoredSession = {
  id: "demo-native",
  title: "Native demo (mock)",
  working_dir: "/proj",
  mode: "build",
  created: "2026-06-24T00:00:00Z",
  updated: "2026-06-24T00:00:00Z",
  messages: [
    { role: "system", content: "You are a coding agent." },
    { role: "user", content: "What does the first line of README.md say?" },
    {
      role: "assistant",
      content: "",
      tool_calls: [{ id: "call_0", name: "read_file", args: { path: "README.md" } }],
    },
    {
      role: "tool",
      tool_name: "read_file",
      tool_call_id: "call_0",
      content: "# Agent Aleph\nA 100% local AI coding agent.",
    },
    {
      role: "assistant",
      content: "The first line of README.md is: \"# Agent Aleph\".",
    },
  ],
};

let skills = [
  { slug: "rust-reviewer", name: "Rust Reviewer", description: "Checklist and conventions for reviewing Rust code.", enabled: true },
  { slug: "svelte-conventions", name: "Svelte 5 Conventions", description: "Runes, $state/$props, and project patterns.", enabled: false },
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
          version: "0.2.0-beta.5",
          models_dir: "/home/matias/.local/share/agent-aleph/models",
          llama_binary: "/bin/llama-server",
          os: "linux",
          arch: "x86_64",
        };
      case "list_gpus":
        return [];
      case "list_agent_sessions":
        return [
          { id: "demo-native", title: "Native demo (mock)", updated: "2026-06-24T00:00:00Z", working_dir: "/proj" },
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
        const sk = { slug: `new-${skills.length}`, name: a.name, description: a.description, enabled: false };
        skills.push(sk);
        return sk;
      }
      case "delete_skill": {
        const a = args as any;
        skills = skills.filter((s) => s.slug !== a.slug);
        return;
      }
      case "read_context_file":
        return { name: "example.txt", content: "example attached file content", truncated: false };
      default:
        // Tauri event/window plugins: return a harmless value so listen() does not break.
        if (cmd.startsWith("plugin:")) return 0;
        return undefined;
    }
  });
  // eslint-disable-next-line no-console
  console.log("[devMocks] Tauri IPC mocked for browser inspection");
}
