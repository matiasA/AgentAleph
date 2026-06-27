import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  CatalogModel,
  ChatMsg,
  DownloadState,
  GpuDevice,
  HfFile,
  HfModel,
  LoadProgress,
  LocalModel,
  ModelStatus,
  SessionMeta,
  Settings,
  Skill,
  SystemMemory,
  Topic,
  ContextFile,
  StoredSession,
} from "./types";

export const api = {
  listCatalog: () => invoke<CatalogModel[]>("list_catalog_models"),
  listTopics: () => invoke<Topic[]>("list_topics"),
  systemMemory: () => invoke<SystemMemory>("system_memory"),
  searchHf: (q: string) => invoke<HfModel[]>("search_hf", { query: q }),
  browseHf: (sort: string, limit: number) =>
    invoke<HfModel[]>("browse_hf", { sort, limit }),
  listModelFiles: (repo: string) => invoke<HfFile[]>("list_model_files", { repo }),
  listLocal: () => invoke<LocalModel[]>("list_local_models"),
  listModelDirs: () => invoke<string[]>("list_model_dirs"),
  addModelDir: (path: string) => invoke<string[]>("add_model_dir", { path }),
  removeModelDir: (path: string) => invoke<string[]>("remove_model_dir", { path }),
  download: (repo: string, filename: string) =>
    invoke<string>("download_model", { repo, filename }),
  cancelDownload: (id: string) => invoke<void>("cancel_download", { id }),
  deleteModel: (path: string) => invoke<void>("delete_model", { path }),
  loadModel: (path: string) => invoke<void>("load_model", { path }),
  unloadModel: () => invoke<void>("unload_model"),
  modelStatus: () => invoke<ModelStatus>("model_status"),
  sendChat: (sessionId: string, messages: ChatMsg[]) =>
    invoke<void>("send_chat", { sessionId, messages }),
  stopChat: (sessionId: string) => invoke<void>("stop_chat", { sessionId }),
  agentSend: (sessionId: string, workingDir: string, mode: string, input: string) =>
    invoke<void>("agent_send", { sessionId, workingDir, mode, input }),
  agentStop: (sessionId: string) => invoke<void>("agent_stop", { sessionId }),
  respondPermission: (requestId: string, approved: boolean, remember = false) =>
    invoke<void>("respond_permission", { requestId, approved, remember }),
  listAgentSessions: () => invoke<SessionMeta[]>("list_agent_sessions"),
  loadAgentSession: (id: string) =>
    invoke<StoredSession | null>("load_agent_session", { id }),
  deleteAgentSession: (id: string) => invoke<void>("delete_agent_session", { id }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (s: Settings) => invoke<void>("save_settings", { settings: s }),
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  listGpus: () => invoke<GpuDevice[]>("list_gpus"),
  // Skills: specialized instruction packs for the agent.
  listSkills: () => invoke<Skill[]>("list_skills"),
  setSkillEnabled: (slug: string, enabled: boolean) =>
    invoke<void>("set_skill_enabled", { slug, enabled }),
  createSkill: (name: string, description: string, body: string) =>
    invoke<Skill>("create_skill", { name, description, body }),
  importSkill: (folder: string) => invoke<Skill>("import_skill", { folder }),
  deleteSkill: (slug: string) => invoke<void>("delete_skill", { slug }),
  readSkill: (slug: string) => invoke<string>("read_skill", { slug }),
  // Context attached to an agent turn.
  readContextFile: (path: string) => invoke<ContextFile>("read_context_file", { path }),
};

export async function onDownloadProgress(
  cb: (d: DownloadState) => void
): Promise<UnlistenFn> {
  return listen<DownloadState>("download://progress", (e) => cb(e.payload));
}

export async function onChatToken(
  cb: (e: { session_id: string; token: string; is_reasoning: boolean }) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; token: string; is_reasoning: boolean }>("chat://token", (e) => cb(e.payload));
}

export async function onChatDone(
  cb: (e: { session_id: string; full: string; reasoning: string; error: string | null }) => void
): Promise<UnlistenFn> {
  return listen<{ session_id: string; full: string; reasoning: string; error: string | null }>(
    "chat://done",
    (e) => cb(e.payload)
  );
}

export interface AgentTokenEvent {
  session_id: string;
  token: string;
  is_reasoning: boolean;
}
export interface AgentStepEvent {
  session_id: string;
  step: number;
  phase: string;
}
export interface AgentToolEvent {
  session_id: string;
  step: number;
  tool: string;
  args: string;
  result: string;
  is_error: boolean;
}
export interface AgentDoneEvent {
  session_id: string;
  text: string;
  reason: string;
  error: string | null;
}

