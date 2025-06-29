import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: 'http://localhost:8080',
				changeOrigin: true,
			},
			'/ws': {
				target: 'ws://localhost:8080',
				ws: true,
				changeOrigin: true,
			}
		}
	}
});
