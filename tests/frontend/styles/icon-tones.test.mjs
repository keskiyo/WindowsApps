import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import * as lucide from 'lucide-react'
import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/app/styles/index.css', 'utf8')

function sourceFiles(directory, found = []) {
	for (const entry of readdirSync(directory)) {
		const full = join(directory, entry)
		if (statSync(full).isDirectory()) sourceFiles(full, found)
		else if (/\.tsx?$/.test(entry)) found.push(full)
	}
	return found
}

/** Every lucide component the production UI imports, whatever alias name it is imported under. */
function importedIconNames() {
	const names = new Set()
	for (const file of sourceFiles('src')) {
		const text = readFileSync(file, 'utf8')
		for (const statement of text.matchAll(
			/import\s*(?:type\s*)?\{([^}]*)\}\s*from\s*'lucide-react'/gs,
		))
			for (const raw of statement[1].split(',')) {
				const name = raw.trim()
				if (name && name !== 'LucideIcon' && name !== 'type LucideIcon')
					names.add(name)
			}
	}
	return [...names].sort()
}

/** The class lucide actually puts on the rendered `<svg>` — the hook the stylesheet targets. */
function renderedIconClass(name) {
	const Icon = lucide[name]
	expect(Icon, `lucide-react exports ${name}`).toBeTruthy()
	const markup = renderToStaticMarkup(createElement(Icon))
	const className = markup.match(/class="([^"]*)"/)?.[1] ?? ''
	const token = className.split(/\s+/).find(part => /^lucide-.+/.test(part))
	expect(token, `${name} renders a lucide-* class`).toBeTruthy()
	return token
}

function tonedClasses() {
	return new Set(
		[...stylesheet.matchAll(/\.(lucide-[a-z0-9-]+)/g)].map(match => match[1]),
	)
}

/**
 * The icon palette is applied from `index.css` by lucide's generated class rather than per call
 * site, so an icon added to a component picks up a colour only if the stylesheet knows its name.
 * Nothing in the type system links the two; this test is that link.
 */
describe('icon tones', () => {
	it('gives every icon the UI renders a palette colour', () => {
		const toned = tonedClasses()
		const untoned = importedIconNames().filter(
			name => !toned.has(renderedIconClass(name)),
		)

		expect(untoned).toEqual([])
	})

	it('paints from the shared category palette, never a raw colour', () => {
		for (const [, declaration] of stylesheet.matchAll(
			/\.lucide-[a-z0-9-]+[^{]*\{([^}]*)\}/g,
		))
			expect(declaration.trim()).toMatch(
				/^color:\s*(var\(--category-[a-z]+\)|inherit);$/,
			)
	})

	it('lets a stateful control keep control of its own icon colour', () => {
		// Window buttons, solid accent buttons, destructive controls, the favorite toggle and
		// disabled controls encode state or severity through colour; a tone there fights the label.
		expect(stylesheet).toMatch(/\.app-titlebar \.lucide/)
		expect(stylesheet).toMatch(/\.icon-follows-color \.lucide/)
		expect(stylesheet).toMatch(/\.danger-button \.lucide/)
		expect(stylesheet).toMatch(/button:disabled \.lucide/)
	})
})
