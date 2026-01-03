import { describe, expect, it } from "vitest";
import { rules } from "../helpers/cli";

describe("rules command", () => {
  describe("text output", () => {
    it("should list all available rules", async () => {
      const result = await rules();

      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain("Available Rules");
      expect(result.stdout).toContain("claude-settings-hooks");
      expect(result.stdout).toContain("husky-init");
    });

    it("should show rule details", async () => {
      const result = await rules();

      expect(result.stdout).toContain("Severity");
      expect(result.stdout).toContain("Can fix");
    });
  });

  describe("JSON output", () => {
    it("should output valid JSON array", async () => {
      const result = await rules({ json: true });

      expect(result.exitCode).toBe(0);
      expect(() => JSON.parse(result.stdout)).not.toThrow();

      const rulesList = JSON.parse(result.stdout);
      expect(Array.isArray(rulesList)).toBe(true);
    });

    it("should have correct rule structure", async () => {
      const result = await rules({ json: true });
      const rulesList = JSON.parse(result.stdout);

      expect(rulesList.length).toBeGreaterThanOrEqual(2);

      for (const rule of rulesList) {
        expect(rule).toHaveProperty("id");
        expect(rule).toHaveProperty("name");
        expect(rule).toHaveProperty("description");
        expect(rule).toHaveProperty("defaultSeverity");
        expect(rule).toHaveProperty("canFix");
        expect(typeof rule.canFix).toBe("boolean");
      }
    });

    it("should include claude-settings-hooks rule", async () => {
      const result = await rules({ json: true });
      const rulesList = JSON.parse(result.stdout);

      const claudeRule = rulesList.find(
        (r: { id: string }) => r.id === "claude-settings-hooks"
      );
      expect(claudeRule).toBeDefined();
      expect(claudeRule.canFix).toBe(true);
      expect(claudeRule.defaultSeverity).toBe("error");
    });

    it("should include husky-init rule", async () => {
      const result = await rules({ json: true });
      const rulesList = JSON.parse(result.stdout);

      const huskyRule = rulesList.find(
        (r: { id: string }) => r.id === "husky-init"
      );
      expect(huskyRule).toBeDefined();
      expect(huskyRule.canFix).toBe(true);
    });
  });
});
