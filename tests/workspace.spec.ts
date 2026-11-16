import { expect, test } from '@playwright/test';

test('preview workflow, independent drafts, recovery and dark theme', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await expect(page.getByText('浏览器示例数据')).toBeVisible();
  await expect(page.getByText('草稿仅保存在本机', { exact: false })).toHaveCount(0);
  await expect(page.getByText('Tauri 桌面环境', { exact: true })).toHaveCount(0);
  await expect(page.locator('.statusbar').getByRole('button', { name: '运行日志' })).toBeVisible();
  await expect(page.getByRole('heading', { name: '四季的信笺 · Seasons', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '准备更新此模组' })).toBeDisabled();
  const title = page.getByLabel('模组名称', { exact: true });
  await title.fill('草稿测试 A');
  await page.getByRole('button', { name: '保存草稿' }).click();
  await page.getByRole('button', { name: /旅人的小背包/ }).click();
  await expect(title).toHaveValue('旅人的小背包');
  await title.fill('草稿测试 B');
  await page.getByRole('button', { name: /四季的信笺 · Seasons/ }).click();
  await expect(title).toHaveValue('草稿测试 A');
  // Explicitly empty values must survive navigation as well.
  await title.fill('');
  await page.getByRole('button', { name: /旅人的小背包/ }).click();
  await expect(title).toHaveValue('草稿测试 B');
  await page.getByRole('button', { name: /四季的信笺 · Seasons/ }).click();
  await expect(title).toHaveValue('');
  await page.getByRole('button', { name: '保存草稿' }).click();
  await page.reload();
  await expect(title).toHaveValue('');
  await expect(page.getByRole('button', { name: '切换主题' })).toHaveCount(0);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('button', { name: '连接 Steam' }).click();
  await expect(page.getByRole('status')).toContainText('浏览器界面预览');
  await page.getByLabel('搜索模组').fill('不存在的作品');
  await expect(page.getByText('没有匹配的作品')).toBeVisible();
  await expect(page.locator('body')).toHaveJSProperty('scrollWidth', 1440);
  expect(errors).toEqual([]);
});

test('dark-only theme, readable typography and minimum window layout', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('ws-theme', 'light'));
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.getByRole('button', { name: '切换主题' })).toHaveCount(0);
  await page.screenshot({ path: '.local/preview-dark.png', fullPage: true });
  for (const size of [{ width: 1440, height: 900 }, { width: 1080, height: 720 }]) {
    await page.setViewportSize(size);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(size.width);
    expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBeLessThanOrEqual(size.height);
    expect(await page.locator('.topbar').evaluate(element => element.getBoundingClientRect().height)).toBeLessThanOrEqual(68);
    expect(await page.locator('.panels').evaluate(element => element.getBoundingClientRect().top)).toBeLessThanOrEqual(110);
    const headingHeights = await page.locator('.panels > .panel > .section-heading').evaluateAll(elements => elements.map(element => element.getBoundingClientRect().height));
    expect(headingHeights).toEqual([46, 46, 46]);
    const footerRects = await page.locator('.panels > .library > .library-footer, .panels > .editor > .editor-footer').evaluateAll(elements => elements.map(element => ({ top: element.getBoundingClientRect().top, height: element.getBoundingClientRect().height })));
    expect(footerRects).toHaveLength(2);
    expect(footerRects[0].height).toBe(72);
    expect(footerRects[1].height).toBe(72);
    expect(Math.abs(footerRects[0].top - footerRects[1].top)).toBeLessThanOrEqual(1);
    await expect(page.locator('.editor .field-heading svg')).toHaveCount(0);
    await expect(page.locator('.editor .field-heading > span')).toHaveCount(0);
    expect(await page.locator('.editor .field-heading h3').evaluateAll(elements => elements.map(element => getComputedStyle(element).fontSize))).toEqual(['15px', '15px', '15px']);
    await expect(page.getByRole('button', { name: '发布新模组', exact: true })).toBeInViewport();
    await expect(page.getByText('让好点子，走进永恒领域', { exact: false })).toHaveCount(0);
    expect(await page.getByLabel('模组名称', { exact: true }).evaluate(e => parseFloat(getComputedStyle(e).fontSize))).toBeGreaterThanOrEqual(15);
    expect(await page.locator('.mod-card strong').first().evaluate(e => parseFloat(getComputedStyle(e).fontSize))).toBeGreaterThanOrEqual(15);
    const fieldSpacing = await page.locator('.local-status').evaluate(element => {
      const next = element.nextElementSibling;
      if (!next) throw new Error('Missing preview field');
      return next.getBoundingClientRect().top - element.getBoundingClientRect().bottom;
    });
    expect(fieldSpacing).toBeGreaterThanOrEqual(24);
    await expect(page.getByRole('button', { name: '保存草稿' })).toBeInViewport();
  }
  await page.screenshot({ path: '.local/preview-minimum.png', fullPage: true });
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByText('外观', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: '最小化窗口' })).toBeVisible();
  await expect(page.getByRole('button', { name: '最大化窗口' })).toBeVisible();
  await expect(page.getByRole('button', { name: '关闭窗口' })).toBeVisible();
});

