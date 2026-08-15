import type { ToggleTrackProps } from './types'

export function ToggleTrack({ checked }: ToggleTrackProps) {
	return (
		<span
			className={`relative block h-7 w-12 shrink-0 rounded-full transition ${checked ? 'utility-accent-button' : 'settings-toggle-track'}`}
		>
			<span
				className={`settings-toggle-knob absolute top-1 left-1 size-5 rounded-full shadow transition-transform ${checked ? 'translate-x-5' : 'translate-x-0'}`}
			/>
		</span>
	)
}
