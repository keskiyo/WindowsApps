import { AppWindow, BadgeCheck, X } from 'lucide-react'
import {
	type AppDetails,
	type AppInfo,
	displayVersion,
} from '../../../../entities/app'
import {
	type CategoryDefinition,
	categoryLabel,
} from '../../../../entities/category'

interface AppIdentityHeaderProps {
	app: AppInfo
	categories: CategoryDefinition[]
	details: AppDetails | null
	closeRef(node: HTMLButtonElement | null): void
	onClose(): void
}

export function AppIdentityHeader({
	app,
	categories,
	details,
	closeRef,
	onClose,
}: AppIdentityHeaderProps) {
	return (
		<header className='flex items-start gap-4 sm:gap-5'>
			<span className='grid size-17 shrink-0 place-items-center overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-raised) shadow-(--shadow-accent-soft) sm:size-20'>
				{app.iconBase64 ? (
					<img src={app.iconBase64} alt='' className='size-12 object-contain' />
				) : (
					<AppWindow
						size={32}
						className='text-(--text-muted)'
						aria-hidden='true'
					/>
				)}
			</span>
			<div className='min-w-0 flex-1 pt-0.5'>
				<div className='flex flex-wrap items-center gap-x-2 gap-y-1'>
					<h2 className='truncate text-xl font-semibold tracking-tight text-(--text-primary) sm:text-2xl'>
						{app.name}
					</h2>
					{details?.signature === 'verified' && (
						<BadgeCheck
							size={19}
							className='shrink-0 text-(--accent-strong)'
							aria-label='Verified digital signature'
						/>
					)}
				</div>
				<p className='mt-1 line-clamp-2 text-sm leading-6 text-(--text-muted)'>
					{app.description?.trim() || 'No description available'}
				</p>
				<div className='mt-3 flex flex-wrap gap-2 text-sm'>
					{app.version && (
						<span className='rounded-lg border border-(--border-neutral) bg-(--surface-inset) px-2.5 py-1 text-(--text-muted)'>
							v{displayVersion(app.version)}
						</span>
					)}
					<span className='rounded-lg border border-(--border-neutral) bg-[color-mix(in_oklab,var(--accent)_15%,var(--surface-raised))] px-2.5 py-1 text-(--accent-strong)'>
						{categoryLabel(categories, app.category)}
					</span>
				</div>
			</div>
			<button
			ref={closeRef}
			type='button'
			aria-label='Close app information'
			onClick={onClose}
			className='grid size-11 shrink-0 place-items-center rounded-xl border border-transparent text-(--text-muted) transition-colors hover:border-(--border-neutral) hover:bg-(--surface-raised) hover:text-(--text-primary) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)'
		>
			<X size={20} aria-hidden='true' />
		</button>
		</header>
	)
}
