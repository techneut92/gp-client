// Locale override on top of the Paraglide runtime.
//
// Resolution: explicit user override (our own localStorage key) → system
// language → English. We deliberately do NOT use Paraglide's localStorage
// strategy: its runtime caches the *resolved* locale back into its own key,
// which makes "no override" indistinguishable from an explicit choice.
// Instead this module owns resolution via overwriteGetLocale(), and the
// override lives under our own key. Import this module before mounting any
// window so the overwrite is installed before the first m.*() call.
import { overwriteGetLocale, locales, localStorageKey, type Locale } from '../paraglide/runtime.js';
import { m } from '../paraglide/messages.js';
import { emit, listen } from '@tauri-apps/api/event';

export type LocaleChoice = 'system' | Locale;

export interface LocaleOption {
  value: LocaleChoice;
  label: string;
  /** English name of the language, shown as a secondary line and searchable. */
  sub?: string;
}

const OVERRIDE_KEY = 'gp-client-locale-override';

function isLocale(v: string | null): v is Locale {
  return v !== null && (locales as readonly string[]).includes(v);
}

function systemLocale(): Locale {
  for (const lang of navigator.languages ?? [navigator.language]) {
    const base = (lang ?? '').toLowerCase().split('-')[0] ?? '';
    if (isLocale(base)) return base;
  }
  return 'en';
}

function resolve(): Locale {
  const override = localStorage.getItem(OVERRIDE_KEY);
  return isLocale(override) ? override : systemLocale();
}

// Installed at module-import time — before any component renders m.*().
overwriteGetLocale(resolve);
// Drop any stale cache Paraglide's own strategy may have written previously.
localStorage.removeItem(localStorageKey);

// Every window (main, settings, manager) installs this on import. When any window
// changes the locale (applyChoice → emit), all of them reload — otherwise a switch
// made in Settings leaves the main window's imperatively-built strings (status
// pill, identity chips) stale in the old language.
void listen('locale-changed', () => window.location.reload()).catch(() => {});

/// The persisted override, or 'system' when following the OS language.
export function currentChoice(): LocaleChoice {
  const stored = localStorage.getItem(OVERRIDE_KEY);
  return isLocale(stored) ? stored : 'system';
}

/// Apply a choice and reload EVERY window in the new locale. We broadcast rather
/// than reload only this window, because a switch made in Settings must also
/// refresh the main window (whose status pill + identity chips are built
/// imperatively and don't re-read the locale without a reload).
export function applyChoice(choice: LocaleChoice): void {
  if (choice === 'system') {
    localStorage.removeItem(OVERRIDE_KEY);
  } else {
    localStorage.setItem(OVERRIDE_KEY, choice);
  }
  // Delivered to all windows incl. this one; each reloads via the listener above.
  void emit('locale-changed').catch(() => window.location.reload());
}

/// Options for a language dropdown. The label is the endonym (native name, never
/// translated); the sub is the English exonym, so the list stays recognisable and
/// searchable in any UI language. Only the "system default" entry is localized.
export function localeOptions(): LocaleOption[] {
  return [
    { value: 'system', label: m.language_system() },
    { value: 'en', label: 'English', sub: 'English' },
    { value: 'nl', label: 'Nederlands', sub: 'Dutch' },
    { value: 'fy', label: 'Frysk', sub: 'Frisian' },
  ];
}
