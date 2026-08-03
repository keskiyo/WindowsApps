import type { AppInfo } from '../../../../entities/app'

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
