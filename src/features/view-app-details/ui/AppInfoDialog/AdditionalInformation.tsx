import { ChevronDown, Info } from 'lucide-react'
import { useId, useState } from 'react'
import { formatFileDate, formatFileSize } from '../../../../entities/app'
import { availability, loadingValue, signature } from './detailValues'
import type { AdditionalInformationProps } from './types'

export function AdditionalInformation({
	details,
	isLoading,
	hasKnownPackageInstallLocation,
	hasResolvedStartAppTarget,
}: AdditionalInformationProps) {
	const [expanded, setExpanded] = useState(false)
	const contentId = useId()
	const rows: [string, string][] = [
		[
			'File size',
			loadingValue(
				formatFileSize(details?.fileSizeBytes ?? null),
				isLoading,
			),
		],
		[
			'Architecture',
			loadingValue(
				details?.architecture === 'notApplicable'
					? 'Not applicable'
					: details?.architecture &&
						  details.architecture !== 'unknown'
						? details.architecture
						: 'Not available',
				isLoading,
			),
		],
		[
			'Created',
			loadingValue(
				formatFileDate(details?.fileCreatedAt ?? null),
				isLoading,
			),
		],
		[
			'Modified',
			loadingValue(
				formatFileDate(details?.fileModifiedAt ?? null),
				isLoading,
			),
		],
		['Digital signature', signature(details?.signature, isLoading)],
		[
			hasResolvedStartAppTarget
				? 'Launch target found'
				: 'Executable found',
			availability(details?.executableExists, isLoading),
		],
		[
			'Install location found',
			hasKnownPackageInstallLocation
				? 'Yes'
				: availability(details?.installLocationExists, isLoading),
		],
	]
	return (
		<section className="rounded-2xl border border-(--border-neutral) bg-(--surface-inset)">
			<button
				type="button"
				aria-expanded={expanded}
				aria-controls={contentId}
				onClick={() => setExpanded(value => !value)}
				className="flex min-h-14 w-full items-center gap-3 rounded-2xl px-4 text-left text-(--text-primary) transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--accent-strong)"
			>
				<Info size={20} aria-hidden="true" />
				<span className="flex-1 font-semibold">
					Additional information
				</span>
				<ChevronDown
					size={20}
					className={`text-(--text-muted) transition-transform duration-(--motion-fast) motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
					aria-hidden="true"
				/>
			</button>
			<div
				id={contentId}
				aria-hidden={!expanded}
				className={`grid transition-[grid-template-rows,opacity] duration-(--motion-medium) ease-(--motion-ease-out) motion-reduce:transition-none ${expanded ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}
			>
				<div className="overflow-hidden">
					<dl className="grid grid-cols-1 border-t border-(--border-neutral) sm:grid-cols-2">
						{rows.map(([label, value]) => (
							<div
								key={label}
								className="grid grid-cols-[minmax(8rem,auto)_minmax(0,1fr)] gap-3 border-b border-(--border-neutral) px-4 py-3 text-sm last:border-b-0 sm:[&:nth-last-child(2)]:border-b-0"
							>
								<dt className="text-(--text-subtle)">
									{label}
								</dt>
								<dd className="break-words text-(--text-primary)">
									{value}
								</dd>
							</div>
						))}
					</dl>
				</div>
			</div>
		</section>
	)
}
