# Workshop Studio · 饥荒创意工坊工具

基于 **Tauri 2 + React 19 + TypeScript + Rust** 的饥荒创意工坊桌面工具，独立项目，支持联机版（DST）与单机版（DS）模组的发布、更新与删除。

## 当前范围（0.1.0，联机版 / 单机版功能测试版）

- 三栏工作区：我的工坊 / 线上详情 / 本地编辑草稿。
- 仅深色主题（参考 Agents Anywhere 的黑灰层次、白色按钮与高对比文字），正文/输入框 15px，辅助文字至少 13px；列表搜索、运行日志与独立滚动区域。
- 去掉 Windows 原生标题栏，使用自定义深色窗口栏，支持拖动、双击最大化、最小化/还原/关闭。
- Rust 动态加载 Steamworks x86 `steam_api.dll`，初始化 Steam、读取当前账号已发布物品。
- 工具 AppID 为 `245850`，消费游戏支持 DST `322330` 与 DS `219740`；复用旧接口版本、双语言查询、分页及 Items → All 回退。
- 64 位 Steam ID / 工坊 ID 用字符串传到 JavaScript，避免精度损失。
- 本地选择模组目录，静态读取 UTF-8 `modinfo.lua` 的独立字符串赋值；支持引号与长字符串、跳过注释和函数/条件块，不执行 Lua。
- 草稿按游戏和物品分别保存在当前 Windows 用户的 WebView 本地存储。
- 浏览器模式明确显示**示例数据**；桌面模式无示例数据，不会假装已连接 Steam。

**已接入，真实上传由用户自测：** DST 目录/manifest 新建更新；DS 单机版 ZIP / Steam Cloud / RemoteStorage 新建更新；封面、本地暂存过滤、同版本硬性禁止上传文件、确认与结果。发布前只准备本地快照并读取线上信息，明确确认后才写入。

**已接入，真实删除由用户自测：** DST DeleteItem 与 DS DeletePublishedFile；所有权预检、完整 ID、风险勾选、5 分钟一次性确认、提交前复查。删除成功后自动刷新列表，刷新失败单独提示，不误报删除失败。回调异常/超时仍提示结果未知，不自动重试；本地文件和草稿保留。

**尚未迁移：** 完整多语言 Lua 规则、INI 导入导出、命令行上传。单机版使用阶段提示，无字节进度、不提交更新说明；ZIP 不超过 100MiB，Cloud 暂存文件保留且暂无自动清理（占用 Steam Cloud 配额）。

### 手动上传自测

1. 建议先备份本地模组，以私密可见性新建专用测试物品；不要直接覆盖正式作品。
2. 新建：选择对应游戏、新建模组草稿、读取目录、选择 ≤1MiB PNG/JPG/GIF 封面，填写标题并选私密，再点“准备首次发布”。
3. 更新：刷新并选择自己拥有的对应游戏物品，选择目录，勾选更新范围；仅上传文件时保留线上其他 Tags。文本/封面更新可不选内容目录。
4. 准备阶段会复制内容到应用缓存，不改源目录；确认窗口展示文件数、版本、可见性和范围。不确认可返回编辑；确认有效期 10 分钟。
5. 勾选目标确认后，点击确认按钮才会调用 Steam 写入。更新模组文件时，本地与线上版本相同会被后端强制阻止（忽略 v 前缀和大小写），无覆盖选项；线上版本未知仍需独立确认。仅修改封面/简介不受文件版本限制。
   可在工具的“修改本地版本号”中修改并点击“保存版本到文件”，明确写入源目录 modinfo.lua 后重新准备。只支持唯一顶层独立 version 字符串，自动备份至 `.workshop-studio-backups/`（上传排除）；保留其他字节、BOM 与换行。动态/重复定义或文件变化拒绝写入。
