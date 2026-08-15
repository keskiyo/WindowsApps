import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsToggle } from '../../../../src/pages/settings/ui/components/SettingsToggle'

describe('SettingsToggle', () => {
	it('reports its state and toggles', async () => {
		const onToggle = vi.fn()
		render(
			<SettingsToggle
				label="Launch at startup"
				checked
				onToggle={onToggle}
			/>,
		)

		const toggle = screen.getByRole('switch', { name: 'Launch at startup' })
		expect(toggle).toBeChecked()
		await userEvent.click(toggle)
		expect(onToggle).toHaveBeenCalledOnce()
	})

	it('does nothing while disabled', async () => {
		const onToggle = vi.fn()
		render(
			<SettingsToggle
				label="Launch at startup"
				checked={false}
				disabled
				onToggle={onToggle}
			/>,
		)

		const toggle = screen.getByRole('switch', { name: 'Launch at startup' })
		expect(toggle).not.toBeChecked()
		await userEvent.click(toggle)
		expect(onToggle).not.toHaveBeenCalled()
	})

	/**
	 * The knob used `bg-slate-50`, which the dark-theme compatibility layer rewrites to
	 * `--surface-raised` — a near-black dot on the violet track. Its colour now comes from a class
	 * that layer cannot match.
	 */
	it('keeps the knob out of the reach of the dark-theme override', () => {
		render(
			<SettingsToggle
				label="Launch at startup"
				checked
				onToggle={vi.fn()}
			/>,
		)

		const knob = screen
			.getByRole('switch')
			.querySelector('.settings-toggle-knob')
		expect(knob).not.toBeNull()
		expect(knob?.className).not.toMatch(/bg-(white|slate-(50|100|200))/)
	})
})
