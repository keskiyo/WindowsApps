import {
	AppGrid,
	type AppGridProps,
} from '../../../widgets/catalog-content'
import {
	WorkspaceSummary,
	type WorkspaceSummaryProps,
} from '../../../widgets/workspace-summary'
import { ScanPrompt } from './ScanPrompt'

export interface CatalogPageProps {
	/** An empty first run has nothing to summarize or grid, so it offers a scan instead. */
	showScanPrompt: boolean
	scanPrompt: {
		isScanning: boolean
		onScan(): Promise<void>
		onDismiss(): void
	}
	summary: WorkspaceSummaryProps
	grid: AppGridProps
}

export function CatalogPage({
	showScanPrompt,
	scanPrompt,
	summary,
	grid,
}: CatalogPageProps) {
	if (showScanPrompt) return <ScanPrompt {...scanPrompt} />
	return (
		<>
			{!grid.isLoading && <WorkspaceSummary {...summary} />}
			<AppGrid {...grid} />
		</>
	)
}
