import { createContext, useContext, type ReactNode } from 'react'
import { useStore } from 'zustand'
import type { StoreApi } from 'zustand/vanilla'
import type { AppState } from './appStore'

const StoreContext = createContext<StoreApi<AppState> | null>(null)

export function AppStoreProvider({
	store,
	children,
}: {
	store: StoreApi<AppState>
	children: ReactNode
}) {
	return (
		<StoreContext.Provider value={store}>{children}</StoreContext.Provider>
	)
}

function useAppStoreApi(): StoreApi<AppState> {
	const store = useContext(StoreContext)
	if (!store) throw new Error('AppStoreProvider is missing')
	return store
}

/**
 * Subscribes to a single app's launching flag. The zustand selector only re-renders the
 * calling card when ITS membership changes, so launching one app doesn't re-render the grid.
 */
export function useIsLaunching(id: string): boolean {
	return useStore(useAppStoreApi(), state => state.launchingIds.includes(id))
}
