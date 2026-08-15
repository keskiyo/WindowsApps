import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

function sourceFiles(directory) {
	const files = []
	for (const entry of readdirSync(directory)) {
		const full = join(directory, entry)
		if (statSync(full).isDirectory()) files.push(...sourceFiles(full))
		else if (/\.tsx$/.test(entry)) files.push(full)
	}
	return files
}

const REWRITTEN = /bg-(white|slate-(50|100|200))\b/

/**
 * The dark-theme compatibility layer matches on the class *string*: `[class*='bg-white']` also
 * matches `hover:bg-white/45`, and its `!important` declaration then paints the control in the
 * resting state. A control that carries the colour only as a hover variant therefore sits
 * permanently filled — which is what happened to the unrecognised-applications header.
 */
describe('hover fills that the theme layer would make permanent', () => {
	it('never uses a rewritten surface colour as a hover-only variant', () => {
		const offenders = []
		for (const file of sourceFiles('src')) {
			for (const [, classes] of readFileSync(file, 'utf8').matchAll(
				/className=(?:"([^"]*)"|\{`([^`]*)`\})/gs,
			)) {
				const value = classes ?? ''
				const tokens = value.split(/\s+/).filter(Boolean)
				const hovered = tokens.filter(
					token =>
						token.startsWith('hover:') && REWRITTEN.test(token),
				)
				const resting = tokens.some(
					token => !token.includes(':') && REWRITTEN.test(token),
				)
				if (hovered.length > 0 && !resting)
					offenders.push(`${file}: ${hovered.join(' ')}`)
			}
		}

		expect(offenders).toEqual([])
	})
})
