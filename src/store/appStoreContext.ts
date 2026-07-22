import { createContext, useContext } from 'react'
import type { StoreApi } from 'zustand/vanilla'
import type { AppState } from './appStore'

export const AppStoreContext = createContext<StoreApi<AppState> | null>(null)

export function useAppStoreApi(): StoreApi<AppState> {
	const store = useContext(AppStoreContext)
	if (!store) throw new Error('AppStoreProvider is missing')
	return store
}
