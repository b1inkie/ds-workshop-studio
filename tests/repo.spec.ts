import { test, expect, type Page } from '@playwright/test';

const REPO = 'https://github.com/b1inkie/ds-workshop-studio';

async function mockDesktop(page: Page) {
  await page.addInitScript(() => {
    const w = window as unknown as { isTauri: boolean; __TAURI_INTERNALS__: Record<string, unknown>; __TAURI_EVENT_PLUGIN_INTERNALS__: Record<string, unknown>; calls: { command: string; args: Record<string, unknown> }[] };
    w.isTauri = true; w.calls = [];
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        w.calls.push({ command, args });
        if (command === 'plugin:window|is_maximized') return false;
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'plugin:opener|open_url') return null;
        return null;
      },
    };
  });
  await page.goto('/');
}

test('repository row opens only the scoped repository URL via the opener plugin', async ({ page }) => {
  await mockDesktop(page);
  await page.getByRole('button', { name: '信息', exact: true }).click();
  const info = page.getByRole('dialog', { name: '信息', exact: true });
  const open = info.getByRole('button', { name: '打开源码仓库' });
  await expect(open).toBeEnabled();
  await open.click();
  const calls = await page.evaluate(() => (window as unknown as { calls: { command: string; args: Record<string, unknown> }[] }).calls.filter(c => c.command === 'plugin:opener|open_url'));
  expect(calls.length).toBe(1);
  expect(calls[0].args.url).toBe(REPO);
  expect(calls[0].args.openWith).toBeUndefined();
});
