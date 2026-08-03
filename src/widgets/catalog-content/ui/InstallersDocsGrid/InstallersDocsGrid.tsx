import { ArtifactSection } from './ArtifactSection'
import type { InstallersDocsGridProps } from './types'

export function InstallersDocsGrid({
	apps,
	hasQuery,
	...actions
}: InstallersDocsGridProps) {
	const installers = []
	const docs = []
	for (const app of apps) {
		if (app.artifactKind === 'installer') installers.push(app)
		if (app.artifactKind === 'documentation') docs.push(app)
	}
	if (!apps.length)
		return (
			<section className='grid min-h-[55vh] place-items-center text-center'>
				<div>
					<h2 className='text-lg font-semibold'>
						{hasQuery
							? 'No matching installers or docs'
							: 'No installers or docs found'}
					</h2>
					<p className='mt-2 text-sm text-[var(--text-muted)]'>
						{hasQuery
							? 'Try a different search.'
							: 'Refresh the catalog to scan supported locations.'}
					</p>
				</div>
			</section>
		)
	return (
		<div aria-label='Installers and documentation' className='space-y-7'>
			{installers.length > 0 && (
				<ArtifactSection title='Installers' apps={installers} {...actions} />
			)}
			{docs.length > 0 && <ArtifactSection title='Docs' apps={docs} {...actions} />}
		</div>
	)
}
