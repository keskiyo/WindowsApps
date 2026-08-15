import { act, fireEvent, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CollapsiblePanel } from '../../../src/shared/ui/CollapsiblePanel'

function Example() {
	const [open, setOpen] = useState(false)
	return (
		<>
			<button type="button" onClick={() => setOpen(value => !value)}>
				Toggle details
			</button>
			<CollapsiblePanel open={open} id="details-panel">
				<button type="button">Details</button>
			</CollapsiblePanel>
		</>
	)
}

describe('CollapsiblePanel', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('renders the closed state for a frame before opening', () => {
		const frames: FrameRequestCallback[] = []
		vi.stubGlobal(
			'requestAnimationFrame',
			(callback: FrameRequestCallback) => {
				frames.push(callback)
				return frames.length
			},
		)
		vi.stubGlobal('cancelAnimationFrame', vi.fn())
		render(<Example />)

		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))
		const panel = screen.getByText('Details').parentElement?.parentElement
		expect(panel).toHaveClass('grid-rows-[0fr]', 'opacity-0')

		act(() => frames.shift()?.(0))
		expect(panel).toHaveClass('grid-rows-[0fr]', 'opacity-0')

		act(() => frames.shift()?.(16))
		expect(panel).toHaveClass('grid-rows-[1fr]', 'opacity-100')
	})

	it('keeps content present for the close transition and then unmounts it', () => {
		render(<Example />)
		expect(screen.queryByText('Details')).not.toBeInTheDocument()

		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))
		const panel = screen.getByText('Details').parentElement?.parentElement
		expect(panel).toHaveAttribute('id', 'details-panel')
		expect(panel).toHaveClass('transition-[grid-template-rows,opacity]')

		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))
		expect(screen.getByText('Details')).toBeInTheDocument()
		fireEvent.transitionEnd(panel!, { propertyName: 'grid-template-rows' })
		expect(screen.queryByText('Details')).not.toBeInTheDocument()
	})

	it('collapses its own padding and slides the content with the height', () => {
		function Padded() {
			const [open, setOpen] = useState(false)
			return (
				<>
					<button type="button" onClick={() => setOpen(true)}>
						Toggle details
					</button>
					<CollapsiblePanel open={open} className="pt-5">
						<button type="button">Details</button>
					</CollapsiblePanel>
				</>
			)
		}
		render(<Padded />)

		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))
		const content = screen.getByText('Details').parentElement

		expect(content).toHaveClass('pt-5', 'overflow-hidden')
		expect(content).toHaveClass('transition-transform')
		expect(content?.parentElement).not.toHaveClass('pt-5')
	})

	it('makes closing content inert before its transition ends', () => {
	render(<Example />)
		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))
		const panel = screen.getByText('Details').parentElement?.parentElement

		fireEvent.click(screen.getByRole('button', { name: 'Toggle details' }))

		expect(panel).toHaveAttribute('aria-hidden', 'true')
		expect(panel).toHaveAttribute('inert')
	})
})
