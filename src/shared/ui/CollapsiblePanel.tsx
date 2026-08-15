import {
	useEffect,
	useState,
	type ReactNode,
	type TransitionEvent,
} from 'react'

interface CollapsiblePanelProps {
	open: boolean
	id?: string
	children: ReactNode
	className?: string
}

export function CollapsiblePanel({
	open,
	id,
	children,
	className = '',
}: CollapsiblePanelProps) {
	const [present, setPresent] = useState(open)
	const [visible, setVisible] = useState(open)

	useEffect(() => {
		if (open) {
			setPresent(true)
			let visibleFrame: number | undefined
			const mountFrame = requestAnimationFrame(() => {
				visibleFrame = requestAnimationFrame(() => setVisible(true))
			})
			return () => {
				cancelAnimationFrame(mountFrame)
				if (visibleFrame !== undefined)
					cancelAnimationFrame(visibleFrame)
			}
		}
		setVisible(false)
		const timeout = window.setTimeout(() => setPresent(false), 250)
		return () => window.clearTimeout(timeout)
	}, [open])

	function onTransitionEnd(event: TransitionEvent<HTMLDivElement>) {
		if (!open && event.propertyName === 'grid-template-rows')
			setPresent(false)
	}

	if (!present) return null
	return (
		<div
			ref={panel => panel?.toggleAttribute('inert', !open)}
			id={id}
			aria-hidden={!open}
			onTransitionEnd={onTransitionEnd}
			className={`grid transition-[grid-template-rows,opacity] duration-(--motion-medium) ease-(--motion-ease-out) motion-reduce:transition-none ${visible ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}
		>
			<div
				className={`overflow-hidden transition-transform duration-(--motion-medium) ease-(--motion-ease-out) motion-reduce:transition-none ${visible ? 'translate-y-0' : '-translate-y-2'}${className ? ` ${className}` : ''}`}
			>
				{children}
			</div>
		</div>
	)
}
