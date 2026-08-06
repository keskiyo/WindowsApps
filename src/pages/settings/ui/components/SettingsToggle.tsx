interface Props {
	label: string
	checked: boolean
	disabled?: boolean
	onToggle(): void
}

/**
 * The settings switch, shared by every toggle on the page.
 *
 * It carries its own colours rather than Tailwind's slate scale: the dark-theme compatibility
 * layer rewrites `bg-slate-50` and friends to `--surface-raised`, which turned the knob into a
 * near-black dot on the violet track. `settings-toggle-*` is invisible to that rule, so the knob
 * stays near-white on both tracks.
 */
export function SettingsToggle({
	label,
	checked,
	disabled = false,
	onToggle,
}: Props) {
	return (
		<button
			type='button'
			role='switch'
			aria-label={label}
			aria-checked={checked}
			disabled={disabled}
			onClick={onToggle}
			className={`relative h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:opacity-50 ${checked ? 'utility-accent-button' : 'settings-toggle-track'}`}
		>
			<span
				className={`settings-toggle-knob absolute left-1 top-1 size-5 rounded-full shadow transition-transform ${checked ? 'translate-x-5' : 'translate-x-0'}`}
			/>
		</button>
	)
}
