import { useCallback, useEffect, useRef, useState } from 'react'
import type { AppInfo } from '../../../entities/app'

export function useAppInfoDialog() {
	const [app, setApp] = useState<AppInfo | null>(null)
	const restoreFocusRef = useRef<HTMLElement | null>(null)
	const open = useCallback((nextApp: AppInfo) => {
		const activeElement = document.activeElement
		restoreFocusRef.current =
			activeElement instanceof HTMLElement ? activeElement : null
		setApp(nextApp)
	}, [])
	const close = useCallback(() => setApp(null), [])
	useEffect(() => {
		if (app) return
		const restoreTarget = restoreFocusRef.current
		if (!restoreTarget?.isConnected) return
		const frame = requestAnimationFrame(() => restoreTarget.focus())
		return () => cancelAnimationFrame(frame)
	}, [app])
	return { app, close, open }
}
