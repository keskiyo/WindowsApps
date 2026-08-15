import { ToggleTrack } from '../../../../shared/ui/ToggleTrack'
import type { SettingsToggleProps } from '../../types'

export function SettingsToggle({
	label,
	checked,
	disabled = false,
	onToggle,
}: SettingsToggleProps) {
	return (
		<button
			type="button"
			role="switch"
			aria-label={label}
			aria-checked={checked}
			disabled={disabled}
			onClick={onToggle}
			className="shrink-0 rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:opacity-50"
		>
			<ToggleTrack checked={checked} />
		</button>
	)
}
