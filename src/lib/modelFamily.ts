export interface ModelFamily {
  key: string;
  label: string;
  color: string;
  initials: string;
  /** Clave de marca real disponible (ver brandMarks.ts); si falta, se usan las iniciales. */
  logo?: "meta" | "mistralai" | "deepseek" | "qwen" | "google";
}

const FAMILIES: { match: RegExp; family: ModelFamily }[] = [
  { match: /gemma/i, family: { key: "gemma", label: "Gemma · Google", color: "#4285F4", initials: "Ge", logo: "google" } },
  { match: /code\s*llama/i, family: { key: "codellama", label: "Code Llama · Meta", color: "#0668E1", initials: "CL", logo: "meta" } },
  { match: /tinyllama/i, family: { key: "tinyllama", label: "TinyLlama", color: "#0668E1", initials: "TL" } },
  { match: /llama/i, family: { key: "llama", label: "Llama · Meta", color: "#0668E1", initials: "Ll", logo: "meta" } },
  { match: /qwen/i, family: { key: "qwen", label: "Qwen · Alibaba", color: "#FF6A00", initials: "Qw", logo: "qwen" } },
  { match: /phi-?\d|\bphi\b/i, family: { key: "phi", label: "Phi · Microsoft", color: "#8661C5", initials: "Phi" } },
  { match: /mixtral/i, family: { key: "mixtral", label: "Mixtral · Mistral AI", color: "#FA5B30", initials: "Mx", logo: "mistralai" } },
  { match: /mistral/i, family: { key: "mistral", label: "Mistral AI", color: "#FA5B30", initials: "Ms", logo: "mistralai" } },
  { match: /deepseek/i, family: { key: "deepseek", label: "DeepSeek", color: "#4D6BFE", initials: "DS", logo: "deepseek" } },
  { match: /falcon/i, family: { key: "falcon", label: "Falcon · TII", color: "#2E9E6D", initials: "Fa" } },
  { match: /starcoder/i, family: { key: "starcoder", label: "StarCoder · BigCode", color: "#6E40C9", initials: "SC" } },
  { match: /granite/i, family: { key: "granite", label: "Granite · IBM", color: "#0F62FE", initials: "Gr" } },
  { match: /(chat)?glm/i, family: { key: "glm", label: "GLM · Zhipu", color: "#00B2A9", initials: "GLM" } },
  { match: /\byi\b/i, family: { key: "yi", label: "Yi · 01.AI", color: "#E0364C", initials: "Yi" } },
  { match: /command-r|cohere/i, family: { key: "cohere", label: "Command R · Cohere", color: "#39594D", initials: "Co" } },
  { match: /vicuna/i, family: { key: "vicuna", label: "Vicuna", color: "#7C5CFC", initials: "Vi" } },
  { match: /gpt-?oss|gpt-?2/i, family: { key: "gpt", label: "GPT", color: "#10A37F", initials: "GPT" } },
  { match: /stablelm/i, family: { key: "stablelm", label: "StableLM · Stability AI", color: "#6B5BFF", initials: "SL" } },
  { match: /orca/i, family: { key: "orca", label: "Orca · Microsoft", color: "#8661C5", initials: "Or" } },
  { match: /openchat/i, family: { key: "openchat", label: "OpenChat", color: "#22A6B3", initials: "OC" } },
  { match: /solar/i, family: { key: "solar", label: "Solar · Upstage", color: "#F39C12", initials: "So" } },
  { match: /internlm/i, family: { key: "internlm", label: "InternLM", color: "#3D5AFE", initials: "IL" } },
  { match: /baichuan/i, family: { key: "baichuan", label: "Baichuan", color: "#D81B60", initials: "Ba" } },
];

const DEFAULT_COLOR = "#6b7280";

/** Infiere la familia/creador del modelo a partir de su nombre de archivo o repo. */
export function detectModelFamily(name: string): ModelFamily {
  for (const { match, family } of FAMILIES) {
    if (match.test(name)) return family;
  }
  const letter = name.trim().replace(/^[^a-zA-Z0-9]+/, "").charAt(0).toUpperCase() || "?";
  return { key: "generic", label: "Modelo", color: DEFAULT_COLOR, initials: letter };
}
