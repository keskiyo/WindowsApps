import type { RefObject } from 'react'
import type { UpdateInstallPhase } from '../../model/useUpdater'

export interface UpdateBannerProps {
	version: string
	onOpen(): void
	onDismiss(): void
}

export interface UpdateDialogProps {
	version: string
	date: string | null
	packageSize: number | null
	releaseUrl: string | null
	notes: string | null
	installing: boolean
	progress: number | null
	downloadedBytes: number
	totalBytes: number | null
	phase: UpdateInstallPhase
	error: string | null
	onInstall(): void
	onDismiss(): void
	onOpenRelease(): void
}

export interface DialogHeaderProps {
	version: string
	releaseDate: string | null
	packageSize: number | null
	installing: boolean
	closeButtonRef: RefObject<HTMLButtonElement>
	onDismiss(): void
}

export interface HighlightsListProps {
	highlights: string[]
	releaseUrl: string | null
	onOpenRelease(): void
}

export interface InstallProgressProps {
	phase: UpdateInstallPhase
	progress: number | null
	downloadedBytes: number
	effectiveTotal: number | null
}

export interface DialogFooterProps {
	phase: UpdateInstallPhase
	installing: boolean
	installLabel: string
	onInstall(): void
	onDismiss(): void
}