test('all standard tags, custom input and per-item persistence', async ({ page }) => {
  await page.goto('/');
  const group = page.getByRole('group', { name: '标准工坊标签' });
  await expect(group.getByRole('checkbox')).toHaveCount(15);
  for (const tag of ['art','character','creature','environment','interface','item','language','other','pet','scenario','tutorial','tweak','utility','worldgen','server_admin']) {
    await expect(group.getByRole('checkbox', { name: new RegExp(`^${tag} `) })).toHaveCount(1);
  }
  await expect(group.getByRole('checkbox', { name: /^environment / })).toBeChecked();
  const art = group.getByRole('checkbox', { name: /^art / });
  const admin = group.getByRole('checkbox', { name: /^server_admin / });
  await art.check(); await admin.check();
  const custom = page.getByPlaceholder('填写其他标签，保留已有自定义标签');
  await custom.fill('MyTag');
  await custom.press('End');
  await custom.pressSequentially(', SecondTag');
  await expect(custom).toHaveValue('MyTag, SecondTag');
  await page.getByRole('button', { name: '保存草稿' }).click();
  await page.getByRole('button', { name: /旅人的小背包/ }).click();
  await expect(art).not.toBeChecked();
  await expect(group.getByRole('checkbox', { name: /^utility / })).toBeChecked();
  await page.getByRole('button', { name: /四季的信笺 · Seasons/ }).click();
  await expect(art).toBeChecked(); await expect(admin).toBeChecked();
  await expect(custom).toHaveValue('MyTag, SecondTag');
  await art.uncheck();
  await page.getByRole('button', { name: '保存草稿' }).click();
  await page.reload();
  await expect(art).not.toBeChecked(); await expect(admin).toBeChecked();
  await expect(custom).toHaveValue('MyTag, SecondTag');
});

test('switching publication mode preserves independent drafts and update scope', async ({ page }) => {
  await page.goto('/');
  await page.getByLabel('模组文件', { exact: true }).uncheck();
  await page.getByLabel('封面图片', { exact: true }).check();
  await page.getByRole('button', { name: '发布新模组', exact: true }).click();
  await expect(page.locator('.publish-mode')).toHaveAttribute('data-mode', 'create');
  await expect(page.getByRole('region', { name: '选择更新范围' })).toHaveCount(0);
  await page.getByLabel('模组名称', { exact: true }).fill('首次发布草稿');
  await page.getByRole('button', { name: /四季的信笺 · Seasons/ }).click();
  await expect(page.locator('.publish-mode')).toHaveAttribute('data-mode', 'update');
  await expect(page.getByLabel('模组文件', { exact: true })).not.toBeChecked();
  await expect(page.getByLabel('封面图片', { exact: true })).toBeChecked();
  await page.getByRole('button', { name: '发布新模组', exact: true }).click();
  await expect(page.getByLabel('模组名称', { exact: true })).toHaveValue('首次发布草稿');
});
