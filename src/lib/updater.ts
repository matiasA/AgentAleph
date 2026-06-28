import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";
import type { UpdateStatus } from "./types";

type Setter = (s: UpdateStatus) => void;

let pendingUpdate: Awaited<ReturnType<typeof check>> | null = null;

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 hours

export async function startUpdateChecker(set: Setter): Promise<void> {
  await runCheck(set);
  setInterval(() => runCheck(set), CHECK_INTERVAL_MS);
}

async function runCheck(set: Setter): Promise<void> {
  const isAppImage = await invoke<boolean>("is_appimage").catch(() => false);

  try {
    const update = await check();
    if (!update?.available) return;

    pendingUpdate = update;

    if (isAppImage) {
      set({
        phase: "downloading",
        version: update.version,
        notes: update.body ?? "",
        percent: 0,
        isAppImage: true,
        error: null,
      });

      let downloaded = 0;
      let total = 0;

      await update.download((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          const percent = total > 0 ? Math.round((downloaded / total) * 100) : 0;
          set({
            phase: "downloading",
            version: update.version,
            notes: update.body ?? "",
            percent,
            isAppImage: true,
            error: null,
          });
        } else if (event.event === "Finished") {
          set({
            phase: "ready",
            version: update.version,
            notes: update.body ?? "",
            percent: 100,
            isAppImage: true,
            error: null,
          });
        }
      });
    } else {
      set({
        phase: "available",
        version: update.version,
        notes: update.body ?? "",
        percent: 0,
        isAppImage: false,
        error: null,
      });
    }
  } catch (e: unknown) {
    // Silently ignore "no update" or network errors — don't surface to user
    const msg = String(e);
    if (msg.includes("No updates available") || msg.includes("status code 404")) return;
    console.warn("Update check failed:", e);
  }
}

export async function installAndRestart(): Promise<void> {
  if (!pendingUpdate) return;
  await pendingUpdate.install();
  await relaunch();
}
