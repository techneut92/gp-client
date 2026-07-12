// Locale override helper on top of the Paraglide runtime.
//
// Resolution order (vite.config.ts strategy): explicit user override in
// localStorage → the system/browser language → English. "System default"
// simply clears the override.
import { getLocale, setLocale, localStorageKey, locales } from '../paraglide/runtime.js';
import { m } from '../paraglide/messages.js';

export type LocaleChoice = 'system' | 'en' | 'nl' | 'fy';

export interface LocaleOption {
  value: LocaleChoice;
  label: string;
}

/// The persisted override, or 'system' when following the OS language.
export function currentChoice(): LocaleChoice {
  const stored = localStorage.getItem(localStorageKey);
  return stored !== null && (locales as readonly string[]).includes(stored) ? (stored as LocaleChoice) : 'system';
}

/// Apply a choice. Both paths reload the window so every m.* call re-renders.
export function applyChoice(choice: LocaleChoice): void {
  if (choice === 'system') {
    localStorage.removeItem(localStorageKey);
    window.location.reload();
    return;
  }
  if (choice !== getLocale()) {
    setLocale(choice); // persists to localStorage and reloads
  } else {
    // Same language but previously implicit — persist the explicit override.
    localStorage.setItem(localStorageKey, choice);
  }
}

/// Options for a language dropdown. Language names are endonyms (never
/// translated); only the "system default" entry is localized.
export function localeOptions(): LocaleOption[] {
  return [
    { value: 'system', label: m.language_system() },
    { value: 'en', label: 'English' },
    { value: 'nl', label: 'Nederlands' },
    { value: 'fy', label: 'Frysk' },
  ];
}
