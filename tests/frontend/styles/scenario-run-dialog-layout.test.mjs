import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const dialog = readFileSync(
	'src/features/manage-scenarios/ui/ScenarioRunDialog/ScenarioRunDialog.tsx',
	'utf8',
)

describe('scenario run dialog layout', () => {
	it('uses a desktop width that still fits the supported minimum window width', () => {
		expect(dialog).toContain('w-[min(42rem,calc(100vw-2rem))]')
		expect(dialog).not.toContain('min-w-xl')
	})
})
