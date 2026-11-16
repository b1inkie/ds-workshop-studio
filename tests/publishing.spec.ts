import { test, expect } from '@playwright/test';

test('publish confirmation, force consent, cancellation and failed-create identity', async ({ page }) => {
  // Fully mocked desktop bridge: this test cannot reach Steam or Rust.
  await page.addInitScript(() => {
    const w = window as unknown as { __TAURI_INTERNALS__: Record<string, unknown>; __TAURI_EVENT_PLUGIN_INTERNALS__: Record<string, unknown>; isTauri: boolean; calls: string[] };
    w.isTauri = true; w.calls = [];
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        w.calls.push(command);
        if (command === 'plugin:window|is_maximized') return false;
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'list_workshop') return { steamId: '123', warnings: [], items: [{ id: '12345', title: '已有模组', description: '简介', titleEnglish: '', descriptionEnglish: '', tags: ['Utility'], version: '1.0', previewUrl: '', updated: 1770000000, visibility: 2 }] };
        if (command === 'prepare_upload') {
          const request = args.request as Record<string, unknown>;
          return { token: 'local-test-token', appId: request.appId, id: request.id, title: request.title, files: 2, bytes: 1024, skipped: 1, version: '1.1', previousVersion: request.id === '0' ? '' : '1.0', updateContent: true, updatePreview: request.id === '0', updateMetadata: request.id === '0', visibility: request.visibility, warnings: [] };
        }
        if (command === 'submit_upload') return { id: '12345678901234567', needsLegalAgreement: false, error: '模拟提交失败' };
        return null;
      },
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await expect(page.getByLabel('模组名称', { exact: true })).toHaveValue('已有模组');
  await expect(page.getByRole('heading', { name: '更新已有模组', exact: true })).toBeVisible();
  await expect(page.locator('.publish-mode code')).toHaveText('12345');
  await expect(page.getByRole('region', { name: '选择更新范围' })).toBeVisible();
  await page.getByRole('button', { name: '准备更新此模组' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const submit = dialog.getByRole('button', { name: /确认发布新模组|确认更新此模组/ });
  await expect(dialog.getByRole('heading', { name: '确认更新已有模组' })).toBeVisible();
  await expect(submit).toBeDisabled();
  await dialog.getByLabel('我确认目标、可见性及更新范围无误').check();
  await expect(submit).toBeEnabled();
  await expect(dialog.getByText('本地与线上版本相同，禁止上传文件。', { exact: false })).toHaveCount(0);
  await expect(dialog.getByLabel('我明确允许覆盖相同或未知版本')).toHaveCount(0);
  expect(await page.evaluate(() => (window as unknown as { calls: string[] }).calls.includes('submit_upload'))).toBe(false);
  await dialog.getByRole('button', { name: '返回编辑' }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => (window as unknown as { calls: string[] }).calls.includes('discard_upload'))).toBe(true);
  await page.getByRole('button', { name: '发布新模组', exact: true }).click();
  await expect(page.getByRole('heading', { name: '首次发布', exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: '选择更新范围' })).toHaveCount(0);
  await expect(page.getByRole('checkbox', { name: '模组文件', exact: true })).toHaveCount(0);
  await expect(page.getByRole('combobox', { name: '可见性', exact: true })).toHaveText('私密');
  await page.getByLabel('模组名称', { exact: true }).fill('新建失败恢复');
  await page.getByRole('button', { name: '准备首次发布' }).click();
  await expect(dialog.getByRole('heading', { name: '确认首次发布' })).toBeVisible();
  await dialog.getByLabel('我确认目标、可见性及更新范围无误').check();
  await submit.click();
  await expect(page.getByRole('alertdialog')).toContainText('12345678901234567');
  await page.getByRole('alertdialog').getByRole('button', { name: '知道了' }).click();
  await expect(page.getByRole('button', { name: '准备更新此模组' })).toBeVisible();
  await expect(page.locator('.publish-mode code')).toHaveText('12345678901234567');
  expect(await page.evaluate(() => localStorage.getItem('ws-draft:322330:12345678901234567'))).toContain('新建失败恢复');
  await page.getByRole('combobox', { name: '选择游戏' }).click();
  await page.getByRole('option', { name: '饥荒 · 单机版', exact: true }).click();
  await expect(page.getByRole('button', { name: '准备首次发布' })).toBeEnabled();
  await expect(page.getByText('单机版采用旧工坊接口', { exact: false })).toBeVisible();
  await page.getByRole('button', { name: '准备首次发布' }).click();
  await expect(dialog).toContainText('饥荒 · 单机版（ZIP / RemoteStorage）');
  await dialog.getByLabel('我确认目标、可见性及更新范围无误').check();
  await submit.click();
  await expect(page.getByRole('alertdialog')).toContainText('12345678901234567');
  await page.getByRole('alertdialog').getByRole('button', { name: '知道了' }).click();
  expect(await page.evaluate(() => localStorage.getItem('ws-draft:219740:12345678901234567'))).toBeTruthy();
  await expect(page.getByRole('button', { name: '准备更新此模组' })).toBeEnabled();
  await page.getByRole('button', { name: '准备更新此模组' }).click();
  await expect(dialog).toContainText('饥荒 · 单机版（ZIP / RemoteStorage）');
  await dialog.getByRole('button', { name: '返回编辑' }).click();
});
