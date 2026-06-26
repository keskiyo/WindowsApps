import { describe, expect, it } from 'vitest'
import {
	getDropAction,
	groupAppsByCategory,
	sortFavoritesFirst,
} from '../../lib/catalog'
import type { AppInfo } from '../../types'

const apps = [
	{ id: 'wow', name: 'World of Warcraft', category: 'games' },
	{ id: 'code', name: 'Code', category: 'development' },
] as AppInfo[]

describe('catalog utilities', () => {
	it('groups applications without mutating their order', () => {
		const original = [...apps]
		const groups = groupAppsByCategory(apps)
		expect(groups.get('games')).toEqual([apps[0]])
		expect(groups.get('development')).toEqual([apps[1]])
		expect(apps).toEqual(original)
	})

	it('moves favorites to the front while keeping the rest stable', () => {
		const games = [
			{ id: 'steam', name: 'Steam', category: 'games' },
			{ id: 'telegram', name: 'Telegram', category: 'games' },
			{ id: 'discord', name: 'Discord', category: 'games' },
		] as AppInfo[]
		const ordered = sortFavoritesFirst(games, ['telegram'])
		expect(ordered.map(app => app.id)).toEqual([
			'telegram',
			'steam',
			'discord',
		])
		// Original array is not mutated.
		expect(games[0].id).toBe('steam')
	})

	it('routes an application drop to a category', () => {
		expect(
			getDropAction(
				{ type: 'app', appId: 'wow', category: 'other' },
				{ type: 'category', category: 'games' },
			),
		).toEqual({ type: 'move-app', appId: 'wow', category: 'games' })
	})
})
