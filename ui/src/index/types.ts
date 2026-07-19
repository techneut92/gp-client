// UI-local types shared between the main-window components.

export type View = '' | 'setup' | 'lock' | 'unlocked' | 'backend' | 'backend-new' | 'import';

export interface Chip {
  k: string;
  v: string;
  color?: string;
}

export interface LogLine {
  msg: string;
  err: boolean;
}

export interface Banner {
  text: string;
  /** Second line — the component versions, e.g. "App v1.4.2 · backend v1.4.1". */
  sub?: string;
  kind: string;
  onClick: () => void;
}