export async function onAgentToken(
  cb: (e: AgentTokenEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentTokenEvent>("agent://token", (e) => cb(e.payload));
}

export async function onAgentStep(
  cb: (e: AgentStepEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentStepEvent>("agent://step", (e) => cb(e.payload));
}

export async function onAgentTool(
  cb: (e: AgentToolEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentToolEvent>("agent://tool", (e) => cb(e.payload));
}

export async function onAgentDone(
  cb: (e: AgentDoneEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentDoneEvent>("agent://done", (e) => cb(e.payload));
}

export interface AgentPermissionEvent {
  session_id: string;
  request_id: string;
  tool: string;
  args: string;
  summary: string;
}

export async function onAgentPermission(
  cb: (e: AgentPermissionEvent) => void
): Promise<UnlistenFn> {
  return listen<AgentPermissionEvent>("agent://permission", (e) => cb(e.payload));
}

export async function onModelStatus(
  cb: (s: ModelStatus) => void
): Promise<UnlistenFn> {
  return listen<ModelStatus>("model://status", (e) => cb(e.payload));
}

export async function onModelLoading(
  cb: (p: LoadProgress) => void
): Promise<UnlistenFn> {
  return listen<LoadProgress>("model://loading", (e) => cb(e.payload));
}

/** Detected hardware normalized for the model-fit badge. */
export interface Hardware {
  hasGpu: boolean;
  vramFreeMb: number;
  ramFreeMb: number;
}

export type FitLevel = "green" | "amber" | "red" | "unknown";

export interface Fit {
  level: FitLevel;
  /** Short badge label. */
  label: string;
  /** Tooltip detail. */
  detail: string;
}

/**
 * Estimates whether a model fits by comparing required memory against VRAM + RAM.
 * A machine without usable VRAM can still run from RAM, so RAM is included.
 * Green = smooth, amber = slow/partial offload, red = does not fit.
 */
export function modelFit(sizeGb: number, contextSize: number, hw: Hardware | null): Fit {
  if (!hw || (hw.vramFreeMb === 0 && hw.ramFreeMb === 0)) {
    return { level: "unknown", label: "", detail: "" };
  }
  const overheadMb = 512 + (contextSize / 1024) * 128; // weights + approximate KV cache
  const neededMb = sizeGb * 1024 + overheadMb;
  const budgetMb = hw.vramFreeMb + hw.ramFreeMb;
  // An iGPU without dedicated VRAM is not useful for offload: treat it as CPU-only.
  const usableGpu = hw.hasGpu && hw.vramFreeMb >= 1024;

  if (usableGpu) {
    if (neededMb <= hw.vramFreeMb * 0.92) {
      return { level: "green", label: "Smooth on GPU", detail: "Fits entirely in your VRAM." };
    }
    if (neededMb <= budgetMb * 0.85) {
      return {
        level: "amber",
        label: "Will run slow",
        detail: "Does not fit entirely in VRAM: part will spill to RAM/CPU, slower but usable.",
      };
    }
    return { level: "red", label: "Does not fit", detail: "Exceeds your available VRAM + RAM." };
  }
  // No usable GPU: run on CPU from RAM.
  if (neededMb <= hw.ramFreeMb * 0.7) {
    return { level: "green", label: "Good on CPU", detail: "Fits comfortably in your RAM." };
  }
  if (neededMb <= hw.ramFreeMb * 0.92) {
    return { level: "amber", label: "Slow on CPU", detail: "Fits in RAM, but barely; it will be slow." };
  }
  return { level: "red", label: "Does not fit", detail: "Exceeds your available RAM." };
}

/** Detected hardware, readable label, and context size for the fit badge. */
export interface HardwareInfo {
  hardware: Hardware;
  label: string;
  contextSize: number;
}

/**
 * Detects GPUs + free RAM and normalizes them into one VRAM+RAM budget.
 * Shared between Catalog and Local Models to avoid duplicating the calculation.
 */
export async function detectHardware(): Promise<HardwareInfo> {
  const [gpus, mem, s] = await Promise.all([
    api.listGpus().catch(() => [] as GpuDevice[]),
    api.systemMemory().catch(() => ({ total_mb: 0, free_mb: 0 } as SystemMemory)),
    api.getSettings().catch(() => null),
  ]);
  // Sum free VRAM across usable GPUs and discard iGPUs with effectively no VRAM.
  // The fit budget is VRAM + RAM.
  const usableGpus = gpus
    .filter((g) => g.free_mb >= 1024)
    .sort((a, b) => b.free_mb - a.free_mb);
  const vramFreeMb = usableGpus.reduce((sum, g) => sum + g.free_mb, 0);
  const hasGpu = vramFreeMb >= 1024;
  const hardware: Hardware = { hasGpu, vramFreeMb, ramFreeMb: mem.free_mb };

  const ramGb = (mem.free_mb / 1024).toFixed(0);
  let label: string;
  if (hasGpu) {
    const vramGb = (vramFreeMb / 1024).toFixed(1);
    const totalGb = ((vramFreeMb + mem.free_mb) / 1024).toFixed(0);
    const gpuName = usableGpus.length > 1 ? `${usableGpus.length} GPUs` : usableGpus[0].name;
    label = `${gpuName} · ${vramGb} GB VRAM + ${ramGb} GB RAM (${totalGb} GB usable)`;
  } else {
    label = `CPU · ${ramGb} GB free RAM`;
  }

  return { hardware, label, contextSize: s ? s.context_size : 4096 };
}

export function humanSize(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2) + " GB";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + " MB";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + " KB";
  return n + " B";
}

export function humanSpeed(bps: number): string {
  if (bps >= 1e6) return (bps / 1e6).toFixed(2) + " MB/s";
  if (bps >= 1e3) return (bps / 1e3).toFixed(1) + " KB/s";
  return bps + " B/s";
}

export function statusIs(s: DownloadState["status"], v: string): boolean {
  if (typeof s === "string") return s === v;
  return (s as any).Failed !== undefined && v === "Failed";
}

export function statusLabel(s: DownloadState["status"]): string {
  if (typeof s === "string") return s;
  return "Failed";
}

export function statusError(s: DownloadState["status"]): string | null {
  if (typeof s === "object" && s && "Failed" in (s as any)) {
    return (s as any).Failed;
  }
  return null;
}

export function isDone(s: DownloadState["status"]): boolean {
  return (
    s === "Completed" ||
    s === "Cancelled" ||
    (typeof s === "object" && s !== null && "Failed" in (s as any))
  );
}

export function isDownloading(s: DownloadState["status"]): boolean {
  return s === "Pending" || s === "Downloading";
}
