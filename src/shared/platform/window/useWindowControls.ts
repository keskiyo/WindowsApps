import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

type AppWindow = ReturnType<typeof getCurrentWindow>

export interface WindowControls {
	/** True while the OS window is maximized (Windows 11 shows a restore glyph then). */
	maximized: boolean
	minimize(): void
	toggleMaximize(): void
	close(): void
}

function runOnWindow(action: (win: AppWindow) => Promise<unknown>): void {
	try {
		void action(getCurrentWindow())
	} catch {
		/* not running inside Tauri (e.g. tests) */
	}
}

/**
 * Keeps the Tauri window API out of the rest of the app — `no-restricted-imports` bans
 * `@tauri-apps/*` across `src/**` except the four integration modules named in
 * `eslint.config.js`, so window chrome talks to the runtime
 * through this hook. Outside the Tauri runtime every control is inert instead of
 * throwing, which is what lets the title bar render in jsdom tests.
 */
export function useWindowControls(): WindowControls {
	const [maximized, setMaximized] = useState(false)
	useEffect(() => {
		// Registration resolves asynchronously, so an unmount can happen first. Without
		// the flag the handle would arrive after cleanup and the resize listener would
		// stay registered for the lifetime of the process.
		let cancelled = false
		let dispose: (() => void) | undefined
		const track = (win: AppWindow) => {
			void win
				.isMaximized()
				.then(value => {
					if (!cancelled) setMaximized(value)
				})
				.catch(() => undefined)
		}
		try {
			const win = getCurrentWindow()
			track(win)
			void win
				.onResized(() => track(win))
				.then(stop => {
					if (cancelled) stop()
					else dispose = stop
				})
				.catch(() => {
					/* window API unavailable — the controls stay inert */
				})
		} catch {
			/* not running inside Tauri (e.g. tests) */
		}
		return () => {
			cancelled = true
			dispose?.()
		}
	}, [])
	return {
		maximized,
		minimize: () => runOnWindow(win => win.minimize()),
		toggleMaximize: () => runOnWindow(win => win.toggleMaximize()),
		close: () => runOnWindow(win => win.close()),
	}
}
