import { useCallback, useRef, useState } from 'react'
import { isInstaller } from '../../../entities/app'
import type { AppInfo } from '../../../entities/app'

export function useInstallerLaunch(onLaunch: (app: AppInfo) => Promise<void>) {
	const [app, setApp] = useState<AppInfo | null>(null)
	const [pending, setPending] = useState(false)
	const pendingRef = useRef(false)

	const requestLaunch = useCallback(
		async (candidate: AppInfo) => {
			if (isInstaller(candidate)) {
				setApp(candidate)
				return
			}
			await onLaunch(candidate)
		},
		[onLaunch],
	)

	const cancel = useCallback(() => {
		if (!pendingRef.current) setApp(null)
	}, [])

	const confirm = useCallback(async () => {
		if (!app || pendingRef.current) return
		pendingRef.current = true
		setPending(true)
		try {
			await onLaunch(app)
			setApp(null)
		} finally {
			pendingRef.current = false
			setPending(false)
		}
	}, [app, onLaunch])

	return { app, pending, requestLaunch, confirm, cancel }
}
