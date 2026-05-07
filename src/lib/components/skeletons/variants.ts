export type VariantInfo = {
	slug: string;
	label: string;
	tagline: string;
};

export const VARIANTS: readonly VariantInfo[] = [
	{ slug: 'v1-stack', label: 'v1 · Stack', tagline: 'Spec-literal' },
	{ slug: 'v2-sidebar', label: 'v2 · Sidebar', tagline: 'Persistent rail' },
	{ slug: 'v3-tabs', label: 'v3 · Tabs', tagline: 'Tabbed controls' },
	{ slug: 'v4-resizable', label: 'v4 · Resizable', tagline: 'Split panes' },
	{ slug: 'v5-drawer', label: 'v5 · Drawer', tagline: 'Mobile-first' },
	{ slug: 'v6-accordion', label: 'v6 · Accordion', tagline: 'Compact' },
];
