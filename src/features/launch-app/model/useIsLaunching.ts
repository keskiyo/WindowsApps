import { useStore } from 'zustand'
import { useAppStoreApi } from './appStoreContext'

/**
 * Subscribes to one app's launching flag so unrelated cards do not re-render.
 */
export function useIsLaunching(id: string): boolean {
	return useStore(useAppStoreApi(), state => state.launchingIds.includes(id))
}
