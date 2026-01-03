import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    root: "./e2e",
    include: ["**/*.test.ts"],
    globals: true,
    testTimeout: 30000,
    hookTimeout: 10000,
    setupFiles: ["./setup.ts"],
    pool: "forks",
    poolOptions: {
      forks: {
        singleFork: false,
      },
    },
    reporters: ["verbose"],
  },
});
