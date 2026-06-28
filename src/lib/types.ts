export interface CatalogModel {
  id: string;
  name: string;
  author: string;
  repo: string;
  description: string;
  params: string;
  default_file: string;
  size_gb: number;
  category: string;
  tags: string[];
}

export interface HfModel {
  repo: string;
  author: string;
  name: string;
  downloads: number;
  likes: number;
  tags: string[];
}

/** Use-case topic for intent-based Hub browsing, mirrored from Rust `catalog::Topic`. */
export interface Topic {
  id: string;
  label: string;
  icon: string;
  /** "rich" = el chip busca directo en HF; "niche" = primero generalistas. */
  tier: "rich" | "niche";
  blurb: string;
  hf_queries: string[];
  hf_lang: string;
  recommended_model_ids: string[];
  note: string | null;
}

export interface SystemMemory {
  total_mb: number;
  free_mb: number;
}

export interface HfFile {
  ref: string;
  path: string;
  size: number;
}

export interface LocalModel {
  path: string;
  name: string;
  size_bytes: number;
  size_human: string;
  modified: string;
}

export interface DownloadState {
  id: string;
  repo: string;
  filename: string;
  downloaded: number;
  total: number;
  speed_bps: number;
  status: "Pending" | "Downloading" | "Completed" | { Failed: string } | "Cancelled";
}

export interface Settings {
  temperature: number;
  max_tokens: number;
  top_p: number;
  repeat_penalty: number;
  system_prompt: string;
  context_size: number;
  n_gpu_layers: number;
  threads: number;
  device: string;
  enable_thinking: boolean;
  gpu_layers_auto: boolean;
  cache_type_k: string;
  cache_type_v: string;
  n_batch: number;
  use_mmap: boolean;
  use_mlock: boolean;
  extra_model_dirs: string[];
  /** Estrategia de tool-calling del agente: "auto" | "native" | "grammar". */
  tool_calling: string;
}

export interface GpuDevice {
  id: string;
  name: string;
  total_mb: number;
  free_mb: number;
}

export interface ModelStatus {
  loaded: boolean;
  model: string | null;
  model_name: string | null;
  port: number;
}

export interface LoadProgress {
  model: string;
  model_name: string;
  phase: string;
  percent: number;
  error: string | null;
}

export interface ChatMsg {
  role: string;
  content: string;
}

export interface ToolCall {
  id: string;
  name: string;
  args: any;
}

/** Rich agent conversation message, mirrored from Rust `agent::message::AgentMsg`. */
export interface AgentMsg {
  role: "system" | "user" | "assistant" | "tool";
  content: string;
  tool_name?: string;
  tool_call_id?: string;
  tool_calls?: ToolCall[];
  is_error?: boolean;
  harness?: boolean;
}

export interface Skill {
  slug: string;
  name: string;
  description: string;
  enabled: boolean;
}

export interface ContextFile {
  name: string;
  content: string;
  truncated: boolean;
}

export interface SessionMeta {
  id: string;
  title: string;
  updated: string;
  working_dir: string;
}

export interface StoredSession {
  id: string;
  title: string;
  working_dir: string;
  mode: string;
  created: string;
  updated: string;
  messages: AgentMsg[];
}

export interface ChatTokenEvent {
  session_id: string;
  token: string;
}

export interface ChatDoneEvent {
  session_id: string;
  full: string;
  error: string | null;
}

export interface AppInfo {
  version: string;
  models_dir: string;
  llama_binary: string | null;
  os: string;
  arch: string;
}

export type UpdatePhase =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

export interface UpdateStatus {
  phase: UpdatePhase;
  version: string;
  notes: string;
  percent: number;
  isAppImage: boolean;
  error: string | null;
}
