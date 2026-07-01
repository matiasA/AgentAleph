// Shared state for the active project's working directory.
// Written by AgentView when the user picks a folder or loads a session;
// read by AgentPanel for project-scoped memory operations.

export const activeProject = $state<{ workingDir: string }>({ workingDir: "" });

export function setActiveWorkingDir(dir: string) {
  activeProject.workingDir = dir;
}
