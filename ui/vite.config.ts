import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ command }) => ({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  // public/ only holds the dev-mock tool list; keep it out of the embedded bundle.
  publicDir: command === "serve" ? "public" : false,
  build: { target: "chrome105", sourcemap: false },
}));
