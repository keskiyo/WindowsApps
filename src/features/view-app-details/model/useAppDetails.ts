import { useCallback, useEffect, useState } from 'react'
import type { AppDetails, AppsClient } from '../../../entities/app'

interface DetailsState {
	details: AppDetails | null
	isLoading: boolean
	error: string | null
}

export function useAppDetails(
	appId: string,
	appsClient: Pick<AppsClient, 'getAppDetails'>,
) {
	const [request, setRequest] = useState(0)
	const [state, setState] = useState<DetailsState>({
		details: null,
		isLoading: true,
		error: null,
	})
	useEffect(() => {
		let current = true
		setState({ details: null, isLoading: true, error: null })
		void appsClient
			.getAppDetails(appId)
			.then(details => {
				if (current)
					setState({ details, isLoading: false, error: null })
			})
			.catch(() => {
				if (current)
					setState({
						details: null,
						isLoading: false,
						error: 'Application details are unavailable.',
					})
			})
		return () => {
			current = false
		}
	}, [appId, appsClient, request])
	const retry = useCallback(() => setRequest(value => value + 1), [])
	return { ...state, retry }
}
