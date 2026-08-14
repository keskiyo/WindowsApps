import { useEffect } from 'react'
import { toast } from 'sonner'
import { toAppClientError } from '../../shared/api/tauri/errors'

interface BootstrapInput {
	initialize(): Promise<() => void>
	error: string | null
	isLoading: boolean
	visibleHydrationIds: string
	hydrateVisibleIcons(ids: string[]): Promise<void>
}

export function useCatalogBootstrap({
	initialize,
	error,
	isLoading,
	visibleHydrationIds,
	hydrateVisibleIcons,
}: BootstrapInput) {
	useEffect(() => {
		let dispose: (() => void) | undefined
		let cancelled = false
		function connect() {
			void initialize()
				.then(value => {
					if (cancelled) value()
					else dispose = value
				})
				.catch(reason => {
					if (cancelled) return
					toast.error(toAppClientError(reason).message, {
						action: { label: 'Retry', onClick: () => connect() },
					})
				})
		}
		connect()
		return () => {
			cancelled = true
			dispose?.()
		}
	}, [initialize])

	useEffect(() => {
		if (error) toast.error(error)
	}, [error])

	useEffect(() => {
		if (isLoading) return
		const ids = visibleHydrationIds.split('|').filter(Boolean)
		if (ids.length) void hydrateVisibleIcons(ids)
	}, [hydrateVisibleIcons, isLoading, visibleHydrationIds])
}
