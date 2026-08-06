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
			className={`relative h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:opacity-50 ${checked ? 'utility-accent-button' : 'settings-toggle-track'}`}
		>
			<span
				className={`settings-toggle-knob absolute top-1 left-1 size-5 rounded-full shadow transition-transform ${checked ? 'translate-x-5' : 'translate-x-0'}`}
			/>
		</button>
	)
}
