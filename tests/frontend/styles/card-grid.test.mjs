import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
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

function sourceFiles(directory, found = []) {
	for (const entry of readdirSync(directory)) {
		const full = join(directory, entry)
		if (statSync(full).isDirectory()) sourceFiles(full, found)
		else if (/\.tsx?$/.test(entry)) found.push(full)
	}
	return found
}

/**
 * The catalog tile has a fixed size and the grid lays out as many fixed tracks as fit. That only
 * works while the track and the tile are the same width — if they drift, every row either clips
 * its last tile or leaves a column-sized hole. The value lives in one token; this test is what
 * keeps both users of it pointed at that token.
 */
describe('catalog card grid', () => {
	it('sizes the grid track from the same token as the tile', () => {
		expect(rule('.app-card-grid')).toContain(
			'repeat(auto-fill, var(--app-card-width))',
		)
		expect(rule('.app-card-tile')).toContain('width: var(--app-card-width)')
		expect(rule('.app-card-tile')).toContain('height: var(--app-card-height)')
	})

	it('centers fixed tracks with a compact shared gap', () => {
		expect(stylesheet).toContain('--app-card-gap: 0.625rem;')
		expect(rule('.app-card-grid')).toContain('gap: var(--app-card-gap)')
		expect(rule('.app-card-grid')).toContain('justify-content: center')
	})

	it('uses a single quiet edge and a thinner interactive spotlight', () => {
		const glass = rule('.app-card-tile.app-card-glass')
		expect(glass).not.toContain('0 0 0 1px')
		expect(glass).toContain(
			'0 8px 22px oklch(0.58 0.14 292 / 0.06)',
		)
		expect(rule('.app-card-tile.app-card-glass::after')).toContain(
			'opacity: 0.25',
		)
		expect(rule('.app-card-tile .spotlight::before')).toContain(
			'padding: 1.4px',
		)
	})

	it('keeps the quieter visual treatment scoped away from auxiliary rows', () => {
		expect(rule('.app-card-glass')).toContain(
			'0 10px 30px oklch(0.58 0.14 292 / 0.12)',
		)
		expect(rule('.app-card-glass::after')).toContain('opacity: 0.5')
		expect(rule('.spotlight::before')).toContain('padding: 2.4px')
	})

	it('reserves the real tile height for off-screen cards', () => {
		// A placeholder taller or shorter than the card makes the scrollbar jump as cards mount.
		expect(rule('.cv-card')).toContain(
			'contain-intrinsic-size: auto var(--app-card-height)',
		)
	})

	it('lets the catalog column count follow the width instead of fixed breakpoints', () => {
		// Every catalog surface (categories, favorites, hidden, artifacts, the loading skeleton)
		// goes through `.app-card-grid`. A breakpoint-pinned column list here would resize the
		// tile again and is what this layout replaced.
		const offenders = [
			...sourceFiles('src/widgets/catalog-content'),
			...sourceFiles('src/entities/app'),
		].filter(file => /\b(sm|md|lg|xl):grid-cols-/.test(readFileSync(file, 'utf8')))

		expect(offenders).toEqual([])
	})

	it('keeps Favorites applications on the shared auto-fitting grid', () => {
		const favorites = readFileSync(
			'src/widgets/catalog-content/ui/FavoritesGrid.tsx',
			'utf8',
		)

		expect(favorites).toContain('className="app-card-grid"')
		expect(favorites).not.toContain('favorites-app-card-grid')
	})

	it('lays out favorite scenarios in one, two, then three columns', () => {
		const scenarios = readFileSync(
			'src/features/manage-scenarios/ui/FavoriteScenarioList/FavoriteScenarioList.tsx',
			'utf8',
		)

		expect(scenarios).toContain('min-[781px]:grid-cols-2')
		expect(scenarios).toContain('min-[1601px]:grid-cols-3')
		expect(scenarios).toContain('items-start')
	})
})
