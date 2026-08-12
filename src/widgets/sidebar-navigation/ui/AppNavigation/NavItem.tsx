import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { navigationItemClass } from './data'
import type { NavItemProps } from './types'

export function NavItem({
	icon: Icon,
	label,
	active,
	count,
	secondaryCount,
	className,
	onClick,
}: NavItemProps) {
	const spotlight = useSpotlight()

	return (
		<button
			type="button"
			onClick={onClick}
			aria-current={active ? 'page' : undefined}
			aria-label={
				secondaryCount === undefined
					? count === undefined
						? label
						: `${label} ${count}`
					: `${label} ${count ?? 0} apps, ${secondaryCount} scenarios`
			}
			title={label}
			{...spotlight}
			className={`${navigationItemClass(active)}${className ? ` ${className}` : ''}`}
		>
			<SpotlightLayer size={90} />
			<Icon className="shrink-0" size={17} />{' '}
			<span className="min-w-0 truncate">{label}</span>
			{(count !== undefined || secondaryCount !== undefined) && (
				<span className="ml-auto shrink-0 rounded-md border border-(--border-neutral) bg-(--surface-inset) px-1.5 py-0.5 text-xs text-(--text-muted)">
					{count ?? 0}
					{secondaryCount !== undefined && ` · ${secondaryCount}`}
				</span>
			)}
		</button>
	)
}
