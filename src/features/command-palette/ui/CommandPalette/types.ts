import type { AppInfo } from '../../../types'

export interface CommandPaletteProps {
	apps: AppInfo[]
	onLaunch(app: AppInfo): Promise<void>
	onClose(): void
}

export interface ResultItemProps {
	app: AppInfo
	selected: boolean
	onHover(): void
	onActivate(): void
}
