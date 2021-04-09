import type { DynamicSvelteComponent } from "sveltelib/dynamicComponent";
import type { SvelteComponentDev } from "svelte/internal";

interface ToolbarItem<T extends typeof SvelteComponentDev = typeof SvelteComponentDev>
    extends DynamicSvelteComponent<T> {
    id?: string;
    hidden?: boolean;
}
