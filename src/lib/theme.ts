import { atom } from 'nanostores';

/** Matches the key the pre-paint script in `app.html` reads. */
const STORAGE_KEY = 'ditherette:theme';
const SYSTEM_DARK = '(prefers-color-scheme: dark)';

export type ThemeChoice = 'system' | 'light' | 'dark';

/** The chosen theme. `system` follows the OS; `app.html` applies the choice before first paint. */
export const themeChoice = atom<ThemeChoice>('system');

function applyTheme(choice: ThemeChoice) {
	const dark = choice === 'system' ? matchMedia(SYSTEM_DARK).matches : choice === 'dark';
	document.documentElement.classList.toggle('dark', dark);
}

/**
 * Read the saved choice, and follow OS theme changes while System is chosen.
 * Call once in the browser; returns the cleanup.
 */
export function startTheme() {
	let stored: string | null = null;
	try {
		stored = localStorage.getItem(STORAGE_KEY);
	} catch {
		// Blocked storage (private mode, sandboxed frames) leaves the System default.
	}
	themeChoice.set(stored === 'light' || stored === 'dark' ? stored : 'system');
	const media = matchMedia(SYSTEM_DARK);
	const follow = () => {
		if (themeChoice.get() === 'system') applyTheme('system');
	};
	media.addEventListener('change', follow);
	return () => media.removeEventListener('change', follow);
}

export function setThemeChoice(choice: ThemeChoice) {
	themeChoice.set(choice);
	applyTheme(choice);
	try {
		if (choice === 'system') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, choice);
	} catch {
		// Storage may be unavailable (private mode, quota); the class still applies for this visit.
	}
}
