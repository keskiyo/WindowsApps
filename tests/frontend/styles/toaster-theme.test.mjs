import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const stylesheet = readFileSync('src/app/styles/index.css', 'utf8')
const app = readFileSync('src/app/App.tsx', 'utf8')

function rule(selector) {
	const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`)
	const body = stylesheet.match(
		new RegExp(String.raw`(^|\})\s*${escaped}\s*\{([^}]*)\}`, 'm'),
	)?.[2]
	expect(body, `${selector} exists`).toBeTruthy()
	return body
}

describe('toaster theme', () => {
	it('uses the dark Sonner base instead of the default light palette', () => {
		expect(app).toMatch(/<Toaster[\s\S]*theme="dark"/)
		expect(app).not.toMatch(/<Toaster[\s\S]*richColors/)
	})

	it('renders every toast on the graphite application surface', () => {
		const toast = rule(
			`.app-toaster [data-sonner-toast][data-styled='true']`,
		)
		expect(toast).toContain('background: var(--surface-raised) !important')
		expect(toast).toContain('color: var(--text-primary) !important')
		expect(toast).toContain(
			'border: 1px solid var(--border-neutral) !important',
		)
	})

	it('keeps semantic colour on the status icon instead of the whole toast', () => {
		expect(
			rule(
				`.app-toaster [data-sonner-toast][data-type='success'] [data-icon]`,
			),
		).toContain('var(--category-green)')
		expect(
			rule(
				`.app-toaster [data-sonner-toast][data-type='error'] [data-icon]`,
			),
		).toContain('var(--category-red)')
	})
})
