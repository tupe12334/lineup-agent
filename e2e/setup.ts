import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, afterEach, beforeAll } from "vitest";

let globalTempDir: string;
const testTempDirs: string[] = [];

beforeAll(async () => {
  globalTempDir = await mkdtemp(join(tmpdir(), "lineup-agent-e2e-"));
});

afterAll(async () => {
  if (globalTempDir) {
    await rm(globalTempDir, { recursive: true, force: true });
  }
});

afterEach(async () => {
  for (const dir of testTempDirs) {
    await rm(dir, { recursive: true, force: true }).catch(() => {});
  }
  testTempDirs.length = 0;
});

export async function createTestDir(prefix = "test"): Promise<string> {
  const dir = await mkdtemp(join(globalTempDir, `${prefix}-`));
  testTempDirs.push(dir);
  return dir;
}