6. 上传中不要杀进程；失败先保存显示的工坊 ID 并刷新核对，不要直接重复新建。上传历史记录在 `%APPDATA%/tools.dst.workshop-studio/upload-history/`，暂存由 RAII 清理（异常退出时可能遗留缓存）。
7. 提示需接受法律协议时，请在 Steam 工坊页面处理。成功返回后仍需自行刷新核对线上版本及内容。

### 手动删除

选择自己拥有的对应游戏作品，在右侧底部点击“删除此工坊模组”。后端读取线上名称、ID 和所有者，再显示确认；输入完整 ID 并勾选风险确认，最后点“确认删除线上模组”才调用删除。只用专用测试物品自测。成功后自动刷新列表，保留本地文件与草稿；失败/未知时请先核对，切勿直接重试。
**测试边界：真实账号和真实上传由用户操作。助手仅运行本地 Rust 夹具、模拟 IPC 和离线窗口检查，不连接 Steam 做验证。**

## 开发

需要 Windows、Node.js 22+、Rust MSVC 工具链、Visual C++ Build Tools（x86 C++ 工具与 Windows SDK）、WebView2 Runtime。Steam 查询还需要已登录 Steam 及工具权限。

```powershell
npm ci
rustup target add i686-pc-windows-msvc
npm run desktop:dev
```

仅预览 React 界面：

```powershell
npm run dev
# http://127.0.0.1:1437
```

选用 1437 而非 Tauri 默认的 1420，避免与本机其他 Tauri 项目开发服务器冲突。严格锁定端口，冲突时应停止本项目自己的服务器或同步修改两个配置，不能连接其他项目页面。

## 构建

```powershell
npm run desktop:build
```

输出：`src-tauri/target/i686-pc-windows-msvc/release/ds-workshop-tool.exe`。

运行时需要同目录的 `resources/steam_api.dll`，不能只复制 exe。生成便携目录：

