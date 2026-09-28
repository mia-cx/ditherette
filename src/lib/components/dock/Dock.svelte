<script lang="ts">
	import { mount, onMount, unmount, type Component, type Snippet } from 'svelte';
	import {
		DockviewComponent,
		type ContextMenuItem,
		type DockviewApi,
		type IContentRenderer
	} from 'dockview-core';
	import DockGroupActions from './DockGroupActions.svelte';
	import SnippetHost from './SnippetHost.svelte';
	import {
		canCollapse,
		collapsedGroups,
		isFloating,
		sidebarSide,
		toggleCollapsed,
		toggleSidebar,
		toggleFloating,
		trackCollapse,
		type DockParams
	} from './dock';
	import './dock.css';

	type Props = {
		/** One snippet per window component name; it renders that window's content. */
		panels: Readonly<Record<string, Snippet<[DockParams]>>>;
		/** Shown when every window is closed. */
		empty?: Snippet<[DockParams]>;
		/** Set up against the API; the returned cleanup runs before the dock is disposed. */
		onready: (api: DockviewApi) => (() => void) | void;
		/** The window whose column is the main area; every other docked column is a sidebar. */
		main?: string;
		class?: string;
	};
	let { panels, empty, onready, main, class: className = '' }: Props = $props();

	let container = $state<HTMLDivElement>();

	/** Mount a Svelte component into an element Dockview owns, and unmount it with the window. */
	function hosted<P extends Record<string, unknown>>(component: Component<P>, props: P) {
		const element = document.createElement('div');
		element.className = 'h-full';
		const instance = mount(component, { target: element, props });
		return { element, dispose: () => void unmount(instance) };
	}

	function content(name: string): IContentRenderer {
		const snippet = panels[name];
		if (!snippet) throw new Error(`Unknown window: ${name}`);
		const element = document.createElement('div');
		element.className = 'h-full overflow-auto';
		let instance: ReturnType<typeof mount> | undefined;
		return {
			element,
			init: ({ params }) => {
				instance = mount(SnippetHost, { target: element, props: { snippet, params } });
			},
			dispose: () => {
				if (instance) void unmount(instance);
			}
		};
	}

	onMount(() => {
		const dock = new DockviewComponent(container!, {
			theme: { name: 'ditherette', className: 'dockview-theme-ditherette' },
			floatingGroupBounds: 'boundedWithinViewport',
			createComponent: ({ name }) => content(name),
			createRightHeaderActionComponent: () => {
				let actions: ReturnType<typeof hosted> | undefined;
				const element = document.createElement('div');
				element.className = 'h-full';
				return {
					element,
					init: ({ containerApi, group }) => {
						actions = hosted(DockGroupActions, { api: containerApi, group, main });
						element.appendChild(actions.element);
					},
					dispose: () => actions?.dispose()
				};
			},
			createWatermarkComponent: () => {
				const view = empty ? hosted(SnippetHost, { snippet: empty, params: {} }) : undefined;
				return {
					element: view?.element ?? document.createElement('div'),
					init: () => {},
					dispose: () => view?.dispose()
				};
			},
			getTabContextMenuItems: ({ group, api }): ContextMenuItem[] => [
				...(sidebarSide(api, group, main)
					? [
							{
								label:
									collapsedGroups.get()[group.id]?.axis === 'width'
										? 'Expand sidebar'
										: 'Collapse sidebar',
								action: () => toggleSidebar(api, group, main)
							}
						]
					: []),
				...(canCollapse(api, group) && collapsedGroups.get()[group.id]?.axis !== 'width'
					? [
							{
								label: collapsedGroups.get()[group.id] ? 'Expand window' : 'Collapse window',
								action: () => toggleCollapsed(api, group, main)
							}
						]
					: []),
				{
					label: isFloating(group) ? 'Dock window' : 'Float window',
					action: () => toggleFloating(api, group, main)
				},
				'maximize',
				'separator',
				'close',
				'closeOthers'
			]
		});
		// Lay out at the real size first, so the first layout keeps its initial widths.
		const { width, height } = container!.getBoundingClientRect();
		dock.layout(width, height);
		const cleanup = onready(dock.api);
		const stopCollapse = trackCollapse(dock.api);
		return () => {
			stopCollapse();
			cleanup?.();
			dock.dispose();
		};
	});
</script>

<div bind:this={container} class="h-full w-full {className}"></div>
