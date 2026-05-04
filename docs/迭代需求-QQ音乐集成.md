# Claudio Desktop — QQ 音乐集成迭代需求

## 1. 背景与目标

### 1.1 现状
Claudio Desktop 当前是一个本地文件播放器，通过 Tauri v2 + rodio 实现音频播放。用户通过文件对话框选择本地音乐文件进行播放，支持播放/暂停/切歌/音量调节/进度拖拽等基础功能。

### 1.2 目标
将 Claudio Desktop 升级为支持 QQ 音乐在线听歌、搜索、收藏的音乐播放器，同时保留本地文件播放能力。

### 1.3 核心原则
- **不替换现有功能**：本地文件播放保持不变
- **增量集成**：QQ 音乐作为新增数据源，与本地文件共存
- **用户体验一致**：在线歌曲的播放、切歌、进度控制与本地文件操作无差别

---

## 2. 功能需求

### 2.1 搜索功能
| 项目 | 说明 |
|------|------|
| 入口 | Playlist 面板增加 Tab 切换：「本地文件」/「QQ 音乐」 |
| 搜索框 | 输入关键词，支持防抖（300ms），避免频繁请求 |
| 搜索结果 | 展示歌曲名、歌手、专辑、封面缩略图 |
| 分页 | 支持加载更多 / 翻页 |
| 操作 | 点击搜索结果 → 播放 / 添加到播放列表 |

### 2.2 在线播放
| 项目 | 说明 |
|------|------|
| 播放方式 | Rust 后端获取 CDN 播放 URL → reqwest 下载音频流 bytes → rodio 解码播放 |
| 音质 | 匿名用户 128kbps（MP3），VIP 用户可获取更高码率 |
| 进度控制 | 与本地文件共用同一套进度条、切歌逻辑 |
| 混合播放 | 本地文件与 QQ 音乐歌曲可同在一个播放列表，按序播放 |

### 2.3 登录与鉴权
| 项目 | 说明 |
|------|------|
| 登录方式 | QQ 扫码登录（QR Code） |
| 流程 | 前端展示二维码 → 用户手机 QQ 扫码 → 轮询登录状态 → 持久化 Cookie |
| 匿名模式 | 不登录也可搜索和试听 128kbps |
| Cookie 持久化 | 保存到本地文件，重启应用无须重新登录 |

### 2.4 收藏功能
| 项目 | 说明 |
|------|------|
| 收藏列表 | 独立页面或标签，展示已收藏的歌曲 |
| 添加收藏 | 搜索结果中点击收藏按钮 |
| 取消收藏 | 收藏列表中移除 |
| 数据持久化 | 收藏数据存储到本地文件 |
| 依赖 | 需登录后才可使用 |

---

## 3. 技术架构

### 3.1 架构总览

```
┌─────────────────────────────────────────────────────┐
│ React 前端                                           │
│  - SearchPanel (搜索)                                 │
│  - FavoritesPage (收藏)                               │
│  - Playlist (双视图: 本地/在线)                        │
└──────────────┬─── Tauri invoke ──────────────────────┘
               │
┌──────────────▼──────────────────────────────────────┐
│ Rust 后端 (Tauri Commands)                           │
│  - search_qqmusic → 转发到 sidecar                   │
│  - play_qqmusic → 获取 CDN URL → rodio 播放          │
│  - qqmusic_login / check_login → QR 码登录            │
│  - add/remove/get_favorites → 收藏管理               │
└──────────┬──────────────┬────────────────────────────┘
           │ HTTP          │ HTTP
┌──────────▼──────────┐  ┌─▼──────────────────────────┐
│ Node.js Sidecar     │  │ QQ Music CDN               │
│ (127.0.0.1:3456)    │  │ (标准 MP3/AAC 流)          │
│ - 处理签名/加密      │  │                            │
│ - 管理 Cookie       │  │                            │
│ - 转发 QQ Music API │  │                            │
└─────────────────────┘  └────────────────────────────┘
```

### 3.2 核心难点：QQ Music 加密鉴权

QQ 音乐 API 使用 **Sign + JSVMP（JS Virtual Machine Protection）** 加密机制：

- 请求参数经 AES-GCM 加密
- 签名算法运行在 VM 混淆的 JavaScript 中
- 无法直接从 Rust 侧调用，必须依赖已逆向的 Node.js 实现

**结论**：不尝试在 Rust 侧逆向加密算法，使用已成熟的 Node.js 开源项目作为 sidecar。

### 3.3 Sidecar 方案选型

