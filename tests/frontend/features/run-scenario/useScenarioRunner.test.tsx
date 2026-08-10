import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useScenarioRunner } from '../../../../src/features/run-scenario'
import {
	MAX_SCENARIO_ENTRIES,
	type Scenario,
} from '../../../../src/entities/scenario'
import type { AppInfo, CloseAppsResult } from '../../../../src/entities/app'

function app(id: string): AppInfo {
	return {
		id,
		name: id,
		path: `C:\\Apps\\${id}.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

function scenario(value: Partial<Scenario> = {}): Scenario {
	return {
		id: 'gaming',
		name: 'Gaming',
		launchIdentities: [],
		closeIdentities: [],
		createdAt: null,
		...value,
	}
}

function closed(count: number): CloseAppsResult {
	return { closed: count, notRunning: 0, unavailable: 0, blocked: 0, failed: 0 }
}

function setup(options: {
	apps: AppInfo[]
	scenarios?: Scenario[]
	launch?: (app: AppInfo) => Promise<void>
	closeApps?: (ids: string[]) => Promise<CloseAppsResult>
}) {
	const launch = vi.fn(options.launch ?? (async () => undefined))
	const closeApps = vi.fn(
		options.closeApps ?? (async (ids: string[]) => closed(ids.length)),
	)
	const onFinished = vi.fn()
	const view = renderHook(() =>
		useScenarioRunner({
			apps: options.apps,
			scenarios: options.scenarios ?? [],
			launch,
			closeApps,
			onFinished,
		}),
	)
	return { view, launch, closeApps, onFinished }
}

describe('useScenarioRunner', () => {
	it('launches the launch list, then closes the close list', async () => {
		const order: string[] = []
		const { view, onFinished } = setup({
			apps: [app('game'), app('chat')],
			launch: vi.fn(async (entry: AppInfo) => {
				order.push(`launch:${entry.id}`)
			}),
			closeApps: vi.fn(async (ids: string[]) => {
				order.push(`close:${ids.join(',')}`)
				return closed(ids.length)
			}),
		})

		await act(async () => {
			await view.result.current.run(
				scenario({ launchIdentities: ['game'], closeIdentities: ['chat'] }),
			)
		})

		expect(order).toEqual(['launch:game', 'close:chat'])
		expect(onFinished).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'gaming' }),
			expect.objectContaining({
				launched: 1,
				closed: 1,
				notRunning: 0,
				unavailable: 0,
				blocked: 0,
				failed: 0,
			}),
		)
	})

	// The backend enumerates once and waits out a single grace period, so the whole close list has
	// to arrive as one request; one call per app would make the user wait that period per app.
	it('closes the whole list in a single request', async () => {
		const { view, closeApps } = setup({
			apps: [app('chat'), app('mail'), app('music')],
		})

		await act(async () => {
			await view.result.current.run(
				scenario({ closeIdentities: ['chat', 'mail', 'music'] }),
			)
		})

		expect(closeApps).toHaveBeenCalledOnce()
		expect(closeApps).toHaveBeenCalledWith(['chat', 'mail', 'music'])
	})

	it('asks for no close at all when the list is empty', async () => {
		const { view, closeApps } = setup({ apps: [app('game')] })

		await act(async () => {
			await view.result.current.run(scenario({ launchIdentities: ['game'] }))
		})

		expect(closeApps).not.toHaveBeenCalled()
	})

	// "It was already closed" is the outcome the scenario wanted, not a failure to report.
	it('reports apps that were not running apart from failures', async () => {
		const { view, onFinished } = setup({
			apps: [app('chat'), app('store-app')],
			closeApps: vi.fn(async () => ({
				closed: 0,
				notRunning: 1,
				unavailable: 1,
		blocked: 0,
		failed: 0,
			})),
		})

		await act(async () => {
			await view.result.current.run(
				scenario({ closeIdentities: ['chat', 'store-app'] }),
			)
		})

		expect(onFinished).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
			launched: 0,
			closed: 0,
			notRunning: 1,
			unavailable: 1,
		blocked: 0,
		failed: 0,
		}))
	})

	it('counts entries the catalog no longer has as unavailable', async () => {
		const { view, launch, closeApps, onFinished } = setup({
			apps: [app('game')],
		})

		await act(async () => {
			await view.result.current.run(
				scenario({
					launchIdentities: ['game', 'uninstalled'],
					closeIdentities: ['also-gone'],
				}),
			)
		})

		expect(launch).toHaveBeenCalledTimes(1)
		expect(closeApps).not.toHaveBeenCalled()
		expect(onFinished).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
			launched: 1,
			closed: 0,
			notRunning: 0,
			unavailable: 2,
		blocked: 0,
		failed: 0,
		}))
	})

	// A scenario is a batch: one app that refuses to start must not abort the rest of it.
	it('keeps going when one entry fails and reports it', async () => {
		const { view, onFinished } = setup({
			apps: [app('broken'), app('game')],
			launch: vi.fn(async (entry: AppInfo) => {
				if (entry.id === 'broken') throw new Error('no')
			}),
		})

		await act(async () => {
			await view.result.current.run(
				scenario({ launchIdentities: ['broken', 'game'] }),
			)
		})

		expect(onFinished).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
			launched: 1,
			closed: 0,
			notRunning: 0,
			unavailable: 1,
			blocked: 0,
		}))
	})

	// A failed close request is one failure per app it was supposed to close, not one overall.
	it('reports every app of a close request that could not be made', async () => {
		const { view, onFinished } = setup({
			apps: [app('chat'), app('mail')],
			closeApps: vi.fn(async () => {
				throw new Error('no')
			}),
		})

		await act(async () => {
			await view.result.current.run(
				scenario({ closeIdentities: ['chat', 'mail'] }),
			)
		})

		expect(onFinished).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
			launched: 0,
			closed: 0,
			notRunning: 0,
			unavailable: 2,
			blocked: 0,
		}))
	})

	it('never starts more than the entry cap, whatever the scenario holds', async () => {
		const apps = Array.from({ length: MAX_SCENARIO_ENTRIES + 5 }, (_, index) =>
			app(`a${index}`),
		)
		const { view, launch } = setup({ apps })

		await act(async () => {
			await view.result.current.run(
				scenario({ launchIdentities: apps.map(entry => entry.id) }),
			)
		})

		expect(launch).toHaveBeenCalledTimes(MAX_SCENARIO_ENTRIES)
	})

	it('refuses a second run while the first is still going', async () => {
		let release = () => {}
		const gate = new Promise<void>(resolve => {
			release = resolve
		})
		const { view, launch } = setup({
			apps: [app('game')],
			launch: vi.fn(() => gate),
		})

		let first: Promise<void> | undefined
		act(() => {
			first = view.result.current.run(scenario({ launchIdentities: ['game'] }))
		})
		await waitFor(() => expect(view.result.current.runningId).toBe('gaming'))
		// A second click while apps are still starting would double every launch.
		await act(async () => {
			await view.result.current.run(scenario({ launchIdentities: ['game'] }))
		})
		expect(launch).toHaveBeenCalledTimes(1)

		await act(async () => {
			release()
			await first
		})
		expect(view.result.current.runningId).toBeNull()
	})

	describe('runById', () => {
		it('runs the scenario with that id', async () => {
			const gaming = scenario({ launchIdentities: ['game'] })
			const { view, launch } = setup({
				apps: [app('game')],
				scenarios: [gaming],
			})

			await act(async () => {
				view.result.current.runById('gaming')
			})

			expect(launch).toHaveBeenCalledOnce()
		})

		// The More card previews a snapshot of the list; a scenario deleted in between must not
		// make the button throw.
		it('does nothing for an id the list no longer holds', async () => {
			const { view, launch } = setup({ apps: [app('game')], scenarios: [] })

			await act(async () => {
				view.result.current.runById('gaming')
			})

			expect(launch).not.toHaveBeenCalled()
		})
	})
})
