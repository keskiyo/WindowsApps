import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/app/styles/index.css', 'utf8')
const resultItem = readFileSync(
	'src/features/command-palette/ui/CommandPalette/ResultItem.tsx',
	'utf8',
)

function token(name) {
	const value = stylesheet.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})`))?.[1]
	expect(value, `${name} must be an opaque hex token`).toBeTruthy()
	return value
}

/** WCAG 2.1 relative luminance. */
function luminance(hex) {
	const channels = [1, 3, 5].map(offset => {
		const value = parseInt(hex.slice(offset, offset + 2), 16) / 255
		return value <= 0.03928
			? value / 12.92
			: ((value + 0.055) / 1.055) ** 2.4
	})
	return (
		0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
	)
}

function contrastRatio(foreground, background) {
	const light = Math.max(luminance(foreground), luminance(background))
	const dark = Math.min(luminance(foreground), luminance(background))
	return (light + 0.05) / (dark + 0.05)
}

// The selected result was `text-slate-100` over `bg-violet-500/24` on a light palette surface —
// roughly 1.3:1, because a translucent selection composites against whatever is behind it and its
// real contrast is therefore not a fixed value. The pair is now opaque tokens so the ratio can be
// asserted rather than estimated.
describe('command palette selected-result contrast', () => {
	it('meets WCAG AA for normal text', () => {
		const ratio = contrastRatio(
			token('--color-palette-selected-text'),
			token('--color-palette-selected'),
		)

		expect(ratio).toBeGreaterThanOrEqual(4.5)
	})

	it('applies the checked token pair rather than a translucent overlay', () => {
		const rule =
			stylesheet.match(/\.palette-result-selected\s*\{([^}]*)\}/)?.[1] ?? ''
		expect(rule).toContain('var(--color-palette-selected)')
		expect(rule).toContain('var(--color-palette-selected-text)')

		expect(resultItem).toContain('palette-result-selected')
		// A translucent selection background is what produced the unreadable row.
		expect(resultItem).not.toMatch(/bg-violet-\d+\/\d+/)
	})

	it('proves the contrast helper by rejecting the previous pair', () => {
		// slate-100 on violet-500 at 24% over the palette's slate-50 surface.
		expect(contrastRatio('#f1f5f9', '#cfc4f3')).toBeLessThan(4.5)
	})
})
