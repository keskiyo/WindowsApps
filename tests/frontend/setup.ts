import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

afterEach(cleanup)

/**
 * jsdom has no IntersectionObserver. Progressive card mounting depends on one, so tests get a
 * controllable stub instead: it records its observed nodes and only advances when a test calls
 * `triggerIntersection`. Nothing scrolls on its own, which keeps the mounted-card assertions
 * deterministic.
 */
class TestIntersectionObserver implements IntersectionObserver {
	static instances: TestIntersectionObserver[] = []

	readonly root = null
	readonly rootMargin = ''
	readonly thresholds: ReadonlyArray<number> = []
	private readonly nodes = new Set<Element>()

	constructor(private readonly callback: IntersectionObserverCallback) {
		TestIntersectionObserver.instances.push(this)
	}

	observe(node: Element) {
		this.nodes.add(node)
	}

	unobserve(node: Element) {
		this.nodes.delete(node)
	}

	disconnect() {
		this.nodes.clear()
		TestIntersectionObserver.instances =
			TestIntersectionObserver.instances.filter(
				instance => instance !== this,
			)
	}

	takeRecords(): IntersectionObserverEntry[] {
		return []
	}

	fire() {
		const entries = [...this.nodes].map(
			target =>
				({ target, isIntersecting: true }) as IntersectionObserverEntry,
		)
		if (entries.length) this.callback(entries, this)
	}
}

globalThis.IntersectionObserver =
	TestIntersectionObserver as unknown as typeof IntersectionObserver

/** Scroll every live sentinel into view, mounting the next batch of cards. */
export function triggerIntersection() {
	for (const instance of [...TestIntersectionObserver.instances])
		instance.fire()
}

afterEach(() => {
	TestIntersectionObserver.instances = []
})
