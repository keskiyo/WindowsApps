import { Square } from 'lucide-react'
import type { ScenarioStopButtonProps } from '../types'

export function ScenarioStopButton({
	scenarioName,
	onCancel,
}: ScenarioStopButtonProps) {
	return (
		<button
			type="button"
			aria-label={`Stop ${scenarioName}`}
			onClick={onCancel}
			className="grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) text-(--text-primary) transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
		>
			<Square size={14} aria-hidden="true" />
		</button>
	)
}
