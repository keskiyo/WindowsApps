import type { ReactNode } from 'react'
import type { StoreApi } from 'zustand/vanilla'
import type { AppState } from './appStore'
import { AppStoreContext } from './appStoreContext'

export function AppStoreProvider({
	store,
	children,
}: {
	store: StoreApi<AppState>
	children: ReactNode
}) {
	return (
		<AppStoreContext.Provider value={store}>
			{children}
		</AppStoreContext.Provider>
	)
}