| 项目 | Stars | 框架 | 说明 |
|------|-------|------|------|
| jsososo/QQMusicApi | 1541 | Express | 功能完整，npm 可装，社区活跃 |
| Rain120/qq-music-api | 949 | Koa2 | 文档好，支持 Docker |
| copws/qq-music-api | 活跃 | - | 轻量，2025年仍有更新 |

**推荐**：jsososo/QQMusicApi 或 copws/qq-music-api（根据实际维护状况选其一）

---

## 4. 实施计划

### 阶段 1：Sidecar 搭建
1. 项目根创建 `src-qqmusic/`，初始化 Node.js 项目
2. 引入选定的 QQ Music API 库
3. 实现 Express 路由：
   - `GET /search?q=keyword&page=1` → 搜索结果
   - `GET /url?songmid=xxx` → CDN 播放 URL
   - `POST /login/qrcode` → 获取 QR 码
   - `GET /login/check` → 轮询登录状态
   - `GET /favorites` / `POST /favorites` / `DELETE /favorites`
4. 配置 `tauri.conf.json` shell + externalBin

### 阶段 2：Rust 后端改造
1. **state.rs** — `TrackInfo` 扩展：新增 `source`（Local/QQMusic）、`song_mid`、`cover_url`、`album`
2. **audio.rs** — 新增 `play_url()`：`reqwest::get → bytes → Cursor → rodio::Decoder`
3. **commands.rs** — 新增 8 个 Tauri 命令（搜索/播放/登录/收藏）
4. **lib.rs** — Sidecar 生命周期管理（伴随应用启动/停止）
5. **config.rs** — Cookie 和收藏数据持久化
6. **Cargo.toml** — 新增 `reqwest`、`tokio`（full features）

### 阶段 3：前端改造
1. **types.ts** — 扩展接口类型
2. **commands.ts** — Tauri invoke 包装函数
3. **usePlayer.ts** — 添加搜索/收藏相关状态
4. **Playlist.tsx** — 重构为双视图（本地文件 / QQ 音乐 Tab 切换）
5. **SearchPanel.tsx** — 新建搜索组件（防抖、分页、封面展示）
6. **FavoritesPage.tsx** — 新建收藏列表组件

---

## 5. 涉及文件清单

| 文件 | 类型 | 改动说明 |
|------|------|----------|
| `src-qqmusic/` | **新建** | Node.js sidecar 项目 |
| `src-tauri/Cargo.toml` | 修改 | + `reqwest`, + `tokio` |
| `src-tauri/tauri.conf.json` | 修改 | externalBin / shell 配置 |
| `src-tauri/capabilities/default.json` | 修改 | shell 权限 |
| `src-tauri/src/state.rs` | 修改 | TrackInfo 扩展 |
| `src-tauri/src/audio.rs` | 修改 | + `play_url` 方法 |
| `src-tauri/src/commands.rs` | 修改 | + 8 个新命令 |
| `src-tauri/src/lib.rs` | 修改 | sidecar 启动/停止 |
| `src-tauri/src/config.rs` | 修改 | cookie 持久化 |
| `src/types.ts` | 修改 | 接口类型扩展 |
| `src/commands.ts` | 修改 | invoke 函数包装 |
| `src/hooks/usePlayer.ts` | 修改 | 搜索/收藏状态 |
| `src/components/Playlist.tsx` | 修改 | 双视图 Tab |
| `src/components/SearchPanel.tsx` | **新建** | 搜索组件 |
| `src/components/FavoritesPage.tsx` | **新建** | 收藏列表 |

---

## 6. 风险与应对

| 风险 | 等级 | 应对方案 |
|------|------|----------|
| QQ Music API 加密算法更新 | 🔴 高 | 依赖社区维护的 sidecar 项目及时跟进 |
| Sidecar 进程异常退出 | 🟡 中 | Rust 侧监控进程状态，自动重启 |
| VIP 歌曲试听受限 | 🟡 中 | 匿名模式仅 128kbps，引导用户登录 |
| 音频流 CDN 地址过期 | 🟡 中 | 播放前实时获取最新 CDN URL |
| 应用打包体积增大 | 🟢 低 | Sidecar + Node.js runtime 增加约 30-50MB |

---

## 7. 验收标准

1. `npm run tauri dev` → sidecar 自动启动，控制台无报错
2. 搜索"周杰伦" → 返回带封面、歌手、专辑的歌曲列表
3. 点击搜索结果 → 添加到播放列表 → 播放 → 有声音
4. 播放过程中进度条正常走动，自动下一曲
5. QR 码登录 → 收藏歌曲 → 重启应用 → 收藏数据保留
6. 本地文件与 QQ 音乐歌曲可混合播放、自由切换
7. 搜索防抖正常工作（快速输入不会触发大量请求）
