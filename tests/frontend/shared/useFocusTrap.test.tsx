import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useRef } from 'react'
import { describe, expect, it } from 'vitest'
import { useFocusTrap } from '../../../src/shared/hooks/useFocusTrap'

function Trapped({ disabledMiddle = false }: { disabledMiddle?: boolean }) {
	const dialog = useRef<HTMLDivElement>(null)
	useFocusTrap(dialog)
	return (
		<>
			<button type='button'>outside before</button>
			<div ref={dialog} role='dialog' aria-modal='true' aria-label='Trapped'>
				<button type='button'>first</button>
				<button type='button' disabled={disabledMiddle}>
					middle
				</button>
				<button type='button'>last</button>
			</div>
			<button type='button'>outside after</button>
		</>
	)
}

describe('useFocusTrap', () => {
	// The hook used to filter candidates by `offsetParent`, which jsdom never provides — the set
	// collapsed to the focused element alone and the trap quietly did nothing. These assertions
	// only hold while the trap actually cycles.
	it('wraps forward from the last element to the first', async () => {
		const user = userEvent.setup()
		render(<Trapped />)
		screen.getByRole('button', { name: 'last' }).focus()

		await user.tab()

		expect(screen.getByRole('button', { name: 'first' })).toHaveFocus()
	})

	it('wraps backward from the first element to the last', async () => {
		const user = userEvent.setup()
		render(<Trapped />)
		screen.getByRole('button', { name: 'first' }).focus()

		await user.tab({ shift: true })

		expect(screen.getByRole('button', { name: 'last' })).toHaveFocus()
	})

	it('never lands on an element outside the trapped container', async () => {
		const user = userEvent.setup()
		render(<Trapped />)
		screen.getByRole('button', { name: 'first' }).focus()

		for (let step = 0; step < 6; step += 1) {
			await user.tab()
			expect(document.activeElement).not.toBe(
				screen.getByRole('button', { name: 'outside after' }),
			)
			expect(document.activeElement).not.toBe(
				screen.getByRole('button', { name: 'outside before' }),
			)
		}
	})

	it('skips a disabled control when wrapping', async () => {
		const user = userEvent.setup()
		render(<Trapped disabledMiddle />)
		screen.getByRole('button', { name: 'last' }).focus()

		await user.tab()

		expect(screen.getByRole('button', { name: 'first' })).toHaveFocus()
	})
})
