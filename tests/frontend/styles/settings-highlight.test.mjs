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
 * The dark-theme compatibility layer matches on the class *string*, so `[class*='bg-violet-100']`
 * also matched `hover:bg-violet-100/55` — and its `!important` declaration applied in the resting
 * state. Every use of that colour in the app is a hover variant, so the Telegram row and the
 * dialog dismiss buttons sat permanently lit instead of highlighting under the pointer.
 */
describe('violet highlight compatibility rule', () => {
	it('only paints while the pointer is on the control', () => {
		expect(rule(`.theme-graphite-surface [class*='bg-violet-100']:hover`)).toContain(
			'background-color',
		)
		expect(stylesheet).not.toMatch(
			/\.theme-graphite-surface \[class\*='bg-violet-100'\]\s*\{/,
		)
	})

	// A highlight lifts the surface; it does not replace it. The fill used to be near-opaque.
	it('is a tint rather than a fill', () => {
		const alpha = rule(
			`.theme-graphite-surface [class*='bg-violet-100']:hover`,
		).match(/oklch\([^)]*\/\s*([\d.]+)\s*\)/)?.[1]

		expect(Number(alpha)).toBeLessThan(0.35)
	})
})
