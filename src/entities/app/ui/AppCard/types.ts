import type { ReactNode, RefObject } from 'react'
import type { AppInfo } from '../../model/app.types'

export interface AppCardActions {
	close(): void
	anchorRef: RefObject<HTMLButtonElement | null>
}

export interface AppCardProps {
	app: AppInfo
	isFavorite: boolean
	launching: boolean
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	isAuxiliary?: boolean
	renderActions(actions: AppCardActions): ReactNode
}

export interface CardIconProps {
	iconBase64: string | null
	launching: boolean
}

export interface CardLabelProps {
	name: string
	version: string | null
	launching: boolean
}

export interface FavoriteButtonProps {
	appName: string
	isFavorite: boolean
	onToggle(): void
}