> **从源码构建注意**：仓库不含 `steam_api.dll`（Valve 专有组件，不随源码再分发）。请自行将 **x86** 版 `steam_api.dll`（可从 [Steamworks SDK](https://partner.steamgames.com/) 的 `redistributable_bin/`，或饥荒单机版安装目录）复制到 `src-tauri/resources/steam_api.dll`。

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1
# .local/portable/Workshop Studio/
```

生成供小范围测试的干净发布 ZIP（只打包现有 Release，不启动程序）：

```powershell
npm run package:release
# .local/releases/Workshop-Studio-0.1.0-Windows-test.zip
# 以及 .zip.sha256.txt 校验清单
```

发布包白名单仅含 `Workshop Studio.exe`、`resources/steam_api.dll`、`使用说明.txt`，不会复制便携目录中的诊断副本或本机日志、草稿。脚本校验 ZIP 内每个文件的 SHA256；改动代码后应先重新 Release 构建。使用说明源文件为 `docs/使用说明.txt`。

### 在工具内更新（设置弹窗）

更新器位于 `src-tauri/src/updater.rs`，流程：HTTPS 清单 → 下载 ZIP → SHA256 校验 → 暂存解包（防路径穿越/链接/超大）→ 替换文件（旧 exe 重命名为 `.old.exe`，下次启动清理）→ 重启生效。下载在 Rust 侧完成，前端只收到进度事件。更新期间持有全局锁并阻止 Steam 操作与窗口关闭。

发布新版本前：
1. 同步提升 `src-tauri/tauri.conf.json` 与 `package.json` 的 `version`（package:release 会校验一致）。
2. `npm run desktop:build && npm run package:release`，取 `.zip.sha256.txt` 首行的 ZIP 哈希。
3. 运行 `npm run publish:release`（见下）或手动把 `latest.json` 上传到 `releases/latest/download/latest.json` 对应的 GitHub Release。

清单格式（camelCase）：

```json
{
  "version": "0.2.0",
  "url": "https://…/Workshop-Studio-0.2.0-Windows-test.zip",
  "sha256": "…zip 的 SHA256…",
  "notes": "本次更新说明",
  "pubDate": "2026-11-16"
}
```

限制：清单与 ZIP 均要求 HTTPS；无签名密钥，完整性依赖 SHA256 与 HTTPS 传输；未做断点续传与自动检查，仅手动触发。

这是依赖系统 WebView2 的便携构建，尚未制作安装包/运行时引导器。`steam_api.dll`（© Valve）仅随 Release 中的应用包分发，不在源码仓库中；再分发前请自行确认符合 Valve 的 Steamworks SDK 条款。程序通过绝对路径加载 DLL，启动时设置自身 `SteamAppId`/`SteamGameId`，不改工作目录或其他软件的 `steam_appid.txt`。

## 验证

```powershell
npm run build
npm run test:rust
cargo clippy --manifest-path src-tauri/Cargo.toml --target i686-pc-windows-msvc -- -D warnings
npx playwright install chromium --only-shell
npm test
```

桌面 IPC / WebView2 冒烟测试采用**单独的测试配置**开启调试端口，正常发布配置不包含调试端口：

```powershell
npx tauri build --debug --target i686-pc-windows-msvc --no-bundle --config tests/tauri.smoke.conf.json
npm run test:desktop
# 此脚本仅检查窗口与本地 Lua IPC，不连接 Steam。
```

`tests/publishing.spec.ts` 和 `tests/deletion.spec.ts` 使用完全模拟的桌面 IPC 验证上传与删除确认、取消、成功、权限拒绝、结果未知及失败后的 ID 保留；不会连接真实 Steam。真实 CreateItem / SubmitItemUpdate / DeleteItem 均交由用户自测。Rust 单元测试覆盖删除回调 ABI、所有权/游戏/目标校验和确认过期。测试生成图片与夹具位于 `.local/`，不提交。

## 代码结构

```text
src/
  App.tsx             React 工作区与交互
  styles.css          主题变量与三栏布局
  bridge.ts           类型化 Tauri 调用
  types.ts            DTO / 草稿类型
src-tauri/
  src/main.rs         命令边界、Steam 互斥、后台执行
  src/steam.rs        旧 Steam DLL FFI、RAII、列表
  src/steam/publishing.rs  所有权预检、创建/提交及进度
  src/steam/deletion.rs    联机/单机删除预检、一次性确认与回调
  src/steam/remote.rs      单机 Steam Cloud 文件共享及旧工坊发布/更新
  src/upload.rs       本地不可变快照、校验、manifest、一次性确认
  src/modinfo.rs      Lua 静态读取、带备份和冲突校验的版本编辑
  capabilities/       仅本地窗口 + 文件夹选择权限
  resources/          原版 x86 Steam DLL
scripts/              桌面冒烟测试、便携目录生成
 tests/               浏览器回归与桌面测试配置
```

### 安全与已知限制

- 前端没有 shell/任意 HTTP/文件系统插件权限，远程页面没有 Tauri capability；上传和删除采用后端保存的一次性 token，预检后再次校验账号与目标。工坊写入中阻止正常关闭，但无法防止任务管理器终止。
- 描述文本按 React 字符串渲染，不执行 HTML；远程封面仅允许 HTTPS。
- Steam 调用串行执行并在后台线程等待，查询句柄和 API 生命周期用 RAII 释放；单次回调 30 秒超时，每种查询最多 20 页（1000 项），部分失败/截断会告警。
- `modinfo.lua` 不是完整 Lua 解释器；动态表达式、复杂转义、语言分支、同一行多个语句不保证读取。读取不到时保留手动编辑能力并输出警告。
- 草稿不是备份系统，清除 WebView 数据会丢失。后续会迁移为 Rust 侧可导出配置。
- Windows x86 目标来自当前 DLL 位数，而不是游戏位数。更换 64 位 SDK 后应独立重新验证 ABI 和接口，不能只改编译目标。

## 许可

本项目代码以 [MIT](LICENSE) 授权。`steam_api.dll` © Valve Corporation，不适用本许可证，仅随发布包中的应用分发。依赖项（Tauri、React、ureq/rustls、zip 等）各自遵循其 MIT / Apache-2.0 等开源许可证。
