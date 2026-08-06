import type { AppGridProps } from '../../widgets/catalog-content'

export interface CatalogPageProps {
	showScanPrompt: boolean
	scanPrompt: ScanPromptProps
	grid: AppGridProps
}

export interface ScanPromptProps {
	isScanning: boolean
	onScan(): Promise<void>
	onDismiss(): void
}
