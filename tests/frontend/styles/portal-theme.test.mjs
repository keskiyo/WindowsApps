import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

function sourceFiles(directory, found = []) {
	for (const entry of readdirSync(directory)) {
		const full = join(directory, entry)
		if (statSync(full).isDirectory()) sourceFiles(full, found)
		else if (/\.tsx$/.test(entry)) found.push(full)
	}
	return found
}

/** Classes the graphite layer in `index.css` rewrites, and only inside `.theme-graphite-surface`. */
const COMPATIBILITY_CLASSES =
	/\b(bg-white|bg-slate-(50|100|200)|bg-violet-100|bg-amber-(50|100)|border-white|border-slate-(200|300)|divide-slate-200|text-slate-(400|500|600|700|800|900)|text-violet-(500|600|700))\b/

/**
 * The delete-scenario dialog rendered as a white card on the graphite desktop: it portals to
 * `document.body`, which is outside `.theme-graphite-surface`, so every compatibility rewrite the
 * light palette depends on stopped applying. A component that leaves that subtree has to paint
 * from tokens. Portalling into `.app-shell` instead — what the card actions menu does — keeps the
 * rewrites and is equally fine.
 */
describe('portalled surfaces', () => {
	it('paints from tokens once it leaves the themed subtree', () => {
		const offenders = sourceFiles('src')
			.map(file => ({ file, text: readFileSync(file, 'utf8') }))
			.filter(entry => entry.text.includes('createPortal'))
			.filter(entry => !entry.text.includes('app-shell'))
			.filter(entry => COMPATIBILITY_CLASSES.test(entry.text))
			.map(entry => entry.file)

		expect(offenders).toEqual([])
	})
})
