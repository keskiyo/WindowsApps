import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { VpnPage } from '../../../components/vpn/VpnPage'
import type { VpnClient } from '../../../types'

function client(over: Partial<VpnClient> = {}): VpnClient {
	return {
		list: vi.fn().mockResolvedValue([
			{ id: 'hiddify', name: 'Hiddify', installed: true, connected: false },
		]),
		set: vi.fn().mockImplementation(async (id, enabled) => ({
			id,
			name: 'Hiddify',
			installed: true,
			connected: enabled,
		})),
		setup: vi
			.fn()
			.mockResolvedValue({ id: 'hiddify', name: 'Hiddify', installed: true, connected: false }),
		...over,
	}
}

describe('VpnPage', () => {
	it('toggles a provider on', async () => {
		const api = client()
		render(<VpnPage client={api} />)
		const toggle = await screen.findByRole('switch', { name: /hiddify/i })
		await userEvent.click(toggle)
		expect(api.set).toHaveBeenCalledWith('hiddify', true)
	})

	it('offers setup when not installed', async () => {
		const api = client({
			list: vi.fn().mockResolvedValue([
				{ id: 'hiddify', name: 'Hiddify', installed: false, connected: false },
			]),
		})
		render(<VpnPage client={api} />)
		await userEvent.click(await screen.findByRole('button', { name: /set up hiddify/i }))
		expect(api.setup).toHaveBeenCalledWith('hiddify')
	})
})
