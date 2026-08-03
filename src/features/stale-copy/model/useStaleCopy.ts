import { useCallback, useEffect, useState } from 'react'
import type { StaleCopyInfo, SystemClient } from '../../../entities/system'

/**
 * Reports a second, outdated copy of the app running from a different location.
 *
 * The client method is optional and rejects outside Tauri (dev browser, tests), which is a
 * normal absence of information rather than an error — nothing is surfaced to the user then.
 */
export function useStaleCopy(systemClient: Pick<SystemClient, 'staleCopyStatus'>) {
	const [staleCopy, setStaleCopy] = useState<StaleCopyInfo | null>(null)

	useEffect(() => {
		let active = true
		systemClient
			.staleCopyStatus?.()
			.then(value => {
				if (active) setStaleCopy(value ?? null)
			})
			.catch(() => {
				// Not in Tauri (dev browser / tests) — no stale copy to report.
			})
		return () => {
			active = false
		}
	}, [systemClient])

	const dismiss = useCallback(() => setStaleCopy(null), [])

	return { dismiss, staleCopy }
}
