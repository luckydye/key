import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
	plugins: [solid()],

	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
	},
}));
