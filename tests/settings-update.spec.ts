import { test, expect, type Page } from '@playwright/test';

const manifest = { version: '0.2.0', url: 'https://cdn.example.org/Workshop-Studio-0.2.0-Windows-test.zip', sha256: 'a'.repeat(64), notes: '修复若干问题，改进删除流程。', pubDate: '2026-11-16' };

async function mockDesktop(page: Page, mode: string) {
  await page.addInitScript(({ mode, manifest }) => {
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
        if (command === 'app_version') return '0.1.0';
        if (command === 'check_update') {
          if (mode === 'unconfigured') throw new Error('更新源尚未配置：请等待开发者提供正式更新地址。');
          if (mode === 'network') throw new Error('无法访问更新源：连接超时');
          return { currentVersion: '0.1.0', isNewer: mode !== 'latest', manifest, source: 'https://updates.example.org/latest.json' };
        }
        if (command === 'apply_update') {
          await new Promise(resolve => setTimeout(resolve, mode === 'apply-fail' ? 120 : 350));
          if (mode === 'apply-fail') throw new Error('更新包 SHA256 校验失败，已放弃安装。');
          return null;
        }
        return null;
      },
    };
  }, { mode, manifest });
  await page.goto('/');
}

function calls(page: Page, command: string) {
  return page.evaluate(command => (window as unknown as { calls: { command: string; args: Record<string, unknown> }[] }).calls.filter(c => c.command === command), command);
}

test('settings updater: new version, busy protection, restart request', async ({ page }) => {
  await mockDesktop(page, 'newer');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置', exact: true });
  await expect(dialog).toContainText('当前版本 v0.1.0');
  await expect(dialog.getByText('浏览器预览不支持更新')).toHaveCount(0);
  await dialog.getByRole('button', { name: '检查更新' }).click();
  await expect(dialog).toContainText('发现新版本 v0.2.0');
  await expect(dialog).toContainText('修复若干问题');
  await expect(dialog).toContainText('updates.example.org');
  expect((await calls(page, 'apply_update')).length).toBe(0);
  await dialog.getByRole('button', { name: '下载并安装' }).click();
  await expect(dialog.getByRole('button', { name: '关闭设置' })).toBeDisabled();
  await expect(dialog.getByRole('button', { name: '下载并安装' })).toHaveCount(0);
  await expect(dialog).toContainText('更新已就绪，重启后生效');
  await expect(dialog.getByRole('button', { name: '关闭设置' })).toBeEnabled();
  await dialog.getByRole('button', { name: '重启并完成更新' }).click();
  const applied = await calls(page, 'apply_update');
  expect(applied.length).toBe(1);
  expect(applied[0].args).toEqual({ manifest });
  expect((await calls(page, 'restart_app')).length).toBe(1);
});

test('settings updater: already latest and error states', async ({ page }) => {
  await mockDesktop(page, 'latest');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置', exact: true });
  await dialog.getByRole('button', { name: '检查更新' }).click();
  await expect(dialog).toContainText('已是最新版本');
  await expect(dialog.getByRole('button', { name: '下载并安装' })).toHaveCount(0);
});

test('settings updater: unconfigured source keeps retry available', async ({ page }) => {
  await mockDesktop(page, 'unconfigured');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置', exact: true });
  await dialog.getByRole('button', { name: '检查更新' }).click();
  await expect(dialog.getByRole('alert')).toContainText('更新源尚未配置');
  await expect(dialog.getByRole('button', { name: '检查更新' })).toBeEnabled();
});

test('settings updater: failed install keeps the old version usable', async ({ page }) => {
  await mockDesktop(page, 'apply-fail');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置', exact: true });
  await dialog.getByRole('button', { name: '检查更新' }).click();
  await dialog.getByRole('button', { name: '下载并安装' }).click();
  await expect(dialog.getByRole('alert')).toContainText('SHA256 校验失败');
  await expect(dialog).toContainText('旧版本仍可继续使用');
  await expect(dialog.getByRole('button', { name: '下载并安装' })).toBeEnabled();
  await expect(dialog.getByRole('button', { name: '重启并完成更新' })).toHaveCount(0);
});
