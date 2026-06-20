import type { AppCategory } from './category'

export type AppLaunchKind = 'executable' | 'shortcut' | 'app_user_model_id'
export type AppSourceKind = 'registry' | 'start_menu' | 'start_apps' | 'msix'

export interface AppInfo {
	id: string
	name: string
	path: string
	iconBase64: string | null
	category: AppCategory
	launchKind: AppLaunchKind
	sourceKind: AppSourceKind
	description: string | null
	version: string | null
	publisher: string | null
	installLocation: string | null
	canUninstall: boolean
}

export type AppView = 'all' | 'favorites' | 'settings' | 'hidden'

export interface CatalogSnapshot {
	apps: AppInfo[]
	hasCache: boolean
}

export interface AppsClient {
	getApps(): Promise<CatalogSnapshot>
	refreshApps(): Promise<AppInfo[]>
	launchApp(app: Pick<AppInfo, 'launchKind' | 'path'>): Promise<void>
	uninstallApp(id: string): Promise<void>
	onAppsUpdated(handler: (apps: AppInfo[]) => void): Promise<() => void>
}
