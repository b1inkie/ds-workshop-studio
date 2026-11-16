import { test, expect, type Page } from '@playwright/test';

const id = '12345678901234567';
async function mockDesktop(page: Page, mode: string) {
  await page.addInitScript(({ mode, id }) => {
    const w = window as unknown as { __TAURI_INTERNALS__: Record<string, unknown>; __TAURI_EVENT_PLUGIN_INTERNALS__: Record<string, unknown>; isTauri: boolean; calls: { command: string; args: Record<string, unknown> }[] };
    w.isTauri = true; w.calls = [];
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    let deleted = false;
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        w.calls.push({ command, args });
        if (command === 'plugin:window|is_maximized') return false;
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'list_workshop') {
          if (deleted && mode === 'refresh-error') throw new Error('模拟刷新断网');
          return { steamId: '123', warnings: [], items: deleted ? [] : [{ id, title: '仅模拟删除的作品', description: 'fixture', titleEnglish: '', descriptionEnglish: '', tags: [], version: '1', previewUrl: '', updated: 1770000000, visibility: 2 }] };
        }
        if (command === 'prepare_delete') {
          if (mode === 'denied') throw new Error('只能操作当前 Steam 账号拥有的物品');
          return { token: 'delete-token', id, title: '后端核实的作品名称', steamId: '123', appId: Number(args.appId) };
        }
        if (command === 'submit_delete') {
          await new Promise(resolve => setTimeout(resolve, 200));
          if (mode === 'ipc-error') throw new Error('模拟 IPC 断开');
          if (mode !== 'unknown') deleted = true;
          return { id, error: mode === 'unknown' ? '删除回调读取失败，结果未知；请刷新核对，勿直接重试。' : null };
        }
        return null;
      },
    };
  }, { mode, id });
  await page.goto('/');
}
async function submissionCount(page: Page) {
  return page.evaluate(() => (window as unknown as { calls: { command: string }[] }).calls.filter(c => c.command === 'submit_delete').length);
}

test('delete requires exact ID and explicit consent; cancellation never submits', async ({ page }) => {
  await mockDesktop(page, 'success');
  await expect(page.getByRole('button', { name: '删除此工坊模组' })).toHaveCount(0);
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await page.getByLabel('模组名称', { exact: true }).fill('未保存的草稿名');
  await page.getByRole('button', { name: '删除此工坊模组' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('后端核实的作品名称');
  await expect(dialog).toContainText(id);
  await expect(dialog).not.toContainText('未保存的草稿名');
  const submit = dialog.getByRole('button', { name: '确认删除线上模组' });
  const input = dialog.getByLabel('输入完整工坊 ID 以确认删除');
  await expect(submit).toBeDisabled();
  await input.fill(id); await expect(submit).toBeDisabled();
  await dialog.getByRole('checkbox').check(); await expect(submit).toBeEnabled();
  await input.fill(` ${id}`); await expect(submit).toBeDisabled();
  await input.fill('123'); await expect(submit).toBeDisabled();
  expect(await submissionCount(page)).toBe(0);
  await dialog.getByRole('button', { name: '取消删除' }).click();
  await expect(dialog).toHaveCount(0);
  expect(await submissionCount(page)).toBe(0);
  expect(await page.evaluate(() => (window as unknown as { calls: { command: string }[] }).calls.some(c => c.command === 'discard_delete'))).toBe(true);
  await page.getByRole('button', { name: '发布新模组', exact: true }).click();
  await expect(page.getByRole('button', { name: '删除此工坊模组' })).toHaveCount(0);
  await page.getByRole('combobox', { name: '选择游戏' }).click();
  await page.getByRole('option', { name: '饥荒 · 单机版', exact: true }).click();
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await expect(page.getByRole('button', { name: '删除此工坊模组' })).toBeEnabled();
  await page.getByRole('button', { name: '删除此工坊模组' }).click();
  await expect(page.getByRole('dialog')).toContainText('饥荒 · 单机版');
  await page.getByRole('button', { name: '取消删除' }).click();
});

for (const mode of ['success', 'unknown', 'ipc-error', 'refresh-error']) {
  test(`mock deletion result: ${mode}`, async ({ page }) => {
    await mockDesktop(page, mode);
    await page.getByRole('button', { name: '刷新工坊列表' }).click();
    await page.getByRole('button', { name: '删除此工坊模组' }).click();
    const dialog = page.getByRole('dialog');
    await dialog.getByLabel('输入完整工坊 ID 以确认删除').fill(id);
    await dialog.getByRole('checkbox').check();
    await dialog.getByRole('button', { name: '确认删除线上模组' }).click();
    await expect(page.getByRole('button', { name: '取消删除' })).toBeDisabled();
    await expect(dialog).toHaveCount(0);
    expect(await submissionCount(page)).toBe(1);
    const submitted = await page.evaluate(() => (window as unknown as { calls: { command: string; args: Record<string, unknown> }[] }).calls.find(c => c.command === 'submit_delete')?.args);
    expect(submitted).toEqual({ token: 'delete-token', confirmedId: id });
    if (mode === 'success' || mode === 'refresh-error') {
      await expect(page.getByRole('status')).toContainText('已删除线上模组');
      await expect(page.getByRole('button', { name: /仅模拟删除的作品/ })).toHaveCount(0);
      await expect(page.getByRole('heading', { name: '首次发布', exact: true })).toBeVisible();
      expect(await page.evaluate(() => (window as unknown as { calls: { command: string }[] }).calls.filter(c => c.command === 'list_workshop').length)).toBe(2);
      if (mode === 'refresh-error') await expect(page.getByRole('alertdialog')).toContainText('删除已成功，但自动刷新失败');
    } else {
      await expect(page.getByRole('alert')).toContainText('结果未知');
      await expect(page.getByRole('alert')).toContainText(id);
      await expect(page.getByRole('button', { name: /仅模拟删除的作品/ })).toHaveCount(1);
    }
    expect(await page.evaluate(id => localStorage.getItem(`ws-draft:322330:${id}`), id)).toContain('仅模拟删除的作品');
  });
}

test('ownership failure blocks deletion dialog and submit', async ({ page }) => {
  await mockDesktop(page, 'denied');
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await page.getByRole('button', { name: '删除此工坊模组' }).click();
  await expect(page.getByRole('alert')).toContainText('当前 Steam 账号');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await submissionCount(page)).toBe(0);
});

test('single-player deletion uses selected game and automatically refreshes', async ({ page }) => {
  await mockDesktop(page, 'success');
  await page.getByRole('combobox', { name: '选择游戏' }).click();
  await page.getByRole('option', { name: '饥荒 · 单机版', exact: true }).click();
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await page.getByRole('button', { name: '删除此工坊模组' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('饥荒 · 单机版');
  await dialog.getByLabel('输入完整工坊 ID 以确认删除').fill(id);
  await dialog.getByRole('checkbox').check();
  await dialog.getByRole('button', { name: '确认删除线上模组' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('status')).toContainText('已删除线上模组');
  const calls = await page.evaluate(() => (window as unknown as { calls: { command: string; args: Record<string, unknown> }[] }).calls);
  expect(calls.find(c => c.command === 'prepare_delete')?.args.appId).toBe(219740);
  expect(calls.filter(c => c.command === 'list_workshop').map(c => c.args.appId)).toEqual([219740, 219740]);
});
