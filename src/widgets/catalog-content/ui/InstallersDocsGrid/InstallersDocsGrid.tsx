import { Box } from 'lucide-react'
import { ArtifactSection } from './ArtifactSection'
import { CatalogViewHeader } from '../CatalogViewHeader'
import type { InstallersDocsGridProps } from './types'

export function InstallersDocsGrid({
	apps,
	hasQuery,
	onBack,
	...actions
}: InstallersDocsGridProps) {
	const installers = []
	const docs = []
	for (const app of apps) {
		if (app.artifactKind === 'installer') installers.push(app)
		if (app.artifactKind === 'documentation') docs.push(app)
	}
	return (
		<section aria-labelledby="installers-docs-title">
			<CatalogViewHeader
				icon={Box}
				title="Installers & Docs"
				titleId="installers-docs-title"
				count={apps.length}
				noun="item"
				back={{ label: 'Back to More', onBack }}
			/>
			{apps.length ? (
				<div className="space-y-7">
					{installers.length > 0 && (
						<ArtifactSection
							title="Installers"
							apps={installers}
							{...actions}
						/>
					)}
					{docs.length > 0 && (
						<ArtifactSection
							title="Docs"
							apps={docs}
							{...actions}
						/>
					)}
				</div>
			) : (
				<div className="grid min-h-[45vh] place-items-center text-center">
					<div>
						<h2 className="text-lg font-semibold">
							{hasQuery
								? 'No matching installers or docs'
								: 'No installers or docs found'}
						</h2>
						<p className="mt-2 text-sm text-(--text-muted)">
							{hasQuery
								? 'Try a different search.'
								: 'Refresh the catalog to scan supported locations.'}
						</p>
					</div>
				</div>
			)}
		</section>
	)
}
