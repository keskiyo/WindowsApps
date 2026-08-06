import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import type { NavItemProps } from './types'

function itemClass(active: boolean) {
	return `relative flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-sm transition-[background-color,border-color,box-shadow] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none ${active ? 'border-(--accent) bg-(--utility-accent) font-medium text-(--text-primary) shadow-(--shadow-active-navigation)' : 'border-(--border-neutral) bg-(--surface-panel) text-(--text-primary) hover:bg-(--surface-raised)'}`
}

export function NavItem({
	icon: Icon,
	label,
	active,
	count,
	className,
	onClick,
}: NavItemProps) {
	const spotlight = useSpotlight()

	return (
		<button
			type='button'
			onClick={onClick}
			aria-current={active ? 'page' : undefined}
			aria-label={count === undefined ? label : `${label} ${count}`}
			title={label}
			{...spotlight}
			className={`${itemClass(active)}${className ? ` ${className}` : ''}`}
		>
			<SpotlightLayer size={90} />
			<Icon className='shrink-0' size={17} />{' '}
			<span className='min-w-0 truncate'>{label}</span>
			{count !== undefined && (
				<span className='ml-auto shrink-0 rounded-md border border-(--border-neutral) bg-(--surface-inset) px-1.5 py-0.5 text-xs text-(--text-muted)'>
					{count}
				</span>
			)}
		</button>
	)
}
