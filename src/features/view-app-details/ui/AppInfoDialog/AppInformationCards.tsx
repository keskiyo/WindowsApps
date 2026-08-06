import { Box, FolderOpen, Search, ShieldCheck } from 'lucide-react'
import {
	type AppDetails,
	type AppInfo,
	SOURCE_LABELS,
} from '../../../../entities/app'
import {
	type CategoryDefinition,
	categoryLabel,
} from '../../../../entities/category'
import { InfoCard } from './InfoCard'

interface AppInformationCardsProps {
	app: AppInfo
	categories: CategoryDefinition[]
	details: AppDetails | null
	isLoading: boolean
}

function valueOrUnavailable(value: string | null | undefined): string {
	return value?.trim() || 'Not available'
}

function availability(
	value: boolean | null | undefined,
	isLoading: boolean,
): string {
	if (value == null) return isLoading ? 'Checking…' : 'Not available'
	return value ? 'Yes' : 'No'
}

function signature(
	status: AppDetails['signature'] | undefined,
	isLoading: boolean,
): string {
	if (status === 'verified') return 'Verified'
	if (status === 'unsigned') return 'Unsigned'
	return status === 'unavailable' || !isLoading ? 'Not available' : 'Checking…'
}

function DetailRows({ rows }: { rows: [string, string][] }) {
	return (
		<dl className='grid grid-cols-[minmax(6.5rem,auto)_minmax(0,1fr)] gap-x-3 gap-y-2 text-sm leading-5'>
			{rows.map(([label, value]) => (
				<div key={label} className='contents'>
					<dt className='text-(--text-subtle)'>{label}</dt>
					<dd className='min-w-0 break-words text-(--text-primary)'>
						{value}
					</dd>
				</div>
			))}
		</dl>
	)
}

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
	const targetLabel = hasResolvedStartAppTarget ? 'Launch target' : 'Executable'
	const targetPath = hasResolvedStartAppTarget ? app.resolvedPath : app.path
	return (
		<div className='grid gap-3 md:grid-cols-2'>
			<InfoCard icon={Box} title='Application'>
				<DetailRows
					rows={[
						['Version', valueOrUnavailable(app.version)],
						['Publisher', valueOrUnavailable(app.publisher)],
						['Category', categoryLabel(categories, app.category)],
					]}
				/>
			</InfoCard>
			<InfoCard icon={FolderOpen} title='Installation'>
				<DetailRows
					rows={[
						['Location', valueOrUnavailable(app.installLocation)],
						[targetLabel, valueOrUnavailable(targetPath)],
						['Launch type', app.launchKind.replace(/_/g, ' ')],
					]}
				/>
			</InfoCard>
			<InfoCard icon={ShieldCheck} title='Status'>
				<DetailRows
					rows={[
						[
							'Installed',
							hasKnownPackageInstallLocation
									? 'Yes'
									: availability(details?.installLocationExists, isLoading),
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
			<InfoCard icon={Search} title='Detection'>
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
