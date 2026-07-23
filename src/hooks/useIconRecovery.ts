import { useEffect } from 'react'

const ICON_RECOVERY_INTERVAL_MS = 3 * 60 * 60 * 1000

export function useIconRecovery(recoverMissingIcons: () => Promise<void>) {
	useEffect(() => {
		const interval = window.setInterval(() => {
			void recoverMissingIcons().catch(() => undefined)
		}, ICON_RECOVERY_INTERVAL_MS)
		return () => window.clearInterval(interval)
	}, [recoverMissingIcons])
}
