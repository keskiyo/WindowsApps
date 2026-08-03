import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import type { NavItemProps } from './types'

function itemClass(active: boolean) {
	return `relative flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-sm transition-[background-color,border-color,box-shadow] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-strong)] motion-reduce:transition-none ${active ? 'border-[var(--accent)] bg-[var(--utility-accent)] font-medium text-[var(--text-primary)] shadow-[var(--shadow-active-navigation)]' : 'border-[var(--border-neutral)] bg-[var(--surface-panel)] text-[var(--text-primary)] hover:bg-[var(--surface-raised)]'}`
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
				<span className='ml-auto shrink-0 rounded-md border border-[var(--border-neutral)] bg-[var(--surface-inset)] px-1.5 py-0.5 text-xs text-[var(--text-muted)]'>
					{count}
				</span>
			)}
		</button>
	)
}
