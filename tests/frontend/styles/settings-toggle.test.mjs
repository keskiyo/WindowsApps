import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/app/styles/index.css', 'utf8')

function rule(selector) {
	const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`)
	const body = stylesheet.match(
		new RegExp(String.raw`(^|\})\s*${escaped}\s*\{([^}]*)\}`, 'm'),
	)?.[2]
	expect(body, `${selector} exists`).toBeTruthy()
	return body
}

/**
 * The knob used `bg-slate-50`, and `.theme-graphite-surface [class*='bg-slate-50']` rewrites that
 * to `--surface-raised` with `!important` — so an enabled switch showed a near-black dot on the
 * violet track. Its colour now comes from a class the compatibility layer cannot match.
 */
describe('settings toggle colours', () => {
	it('paints the knob near-white rather than from a surface token', () => {
		const knob = rule('.settings-toggle-knob')

		expect(knob).toMatch(/background:\s*oklch\(0\.9\d/)
		expect(knob).not.toContain('--surface')
	})

	it('gives the off track a dark surface so the same knob stays readable', () => {
		expect(rule('.settings-toggle-track')).toContain('var(--surface-inset)')
	})

	// The class names themselves are the fix: anything matching the override is back to square one.
	it('uses class names the dark-theme override cannot match', () => {
		for (const name of ['settings-toggle-knob', 'settings-toggle-track'])
			expect(name).not.toMatch(/bg-(white|slate-(50|100|200))/)
	})
})
