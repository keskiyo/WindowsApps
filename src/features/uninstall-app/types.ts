import type { UninstallPreview } from '../../entities/app'

export interface UninstallDialogProps {
	appName: string
	preview: UninstallPreview | null
	isPreviewLoading: boolean
	previewError: string | null
	onClose(): void
	onConfirm(): Promise<void>
}
