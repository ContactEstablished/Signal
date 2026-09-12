import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
// App components opt into runes; dependencies may still ship legacy Svelte syntax.
export default { preprocess: vitePreprocess() };
