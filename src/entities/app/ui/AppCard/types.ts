import type { ReactNode, RefObject } from 'react'
import type { AppInfo } from '../../model/app.types'

/** What the card hands to whoever renders the menu, so the entity owns no action. */
export interface AppCardActions {
	/** Closes the menu and restores focus to the menu trigger. */
	close(): void
	/** The menu button the menu positions itself against. */
	anchorRef: RefObject<HTMLButtonElement | null>
}

export interface AppCardProps {
	app: AppInfo
	isFavorite: boolean
	/** Owned by whoever knows the launch state; the card only reflects it. */
	launching: boolean
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	isAuxiliary?: boolean
	/** Rendered only while the menu is open. Returning `null` keeps the card actionless. */
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
