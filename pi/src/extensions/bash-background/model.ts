/** A background shell command and the output-file lifecycle observed by the footer and browser. */
export interface BackgroundProcess {
  pid: number;
  logPath: string;
  command: string;
  startedAt: number;
  exit?: BackgroundExit;
}

export interface BackgroundExit {
  code: number | null;
  signal: NodeJS.Signals | null;
  timedOut: boolean;
  endedAt: number;
}

/** Mutable state for one Pi session. Completed processes remain available to the output browser. */
export interface BackgroundSession {
  active: boolean;
  processes: Map<number, BackgroundProcess>;
  listeners: Set<() => void>;
}

export function createBackgroundSession(): BackgroundSession {
  return { active: true, processes: new Map(), listeners: new Set() };
}

export function runningProcesses(session: BackgroundSession): BackgroundProcess[] {
  return [...session.processes.values()].filter((process) => process.exit === undefined);
}

export function publishProcesses(session: BackgroundSession): void {
  for (const listener of session.listeners) listener();
}

export function subscribeToProcesses(session: BackgroundSession, listener: () => void): () => void {
  session.listeners.add(listener);
  return () => session.listeners.delete(listener);
}
