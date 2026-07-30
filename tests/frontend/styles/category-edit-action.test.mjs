import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/index.css', 'utf8')

describe('category edit action styles', () => {
	it('does not paint a background on hover', () => {
		expect(stylesheet).not.toContain('.category-edit-action:hover')
	})
})
