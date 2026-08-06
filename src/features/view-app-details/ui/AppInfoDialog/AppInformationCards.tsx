import { Box, FolderOpen, Search, ShieldCheck } from 'lucide-react'
import { SOURCE_LABELS } from '../../../../entities/app'
import { categoryLabel } from '../../../../entities/category'
import { DetailRows } from './DetailRows'
import { availability, signature, valueOrUnavailable } from './detailValues'
import { InfoCard } from './InfoCard'
import type { AppInformationCardsProps } from './types'

export function AppInformationCards({
	app,
	categories,
	details,
	isLoading,
}: AppInformationCardsProps) {
	const hasKnownPackageInstallLocation =
		app.sourceKind === 'msix' && Boolean(app.installLocation?.trim())
	const hasResolvedStartAppTarget =
		app.sourceKind === 'start_apps' &&
		app.launchKind === 'app_user_model_id' &&
		Boolean(app.resolvedPath?.trim())
	const targetLabel = hasResolvedStartAppTarget
		? 'Launch target'
		: 'Executable'
	const targetPath = hasResolvedStartAppTarget ? app.resolvedPath : app.path
	return (
		<div className="grid gap-3 md:grid-cols-2">
			<InfoCard icon={Box} title="Application">
				<DetailRows
					rows={[
						['Version', valueOrUnavailable(app.version)],
						['Publisher', valueOrUnavailable(app.publisher)],
						['Category', categoryLabel(categories, app.category)],
					]}
				/>
			</InfoCard>
			<InfoCard icon={FolderOpen} title="Installation">
				<DetailRows
					rows={[
						['Location', valueOrUnavailable(app.installLocation)],
						[targetLabel, valueOrUnavailable(targetPath)],
						['Launch type', app.launchKind.replace(/_/g, ' ')],
					]}
				/>
			</InfoCard>
			<InfoCard icon={ShieldCheck} title="Status">
				<DetailRows
					rows={[
						[
							'Installed',
							hasKnownPackageInstallLocation
								? 'Yes'
								: availability(
										details?.installLocationExists,
										isLoading,
									),
						],
						[
							hasResolvedStartAppTarget
								? 'Launch target found'
								: 'Executable found',
							availability(details?.executableExists, isLoading),
						],
						['Signature', signature(details?.signature, isLoading)],
					]}
				/>
			</InfoCard>
			<InfoCard icon={Search} title="Detection">
				<DetailRows
					rows={[
						['Source', SOURCE_LABELS[app.sourceKind]],
						[
							'Original filename',
							valueOrUnavailable(app.originalFilename),
						],
						[
							'Catalog state',
							app.visibilityClass === 'auxiliary'
								? 'Auxiliary tool'
								: 'Primary application',
						],
					]}
				/>
			</InfoCard>
		</div>
	)
}
