import { defineConfig } from "vite";

export default defineConfig({
	server: {
		open: true,
		port: 3333,
	},
	build: {
		outDir: "dist",
	},
});
