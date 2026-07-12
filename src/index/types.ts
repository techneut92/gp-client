// UI-local types shared between the main-window components.

export type View = '' | 'setup' | 'lock' | 'unlocked' | 'backend';

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
  kind: string;
  onClick: () => void;
}
