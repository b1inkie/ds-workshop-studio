// Launch only our executable with temporary WebView debugging; never changes release defaults.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const executable = path.resolve(process.argv[2] || 'src-tauri/target/i686-pc-windows-msvc/debug/ds-workshop-tool.exe');
const fixture = path.resolve('.local/smoke-mod');
await mkdir(fixture, { recursive: true });
await writeFile(path.join(fixture, 'modinfo.lua'), 'name = "中文测试模组"\nauthor = "Studio"\nversion = "1.2.3"\ndescription = [=[本地读取测试]=]\n', 'utf8');
const child = spawn(executable, [], {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9347', WEBVIEW2_USER_DATA_FOLDER: path.resolve(`.local/webview-smoke-${Date.now()}`) },
  stdio: 'ignore',
});
let browser;
try {
  for (let attempt = 0; attempt < 100; attempt++) {
    try { browser = await chromium.connectOverCDP('http://127.0.0.1:9347'); break; }
    catch { await new Promise(resolve => setTimeout(resolve, 250)); }
  }
  assert.ok(browser, 'WebView2 did not expose the test debugging endpoint');
  const page = browser.contexts()[0].pages()[0];
  await page.waitForSelector('h1');
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  assert.match(await page.title(), /Workshop Studio/);
  assert.equal(await page.getByText('浏览器示例数据').count(), 0);
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  assert.equal(await page.getByRole('button', { name: '切换主题' }).count(), 0);
  assert.equal(await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:window|is_decorated', { label: 'main' })), false);
  await page.getByRole('button', { name: '最大化窗口' }).click();
  await page.waitForFunction(async () => window.__TAURI_INTERNALS__.invoke('plugin:window|is_maximized', { label: 'main' }));
  await page.getByRole('button', { name: '还原窗口' }).click();
  await page.waitForFunction(async () => !(await window.__TAURI_INTERNALS__.invoke('plugin:window|is_maximized', { label: 'main' })));
  // Double-clicking the draggable title area follows the native maximize gesture.
  await page.locator('.window-drag-area').dispatchEvent('mousedown', { button: 0, buttons: 1, detail: 2 });
  await page.waitForFunction(async () => window.__TAURI_INTERNALS__.invoke('plugin:window|is_maximized', { label: 'main' }));
  await page.getByRole('button', { name: '还原窗口' }).click();
  await page.waitForFunction(async () => !(await window.__TAURI_INTERNALS__.invoke('plugin:window|is_maximized', { label: 'main' })));
  const result = await page.evaluate(async folder => {
    return window.__TAURI_INTERNALS__.invoke('read_mod_info', { folder });
  }, fixture);
  assert.equal(result.name, '中文测试模组');
  assert.equal(result.version, '1.2.3');
  // Deliberately no Steam status, list, prepare, submit, or real-account calls.
  // Publishing is covered only by Rust local fixtures and browser mock IPC tests.
  await page.getByRole('button', { name: '运行日志' }).click();
  await page.screenshot({ path: '.local/desktop-smoke.png', fullPage: true, omitBackground: true });
  assert.equal(await page.getByRole('button', { name: /准备更新此模组|准备首次发布/ }).isEnabled(), true);
  assert.deepEqual(errors, []);
  const closed = new Promise(resolve => child.once('exit', resolve));
  await page.getByRole('button', { name: '关闭窗口' }).click();
  await Promise.race([closed, new Promise((_, reject) => setTimeout(() => reject(new Error('Custom close button did not exit the app')), 5000))]);
  console.log('Desktop smoke passed: window controls and local Lua IPC only. No Steam calls.');
} finally {
  if (browser) await browser.close();
  child.kill();
}
