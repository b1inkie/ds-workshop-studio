import { test, expect } from '@playwright/test';

test('edit local version explicitly, save with file hash, then prepare updated content', async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as { isTauri: boolean; __TAURI_INTERNALS__: Record<string, unknown>; __TAURI_EVENT_PLUGIN_INTERNALS__: Record<string, unknown>; calls: { command: string; args: Record<string, unknown> }[] };
    w.isTauri = true; w.calls = [];
    let version = '2.0.0';
    const info = () => ({ folder: 'C:/fixture', name: '模拟模组', description: '简介', author: 'fixture', version, versionCompatible: '', warnings: [], fileHash: `hash-${version}` });
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        w.calls.push({ command, args });
        if (command === 'plugin:window|is_maximized') return false;
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'list_workshop') return { steamId: '123', warnings: [], items: [{ id: '12345', title: '模拟模组', description: '简介', titleEnglish: '', descriptionEnglish: '', tags: [], version: '2.0.0', previewUrl: '', updated: 1770000000, visibility: 2 }] };
        if (command === 'read_mod_info') return info();
        if (command === 'save_mod_version') { version = String(args.version); return { info: info(), backup: 'C:/fixture/.workshop-studio-backups/mock.lua.bak' }; }
        if (command === 'prepare_upload') {
          if (version === '2.0.0') throw new Error('本地与线上版本相同（2.0.0），禁止上传模组文件。请修改并保存本地版本。');
          return { token: 'fixture-token', id: '12345', title: '模拟模组', files: 1, bytes: 100, skipped: 0, version, previousVersion: '2.0.0', updateContent: true, updatePreview: false, updateMetadata: false, visibility: 2, warnings: [] };
        }
        return null;
      },
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: '刷新工坊列表' }).click();
  await page.getByPlaceholder('选择包含 modinfo.lua 的文件夹').fill('C:/fixture');
  await page.getByRole('button', { name: '重新读取', exact: true }).click();
  const edit = page.getByRole('textbox', { name: '修改本地版本号', exact: true });
  await expect(edit).toHaveValue('2.0.0');
  await page.getByRole('button', { name: '准备更新此模组' }).click();
  await expect(page.getByRole('alertdialog')).toContainText('禁止上传');
  await expect(page.getByRole('alertdialog')).toBeVisible();
  const shellStyles = (element: Element) => {
    const header = element.querySelector('.dialog-header')!;
    const body = element.querySelector('.dialog-body')!;
    const footer = element.querySelector('.dialog-footer')!;
    const shell = getComputedStyle(element);
    return { width: element.getBoundingClientRect().width, background: shell.backgroundColor, radius: shell.borderRadius, border: shell.border, headerHeight: header.getBoundingClientRect().height, bodyPadding: getComputedStyle(body).padding, bodyFont: getComputedStyle(body).fontSize, bodyLineHeight: getComputedStyle(body).lineHeight, footerPadding: getComputedStyle(footer).padding };
  };
  const warningStyles = await page.getByRole('alertdialog').evaluate(shellStyles);
  await expect(page.getByRole('alertdialog')).toBeFocused();
  await expect(page.getByRole('tooltip')).toHaveCount(0);
  await page.screenshot({ path: '.local/warning-dialog.png' });
  await page.getByRole('alertdialog').getByRole('button', { name: '知道了' }).click();
  await expect(page.getByRole('alertdialog')).toHaveCount(0);
  await page.getByRole('button', { name: '联系与交流', exact: true }).click();
  expect(await page.getByRole('dialog').evaluate(shellStyles)).toEqual(warningStyles);
  await page.getByRole('dialog').getByRole('button', { name: '知道了' }).click();
  await edit.fill('2.0.1');
  await page.getByRole('button', { name: '准备更新此模组' }).click();
  await expect(page.getByRole('alertdialog')).toContainText('版本号尚未保存');
  await page.getByRole('alertdialog').getByRole('button', { name: '知道了' }).click();
  await page.getByRole('button', { name: '保存版本到文件' }).click();
  await expect(page.getByRole('status')).toContainText('本地版本已保存为 2.0.1');
  expect(await page.evaluate(() => (window as unknown as { calls: { command: string; args: Record<string, unknown> }[] }).calls.find(c => c.command === 'save_mod_version')?.args)).toEqual({ folder: 'C:/fixture', version: '2.0.1', expectedHash: 'hash-2.0.0' });
  await page.getByRole('button', { name: '准备更新此模组' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('2.0.0 → 2.0.1');
  await expect(dialog.getByRole('checkbox')).toHaveCount(1);
  await dialog.getByLabel('我确认目标、可见性及更新范围无误').check();
  await expect(dialog.getByRole('button', { name: '确认更新此模组' })).toBeEnabled();
  expect(await page.evaluate(() => (window as unknown as { calls: { command: string }[] }).calls.some(c => c.command === 'submit_upload'))).toBe(false);
  await dialog.getByRole('button', { name: '返回编辑' }).click();
});
