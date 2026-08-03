import { useStore } from 'zustand'
// The store seam: launching state is owned by the single root store, and a per-card
// subscription is what keeps a launch from re-rendering the whole grid. See src/AGENTS.md §3
// for the single permitted upward edge into app/store.
import { useAppStoreApi } from '../../../app/store/appStoreContext'

/**
 * Subscribes to one app's launching flag so unrelated cards do not re-render.
 */
export function useIsLaunching(id: string): boolean {
	return useStore(useAppStoreApi(), state => state.launchingIds.includes(id))
}
