import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/app/styles/index.css', 'utf8')

// Off-screen cards skip layout/paint via CSS content-visibility — the dependency-free virtualization
// that keeps large catalogs smooth. This locks the mechanism (and its menu-clip escape hatch) so a
// refactor cannot silently drop it.
describe('card virtualization styles', () => {
	it('applies content-visibility to off-screen cards with a stable placeholder size', () => {
		const rule = stylesheet.match(/\.cv-card\s*\{([^}]*)\}/)?.[1] ?? ''
		expect(rule).toContain('content-visibility: auto')
		expect(rule).toContain('contain-intrinsic-size')
	})

	// The scenario picker offers the whole catalog in one list, so it leans on the same mechanism
	// instead of revealing rows in batches the user cannot always reach.
	it('applies the same skip to scenario picker rows', () => {
		const rule = stylesheet.match(/\.picker-row\s*\{([^}]*)\}/)?.[1] ?? ''
		expect(rule).toContain('content-visibility: auto')
		expect(rule).toContain('contain-intrinsic-size')
	})

	it('lifts containment while a card menu is open so the popover is not clipped', () => {
		const rule =
			stylesheet.match(
				/\.app-card\[data-menu-open\]\s*\{([^}]*)\}/,
			)?.[1] ?? ''
		expect(rule).toContain('content-visibility: visible')
		expect(rule).toContain('contain: none')
	})
})
