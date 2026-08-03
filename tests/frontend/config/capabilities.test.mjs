import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const capabilities = JSON.parse(
	readFileSync('src-tauri/capabilities/default.json', 'utf8'),
)
const updaterHook = readFileSync('src/features/update-app/model/useUpdater.ts', 'utf8')

// The webview is untrusted, so a capability is an attack surface, not a convenience. `*:default`
// bundles every operation a plugin offers: `process:default` also grants `exit`, and
// `updater:default` grants `download-and-install`. The frontend uses neither.
describe('Tauri capabilities', () => {
	it('grants only the process and updater operations the frontend calls', () => {
		expect(capabilities.permissions).toEqual([
			'core:default',
			'core:window:allow-start-dragging',
			'core:window:allow-minimize',
			'core:window:allow-toggle-maximize',
			'core:window:allow-close',
			'dialog:allow-open',
			'updater:allow-check',
			'updater:allow-download',
			'updater:allow-install',
			'process:allow-restart',
		])
	})

	it('does not grant a plugin default bundle', () => {
		const bundles = capabilities.permissions.filter(
			permission =>
				permission.endsWith(':default') && !permission.startsWith('core:'),
		)

		expect(bundles).toEqual([])
	})

	// The grant and the call site have to move together: dropping a plugin call without dropping
	// its permission silently leaves the surface open.
	it('matches the updater operations the hook actually performs', () => {
		expect(updaterHook).toContain('check(')
		expect(updaterHook).toContain('.download(')
		expect(updaterHook).toContain('.install(')
		expect(updaterHook).toContain('relaunch()')
		// `exit` and the combined download-and-install are not used and are not granted.
		expect(updaterHook).not.toContain('downloadAndInstall')
		expect(updaterHook).not.toContain('exit(')
	})
})
