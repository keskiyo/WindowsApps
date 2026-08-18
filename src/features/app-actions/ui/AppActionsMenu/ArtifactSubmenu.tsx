import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { ARTIFACT_DESTINATIONS, SUBMENU_PANEL } from './data'
import type { ArtifactSubmenuProps } from './types'

export function ArtifactSubmenu({
	onSelect,
	menuRef,
	position,
	onKeyDown,
	label,
}: ArtifactSubmenuProps) {
	const spotlight = useSpotlight()

	return (
		<div
			ref={menuRef}
			onKeyDown={onKeyDown}
			style={position}
			role="menu"
			aria-label={label}
			className={SUBMENU_PANEL}
		>
			{ARTIFACT_DESTINATIONS.map(destination => (
				<button
					key={destination.kind}
					type="button"
					role="menuitem"
					onClick={() => onSelect(destination.kind)}
					{...spotlight}
					className="relative flex w-full items-center rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-slate-500/15 focus-visible:outline-2 focus-visible:outline-violet-500"
				>
					<SpotlightLayer size={60} />
					{destination.label}
				</button>
			))}
		</div>
	)
}
