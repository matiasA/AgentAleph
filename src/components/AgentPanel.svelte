<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icon.svelte";
  import BrandLogo from "./BrandLogo.svelte";
  import { api } from "../lib/api";
  import type { Skill } from "../lib/types";
  import {
    agentContext,
    addContextItem,
    removeContextItem,
  } from "../lib/agentContext.svelte";
  import { activeProject } from "../lib/activeProject.svelte";

  let { mode = "chat" }: { mode?: "chat" | "agent" } = $props();

  let skillsOpen = $state(true);
  let memOpen = $state(true);
  let connOpen = $state(true);
  let ctxOpen = $state(true);

  let skills = $state<Skill[]>([]);
  let busy = $state(false);

  // Memory: project (repo-scoped) + user (global) persistent facts.
  let memProjectContent = $state("");
  let memUserContent = $state("");
  let memProjectBudget = $state(2200);
  let memUserBudget = $state(1375);
  let editingProject = $state(false);
  let editingUser = $state(false);
  let memProjectDraft = $state("");
  let memUserDraft = $state("");
  let memBusy = $state(false);

  // Skill creation form.
  let creating = $state(false);
  let nName = $state("");
  let nDesc = $state("");
  let nBody = $state("");

  // Text attached to context.
  let addingText = $state(false);
  let textVal = $state("");

  // Connections: disabled placeholder. The app is 100% local; integrations are future work.
  const connections: { name: string; desc: string; logo: "github" | "google" }[] = [
    { name: "GitHub", desc: "Repos, issues, and PRs", logo: "github" },
    { name: "Google", desc: "Drive, Calendar, search", logo: "google" },
  ];

  onMount(refreshSkills);
  onMount(refreshUserMemory);

  $effect(() => {
    activeProject.workingDir; // dependency: re-fetch when the active project changes
    refreshProjectMemory();
  });

  async function refreshSkills() {
    try {
      skills = await api.listSkills();
    } catch {
      skills = [];
    }
  }

  async function toggleSkill(s: Skill) {
    try {
      await api.setSkillEnabled(s.slug, !s.enabled);
      s.enabled = !s.enabled;
    } catch (e) {
      console.error("toggle skill", e);
    }
  }

  async function createSkill() {
    if (!nName.trim() || busy) return;
    busy = true;
    try {
      await api.createSkill(nName.trim(), nDesc.trim(), nBody.trim());
      nName = nDesc = nBody = "";
      creating = false;
      await refreshSkills();
    } catch (e) {
      alert("Could not create skill: " + e);
    } finally {
      busy = false;
    }
  }

  async function importSkill() {
    if (busy) return;
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    busy = true;
    try {
      await api.importSkill(dir);
      await refreshSkills();
    } catch (e) {
      alert("Could not import: " + e);
    } finally {
      busy = false;
    }
  }

  async function deleteSkill(s: Skill) {
    if (!confirm(`Delete skill "${s.name}"?`)) return;
    try {
      await api.deleteSkill(s.slug);
      await refreshSkills();
    } catch (e) {
      alert("Could not delete: " + e);
    }
  }

  // ---------- Memory ----------

  async function refreshUserMemory() {
    try {
      const settings = await api.getSettings();
      memProjectBudget = settings.memory_project_budget;
      memUserBudget = settings.memory_user_budget;
    } catch {
      // keep defaults
    }
    try {
      memUserContent = await api.readUserMemory();
    } catch {
      memUserContent = "";
    }
  }

  async function refreshProjectMemory() {
    if (!activeProject.workingDir) {
      memProjectContent = "";
      return;
    }
    try {
      memProjectContent = await api.readProjectMemory(activeProject.workingDir);
    } catch {
      memProjectContent = "";
    }
  }

  function startEditProject() {
    memProjectDraft = memProjectContent;
    editingProject = true;
  }

  async function saveProject() {
    if (!activeProject.workingDir || memBusy) return;
    memBusy = true;
    try {
      await api.writeProjectMemory(activeProject.workingDir, memProjectDraft);
      memProjectContent = memProjectDraft;
      editingProject = false;
    } catch (e) {
      alert("Could not save project memory: " + e);
    } finally {
      memBusy = false;
    }
  }

  async function clearProjectMem() {
    if (!activeProject.workingDir || memBusy) return;
    if (!confirm("Clear project memory (MEMORY.md)? This cannot be undone.")) return;
    memBusy = true;
    try {
      await api.clearProjectMemory(activeProject.workingDir);
      memProjectContent = "";
      editingProject = false;
    } catch (e) {
      alert("Could not clear: " + e);
    } finally {
      memBusy = false;
    }
  }

  function startEditUser() {
    memUserDraft = memUserContent;
    editingUser = true;
  }

  async function saveUser() {
    if (memBusy) return;
    memBusy = true;
    try {
      await api.writeUserMemory(memUserDraft);
      memUserContent = memUserDraft;
      editingUser = false;
    } catch (e) {
      alert("Could not save user memory: " + e);
    } finally {
      memBusy = false;
    }
  }

  async function clearUserMem() {
    if (memBusy) return;
    if (!confirm("Clear user memory (USER.md)? This applies to all projects.")) return;
    memBusy = true;
    try {
      await api.clearUserMemory();
      memUserContent = "";
      editingUser = false;
    } catch (e) {
      alert("Could not clear: " + e);
    } finally {
      memBusy = false;
    }
  }

  // ---------- Context ----------

  async function addFiles() {
    const sel = await open({ multiple: true });
    const paths = Array.isArray(sel) ? sel : sel ? [sel] : [];
    for (const p of paths) {
      try {
        const f = await api.readContextFile(p);
        addContextItem({
          id: crypto.randomUUID(),
          kind: "file",
          label: f.name,
          content: f.content,
          truncated: f.truncated,
        });
      } catch (e) {
        console.error("read context file", e);
      }
    }
  }

  function addText() {
    const v = textVal.trim();
    if (!v) return;
    addContextItem({
      id: crypto.randomUUID(),
      kind: "text",
      label: "Text",
      content: v,
    });
    textVal = "";
    addingText = false;
  }
