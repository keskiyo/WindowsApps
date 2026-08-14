import { describe, expect, it } from 'vitest'
import {
	filterAppsByQuery,
	rankAppsByQuery,
} from '../../../../src/entities/app/lib/catalogSearch'
import type { AppInfo } from '../../../../src/entities/app'

function app(
	value: Partial<AppInfo> & Pick<AppInfo, 'id' | 'name'>,
): AppInfo {
	return {
		path: `C:\\Apps\\${value.id}\\app.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	} as AppInfo
}

const docker = app({
	id: 'docker',
	name: 'Docker Desktop',
	productName: 'Docker Desktop',
})

const terminal = app({
	id: 'terminal',
	name: 'Терминал',
	path: 'Microsoft.WindowsTerminal_8wekyb3d8bbwe!App',
	productName: 'Microsoft.WindowsTerminal',
	launchKind: 'app_user_model_id',
	sourceKind: 'msix',
})

describe('catalog search layout correction', () => {
	it.each(['вщслук', 'вщлук'])(
		'finds Docker from the Russian-layout query %s',
		query => {
			expect(rankAppsByQuery([docker], query)).toEqual([docker])
		},
	)

	it('corrects an English-layout query for a Russian app name', () => {
		const notepad = app({ id: 'notepad', name: 'Блокнот' })

		expect(rankAppsByQuery([notepad], ',kjryjn')).toEqual([notepad])
	})

	it('ranks a literal match above a layout-corrected match', () => {
		const corrected = app({ id: 'corrected', name: 'Вщслук' })

		expect(rankAppsByQuery([corrected, docker], 'docker')).toEqual([
			docker,
			corrected,
		])
	})
})

// Letter-for-letter only. A phonetic spelling such as "гугл" maps to "gugl" and will not reach
// "Google"; matching that would need pronunciation rules and invites false positives.
describe('catalog search transliteration', () => {
	const chrome = app({ id: 'chrome', name: 'Google Chrome' })
	const telegram = app({ id: 'telegram', name: 'Telegram Desktop' })

	it.each([
		['хром', chrome],
		['телеграм', telegram],
	])('finds an English name from the transliterated query %s', (query, expected) => {
		expect(rankAppsByQuery([chrome, telegram], query as string)).toEqual([
			expected,
		])
	})

	it('ranks a literal match above a transliterated one', () => {
		const cyrillic = app({ id: 'cyrillic', name: 'Хром' })

		expect(rankAppsByQuery([cyrillic, chrome], 'хром')).toEqual([
			cyrillic,
			chrome,
		])
	})

	it('leaves a query that needs no transliteration alone', () => {
		expect(filterAppsByQuery([chrome], 'chrome')).toEqual([chrome])
	})
})

describe('Windows Terminal cmd alias', () => {
	const commandPrompt = app({
		id: 'command-prompt',
		name: 'Command Prompt',
		path: String.raw`C:\Menu\Command Prompt.lnk`,
		originalFilename: 'Cmd.Exe.MUI',
	})
	const gitCmd = app({ id: 'git-cmd', name: 'Git CMD' })
	const internalTerminal = app({
		id: 'open-console',
		name: 'Windows Terminal',
		path: String.raw`C:\VSCode\node-pty\OpenConsole.exe`,
		productName: 'Windows Terminal',
	})
	const fakePackage = app({
		id: 'fake-terminal-package',
		name: 'Windows Terminal',
		path: 'Microsoft.WindowsTerminal_FakePublisher!App',
		launchKind: 'app_user_model_id',
		sourceKind: 'msix',
	})
	const candidates = [
		commandPrompt,
		gitCmd,
		internalTerminal,
		fakePackage,
		terminal,
	]

	it.each(['cmd', 'сьв'])(
		'returns only genuine Windows Terminal for %s',
		query => {
			expect(rankAppsByQuery(candidates, query)).toEqual([terminal])
			expect(filterAppsByQuery(candidates, query)).toEqual([terminal])
		},
	)

	it('retains normal full-name searches for auxiliary command tools', () => {
		expect(rankAppsByQuery(candidates, 'command prompt')).toEqual([
			commandPrompt,
		])
		expect(rankAppsByQuery(candidates, 'git cmd')).toEqual([gitCmd])
	})
})

describe('catalog search typo tolerance', () => {
	it.each(['doker', 'dcoker'])(
		'finds Docker with one name edit in %s',
		query => {
			expect(rankAppsByQuery([docker], query)).toEqual([docker])
		},
	)

	it('does not fuzzy-match short or two-edit tokens', () => {
		expect(rankAppsByQuery([docker], 'dkr')).toEqual([])
		expect(rankAppsByQuery([docker], 'dakr')).toEqual([])
	})

	it('does not fuzzy-match a name word shorter than four characters', () => {
		const git = app({ id: 'git', name: 'Git' })

		expect(rankAppsByQuery([git], 'gitx')).toEqual([])
	})

	it('does not fuzzy-match paths or other secondary metadata', () => {
		const pathOnly = app({
			id: 'editor',
			name: 'Editor',
			path: String.raw`C:\Tools\doker\editor.exe`,
		})

		expect(rankAppsByQuery([pathOnly], 'docker')).toEqual([])
	})

	it('keeps AND semantics across corrected and literal tokens', () => {
		expect(filterAppsByQuery([docker], 'вщслук desktop')).toEqual([
			docker,
		])
		expect(filterAppsByQuery([docker], 'вщслук missing')).toEqual([])
	})
})

describe('catalog search metadata', () => {
	it('finds an app by its version', () => {
		const appWithVersion = app({
			id: 'power-toys',
			name: 'PowerToys',
			version: '0.93.1',
		})

		expect(rankAppsByQuery([appWithVersion], '0.93.1')).toEqual([
			appWithVersion,
		])
	})
})
