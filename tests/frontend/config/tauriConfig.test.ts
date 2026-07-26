import { describe, expect, it } from 'vitest'
import tauriConfig from '../../../src-tauri/tauri.conf.json'

describe('Tauri window configuration', () => {
	it('allows the catalog window to shrink to the two-card layout', () => {
		expect(tauriConfig.app.windows[0]?.minWidth).toBe(560)
	})
})
