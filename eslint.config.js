// Shared lint rules from eslint-config-agent. Starts on the gentlest preset
// (divisive rules off, everything else warn-level) so `eslint .` stays green
// while the backlog is burned down; tighten to `recommended` once clean.
import recommendedIncremental from "eslint-config-agent/recommended-incremental";

export default [
  ...recommendedIncremental,
  {
    ignores: ["dist/**", "native/**", "npm/**", "index.js", "index.d.ts"],
  },
  {
    languageOptions: {
      parserOptions: {
        // e2e/ has its own tsconfig; these root config files live outside any project.
        projectService: {
          allowDefaultProject: ["eslint.config.js", "tsup.config.ts"],
        },
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
];
