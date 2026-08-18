import { fireEvent, render, screen } from '@testing-library/react'
import { useRef, type RefObject } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { useModalDialog } from '../../../src/shared/hooks/useModalDialog'

interface PanelProps {
	restoreFocusTo?: RefObject<HTMLButtonElement>
	onDismiss?: () => void
	dismissible?: boolean
}

function Panel({ restoreFocusTo, onDismiss, dismissible }: PanelProps) {
	const dialogRef = useRef<HTMLDivElement>(null)
	const closeRef = useRef<HTMLButtonElement>(null)
	useModalDialog({
		ref: dialogRef,
		initialFocusRef: closeRef,
		restoreFocusTo,
		onDismiss,
		dismissible,
	})
	return (
		<div ref={dialogRef} role="dialog" aria-modal="true" aria-label="Panel">
			<button type="button">First</button>
			<button type="button" ref={closeRef}>
				Close
			</button>
		</div>
	)
}

interface HarnessProps extends Omit<PanelProps, 'restoreFocusTo'> {
	open?: boolean
	withRestoreTarget?: boolean
}

function Harness({
	open = true,
	withRestoreTarget = false,
	...panel
}: HarnessProps) {
	const restoreRef = useRef<HTMLButtonElement>(null)
	return (
		<div>
			<button type="button" ref={restoreRef}>
				Restore target
			</button>
			{open && (
				<Panel
					{...panel}
					restoreFocusTo={withRestoreTarget ? restoreRef : undefined}
				/>
			)}
		</div>
	)
}

describe('useModalDialog', () => {
	it('focuses the requested control and hands focus back to the opener', () => {
		render(<button type="button">Opener</button>)
		const opener = screen.getByRole('button', { name: 'Opener' })
		opener.focus()

		const view = render(<Harness />)
		expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus()

		view.unmount()
		expect(opener).toHaveFocus()
	})

	it('prefers an explicit restore target over the opener', () => {
		render(<button type="button">Opener</button>)
		screen.getByRole('button', { name: 'Opener' }).focus()

		const view = render(<Harness withRestoreTarget />)
		const restoreTarget = screen.getByRole('button', {
			name: 'Restore target',
		})

		view.rerender(<Harness withRestoreTarget open={false} />)
		expect(restoreTarget).toHaveFocus()
	})

	it('keeps the opener it captured when the dialog re-renders', () => {
		render(<button type="button">Opener</button>)
		const opener = screen.getByRole('button', { name: 'Opener' })
		opener.focus()

		const view = render(<Harness dismissible />)
		screen.getByRole('button', { name: 'First' }).focus()
		view.rerender(<Harness dismissible={false} />)

		view.unmount()
		expect(opener).toHaveFocus()
	})

	it('dismisses on Escape only while the dialog allows it', () => {
		const onDismiss = vi.fn()
		const view = render(<Harness onDismiss={onDismiss} />)

		fireEvent.keyDown(document, { key: 'Escape' })
		expect(onDismiss).toHaveBeenCalledOnce()

		view.rerender(<Harness onDismiss={onDismiss} dismissible={false} />)
		fireEvent.keyDown(document, { key: 'Escape' })
		expect(onDismiss).toHaveBeenCalledOnce()
	})

	it('leaves Escape alone when the dialog owns the key itself', () => {
		render(<Harness />)

		fireEvent.keyDown(document, { key: 'Escape' })

		expect(screen.getByRole('dialog')).toBeInTheDocument()
	})

	it('wraps Tab inside the dialog', () => {
		render(<Harness />)
		const first = screen.getByRole('button', { name: 'First' })
		const close = screen.getByRole('button', { name: 'Close' })

		close.focus()
		fireEvent.keyDown(document, { key: 'Tab' })
		expect(first).toHaveFocus()

		fireEvent.keyDown(document, { key: 'Tab', shiftKey: true })
		expect(close).toHaveFocus()
	})
})
