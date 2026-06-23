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
  messages: ChatMsg[];
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
