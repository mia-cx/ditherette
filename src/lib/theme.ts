import { atom } from 'nanostores';

/** Matches the key the pre-paint script in `app.html` reads. */
const STORAGE_KEY = 'ditherette:theme';

export type ThemeChoice = 'system' | 'light' | 'dark';

/** The chosen theme. `system` follows the OS; `app.html` applies the choice before first paint. */
export const themeChoice = atom<ThemeChoice>('system');

/** Read the saved choice into the store. Call once in the browser. */
export function loadThemeChoice() {
	const stored = localStorage.getItem(STORAGE_KEY);
	themeChoice.set(stored === 'light' || stored === 'dark' ? stored : 'system');
}

export function setThemeChoice(choice: ThemeChoice) {
	themeChoice.set(choice);
	const dark =
		choice === 'system' ? matchMedia('(prefers-color-scheme: dark)').matches : choice === 'dark';
	document.documentElement.classList.toggle('dark', dark);
	try {
		if (choice === 'system') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, choice);
	} catch {
		// Storage may be unavailable (private mode, quota); the class still applies for this visit.
	}
}
