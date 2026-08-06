import { useCallback, useEffect, useState } from 'react'
import type { StaleCopyInfo, SystemClient } from '../../../entities/system'

export function useStaleCopy(
	systemClient: Pick<SystemClient, 'staleCopyStatus'>,
) {
	const [staleCopy, setStaleCopy] = useState<StaleCopyInfo | null>(null)

	useEffect(() => {
		let active = true
		systemClient
			.staleCopyStatus?.()
			.then(value => {
				if (active) setStaleCopy(value ?? null)
			})
			.catch(() => {})
		return () => {
			active = false
		}
	}, [systemClient])

	const dismiss = useCallback(() => setStaleCopy(null), [])

	return { dismiss, staleCopy }
}
