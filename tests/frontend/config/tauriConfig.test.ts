import { describe, expect, it } from 'vitest'
import tauriConfig from '../../../src-tauri/tauri.conf.json'

describe('Tauri window configuration', () => {
	it('allows the catalog window to shrink to the two-card layout', () => {
		expect(tauriConfig.app.windows[0]?.minWidth).toBe(560)
	})

	// The size is fixed at startup on purpose: nothing persists the window geometry, so every
	// launch opens at this size and resizing stays a per-session choice.
	it('opens at the chosen size and still lets the user resize', () => {
		const window = tauriConfig.app.windows[0]

		expect(window?.width).toBe(1132)
		expect(window?.height).toBe(715)
		expect(window?.resizable).toBe(true)
		expect(window?.width).toBeGreaterThanOrEqual(window?.minWidth ?? 0)
		expect(window?.height).toBeGreaterThanOrEqual(window?.minHeight ?? 0)
	})
})
