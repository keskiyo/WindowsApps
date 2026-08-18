import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useAppDerivations } from '../../../src/app/model/useAppDerivations'
import { appIdentity, type AppInfo } from '../../../src/entities/app'

function app(name: string, overrides: Partial<AppInfo> = {}): AppInfo {
	return {
		id: name.toLowerCase(),
		name,
		path: `C:\\${name}.exe`,
		iconBase64: null,
		category: 'utilities',
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...overrides,
	}
}

function derive(apps: AppInfo[], firstSeenAt: Record<string, number>) {
	return renderHook(() =>
		useAppDerivations({
			catalogApps: apps,
			primaryApps: apps,
			firstSeenAt,
			scenarios: [],
			favoriteScenarioIds: [],
		}),
	).result.current
}

function seenAt(apps: AppInfo[], at: number): Record<string, number> {
	return Object.fromEntries(apps.map(entry => [appIdentity(entry), at]))
}

describe('useAppDerivations', () => {
	it('lists the newest catalog entries first', () => {
		const apps = [app('Older'), app('Newer')]

		const { recentApps } = derive(apps, {
			older: 1_000,
			newer: 2_000,
		})

		expect(recentApps.map(entry => entry.app.name)).toEqual([
			'Newer',
			'Older',
		])
	})

	// A first scan stamps every record with the same Date.now(), so the tie-break is what the
	// reader actually sees on a fresh install. It is alphabetical, matching the other
	// "Recently added" lists on the More page rather than raw catalog order.
	it('breaks a shared first-seen timestamp alphabetically', () => {
		const apps = [app('Zulu'), app('Alpha'), app('Mike')]

		const { recentApps } = derive(apps, seenAt(apps, 1_700_000_000_000))

		expect(recentApps.map(entry => entry.app.name)).toEqual([
			'Alpha',
			'Mike',
			'Zulu',
		])
	})

	it('leaves out records the catalog has never dated', () => {
		const apps = [app('Dated'), app('Undated')]

		const { recentApps } = derive(apps, { dated: 1_000 })

		expect(recentApps).toHaveLength(1)
		expect(recentApps[0]).toEqual({ app: apps[0], firstSeenAt: 1_000 })
	})

	it('keeps the recent list bounded', () => {
		const apps = Array.from({ length: 25 }, (_, index) =>
			app(`App ${String(index).padStart(2, '0')}`),
		)

		const { recentApps } = derive(apps, seenAt(apps, 5_000))

		expect(recentApps).toHaveLength(20)
	})

	it('keeps only the records no rule could classify', () => {
		const apps = [
			app('Filed', { category: 'utilities' }),
			app('Unfiled', { category: 'other' }),
		]

		const { unclassifiedApps } = derive(apps, {})

		expect(unclassifiedApps.map(entry => entry.name)).toEqual(['Unfiled'])
	})
})
