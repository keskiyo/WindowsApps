import { useStore } from 'zustand'
import { useAppStoreApi } from '../../../app/store/appStoreContext'

export function useIsLaunching(id: string): boolean {
	return useStore(useAppStoreApi(), state => state.launchingIds.includes(id))
}