</script>

<div class="panel">
  <!-- Skills -->
  <div class="panel-section">
    <button class="section-head" onclick={() => (skillsOpen = !skillsOpen)}>
      <span class="section-label">Skills</span>
      <span class="chev" class:open={skillsOpen}><Icon name="chevron-up" size="sm" /></span>
    </button>

    {#if skillsOpen}
      <div class="row-actions">
        <button class="mini-btn" onclick={() => (creating = !creating)}>
          <Icon name="plus" size="sm" /> Create
        </button>
        <button class="mini-btn" onclick={importSkill} disabled={busy}>
          <Icon name="folder-plus" size="sm" /> Import
        </button>
      </div>

      {#if creating}
        <div class="create-form">
          <input class="inp" placeholder="Name" bind:value={nName} />
          <input class="inp" placeholder="Description" bind:value={nDesc} />
          <textarea class="inp" rows="4" placeholder="Instructions (SKILL.md body)" bind:value={nBody}></textarea>
          <div class="row-actions">
            <button class="mini-btn primary" onclick={createSkill} disabled={busy || !nName.trim()}>Save</button>
            <button class="mini-btn" onclick={() => (creating = false)}>Cancel</button>
          </div>
        </div>
      {/if}

      <div class="list">
        {#each skills as s (s.slug)}
          <div class="skill-card" class:on={s.enabled}>
            <span class="skill-ico"><Icon name="box" /></span>
            <span class="skill-text">
              <span class="skill-title">{s.name}</span>
              <span class="skill-desc">{s.description || "No description"}</span>
            </span>
            <button
              class="toggle"
              class:on={s.enabled}
              title={s.enabled ? "Active" : "Inactive"}
              onclick={() => toggleSkill(s)}>
              <span class="knob"></span>
            </button>
            <button class="icon-btn" title="Delete" onclick={() => deleteSkill(s)}>
              <Icon name="x" size="sm" />
            </button>
          </div>
        {/each}
        {#if !skills.length}
          <div class="empty">No skills yet. Create one or import a folder with a <code>SKILL.md</code>.</div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Memory -->
  <div class="panel-section">
    <button class="section-head" onclick={() => (memOpen = !memOpen)}>
      <span class="section-label">Memory</span>
      <span class="chev" class:open={memOpen}><Icon name="chevron-up" size="sm" /></span>
    </button>

    {#if memOpen}
      <div class="mem-block">
        <div class="mem-head">
          <span class="mem-title">Project</span>
          <span class="mem-count">{memProjectContent.length} / {memProjectBudget}</span>
        </div>
        <div class="mem-note">
          Saved in the repo (<code>.agent-aleph/MEMORY.md</code>) and shareable via git.
          Don't store credentials or sensitive data.
        </div>
        {#if editingProject}
          <textarea class="inp mem-area" rows="5" bind:value={memProjectDraft}></textarea>
          <div class="row-actions">
            <button class="mini-btn primary" onclick={saveProject} disabled={memBusy}>Save</button>
            <button class="mini-btn" onclick={() => (editingProject = false)}>Cancel</button>
          </div>
        {:else}
          <pre class="mem-area readonly">{memProjectContent || "No project memory yet. The agent will record facts here as it works."}</pre>
          <div class="row-actions">
            <button class="mini-btn" onclick={startEditProject} disabled={!activeProject.workingDir}>Edit</button>
            <button class="mini-btn" onclick={clearProjectMem} disabled={!memProjectContent || memBusy}>Clear</button>
          </div>
        {/if}
      </div>

      <div class="mem-block">
        <div class="mem-head">
          <span class="mem-title">User (global)</span>
          <span class="mem-count">{memUserContent.length} / {memUserBudget}</span>
        </div>
        {#if editingUser}
          <textarea class="inp mem-area" rows="5" bind:value={memUserDraft}></textarea>
          <div class="row-actions">
            <button class="mini-btn primary" onclick={saveUser} disabled={memBusy}>Save</button>
            <button class="mini-btn" onclick={() => (editingUser = false)}>Cancel</button>
          </div>
        {:else}
          <pre class="mem-area readonly">{memUserContent || "No user memory yet. Applies across all projects."}</pre>
          <div class="row-actions">
            <button class="mini-btn" onclick={startEditUser}>Edit</button>
            <button class="mini-btn" onclick={clearUserMem} disabled={!memUserContent || memBusy}>Clear</button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Connections (placeholder) -->
  <div class="panel-section">
    <button class="section-head" onclick={() => (connOpen = !connOpen)}>
      <span class="section-label">Connections</span>
      <span class="badge">coming soon</span>
      <span class="chev" class:open={connOpen}><Icon name="chevron-up" size="sm" /></span>
    </button>
    {#if connOpen}
      <div class="list">
        {#each connections as c (c.name)}
          <div class="conn-card" aria-disabled="true">
            <span class="conn-ico"><BrandLogo name={c.logo} size={18} /></span>
            <span class="skill-text">
              <span class="skill-title">{c.name}</span>
              <span class="skill-desc">{c.desc}</span>
            </span>
            <span class="soon">coming soon</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Context -->
  <div class="panel-section grow">
    <button class="section-head" onclick={() => (ctxOpen = !ctxOpen)}>
      <span class="section-label">Context</span>
      <span class="chev" class:open={ctxOpen}><Icon name="chevron-up" size="sm" /></span>
    </button>

    {#if ctxOpen}
      <div class="row-actions">
        <button class="mini-btn" onclick={addFiles}><Icon name="paperclip" size="sm" /> File</button>
        <button class="mini-btn" onclick={() => (addingText = !addingText)}><Icon name="file-text" size="sm" /> Text</button>
      </div>

      {#if addingText}
        <div class="create-form">
          <textarea class="inp" rows="3" placeholder="Paste text to attach to context" bind:value={textVal}></textarea>
          <div class="row-actions">
            <button class="mini-btn primary" onclick={addText} disabled={!textVal.trim()}>Add</button>
            <button class="mini-btn" onclick={() => (addingText = false)}>Cancel</button>
          </div>
        </div>
      {/if}

      <div class="list">
        {#each agentContext.items as it (it.id)}
          <div class="ctx-card">
            <span class="conn-ico"><Icon name={it.kind === "file" ? "file-text" : "files"} /></span>
            <span class="skill-text">
              <span class="skill-title">{it.label}</span>
              <span class="skill-desc">{it.content.slice(0, 80)}{it.content.length > 80 ? "…" : ""}</span>
            </span>
            <button class="icon-btn" title="Remove" onclick={() => removeContextItem(it.id)}>
              <Icon name="x" size="sm" />
            </button>
          </div>
        {/each}
        {#if !agentContext.items.length}
          <div class="empty">No context attached. Add files or text for the next turn.</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px 14px;
    gap: 16px;
    overflow-y: auto;
  }
  .panel-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .panel-section.grow {
    flex: 1;
    min-height: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 2px;
    color: var(--text-2);
  }
  .section-head:hover {
    color: var(--text-1);
  }
  .section-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1px;
    flex: 1;
    text-align: left;
  }
  .chev {
    display: inline-flex;
    color: var(--text-2);
    transition: transform var(--t);
  }
  .chev:not(.open) {
    transform: rotate(180deg);
  }
  .badge {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-3);
    border: 1px solid var(--border-soft);
    border-radius: 999px;
    padding: 1px 6px;
  }

  .row-actions {
    display: flex;
    gap: 6px;
  }
  .mini-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 500;
    padding: 5px 9px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-1);
  }
  .mini-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-0);
  }
  .mini-btn:disabled {
    opacity: 0.5;
  }
  .mini-btn.primary {
    background: var(--accent-bg);
    color: var(--accent-2);
  }

  .create-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--bg-2);
    border-radius: var(--radius);
  }
  .inp {
    width: 100%;
    font-size: 12px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-soft);
    background: var(--bg-1);
    color: var(--text-0);
    box-sizing: border-box;
    resize: vertical;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .skill-card,
  .conn-card,
  .ctx-card {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-2);
    border: 1px solid transparent;
    border-radius: var(--radius);
    padding: 9px 10px;
  }
  .skill-card.on {
    border-color: var(--accent-border);
  }
  .conn-card {
    opacity: 0.6;
  }
  .skill-ico,
  .conn-ico {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm);
    flex: none;
  }
  .skill-ico {
    background: var(--accent-bg);
    color: var(--accent-2);
  }
  .conn-ico {
    background: #fff;
  }
  .skill-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .skill-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .skill-desc {
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toggle {
    width: 30px;
    height: 17px;
    border-radius: 999px;
    background: var(--bg-4, #3a3a3a);
    border: none;
    padding: 0;
    position: relative;
    flex: none;
    transition: background var(--t-fast);
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: #fff;
    transition: transform var(--t-fast);
  }
  .toggle.on .knob {
    transform: translateX(13px);
  }
  .soon {
    font-size: 10px;
    color: var(--text-3);
    flex: none;
  }
  .empty {
    font-size: 11.5px;
    color: var(--text-3);
    padding: 10px;
    border: 1px dashed var(--border-soft);
    border-radius: var(--radius);
    line-height: 1.5;
  }
  .empty code {
    font-size: 10.5px;
  }

  .mem-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--bg-2);
    border-radius: var(--radius);
    padding: 9px 10px;
  }
  .mem-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .mem-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-0);
  }
  .mem-count {
    font-size: 10.5px;
    color: var(--text-3);
  }
  .mem-note {
    font-size: 10.5px;
    color: var(--text-3);
    line-height: 1.4;
  }
  .mem-note code {
    font-size: 10px;
  }
  .mem-area {
    font-size: 11.5px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .mem-area.readonly {
    margin: 0;
    color: var(--text-2);
    font-family: inherit;
    max-height: 160px;
    overflow-y: auto;
  }
</style>
