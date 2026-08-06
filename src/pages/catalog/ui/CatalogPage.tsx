import {
	AppGrid,
	type AppGridProps,
} from '../../../widgets/catalog-content'
import { ScanPrompt } from './ScanPrompt'

export interface CatalogPageProps {
	/** An empty first run has nothing to grid, so it offers a scan instead. */
	showScanPrompt: boolean
	scanPrompt: {
		isScanning: boolean
		onScan(): Promise<void>
		onDismiss(): void
	}
	grid: AppGridProps
}

export function CatalogPage({
	showScanPrompt,
	scanPrompt,
	grid,
}: CatalogPageProps) {
	if (showScanPrompt) return <ScanPrompt {...scanPrompt} />
	return <AppGrid {...grid} />
}
