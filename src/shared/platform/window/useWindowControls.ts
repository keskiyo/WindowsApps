import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

type AppWindow = ReturnType<typeof getCurrentWindow>

export interface WindowControls {
	maximized: boolean
	minimize(): void
	toggleMaximize(): void
	close(): void
}

function runOnWindow(action: (win: AppWindow) => Promise<unknown>): void {
	try {
		void action(getCurrentWindow())
	} catch (ignored) {
		void ignored
	}
}

export function useWindowControls(): WindowControls {
	const [maximized, setMaximized] = useState(false)
	useEffect(() => {
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
				.catch(() => {})
		} catch (ignored) {
			void ignored
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
