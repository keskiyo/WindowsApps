import { AppGrid } from '../../../widgets/catalog-content'
import type { CatalogPageProps } from '../types'
import { ScanPrompt } from './ScanPrompt'

export function CatalogPage({
	showScanPrompt,
	scanPrompt,
	grid,
}: CatalogPageProps) {
	if (showScanPrompt) return <ScanPrompt {...scanPrompt} />
	return <AppGrid {...grid} />
}
