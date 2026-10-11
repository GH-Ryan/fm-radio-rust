// 中文翻译。
//
// 作为 as const 导出，英文版用 satisfies 约束，所以漏译或拼错 key 在编译期
// 就报错，而不是运行时显示成 key 本身。
//
// 参数用 {name} 占位，调用方 t('searchFailed', { error: '...' })。
export const zhCN = {
  // ---- 通用 ----
  close: '关闭',
  unknown: '未知',

  // ---- 标题栏与标签页 ----
  appTitle: 'FM 电台',
  tabStations: '电台',
  tabFavorites: '收藏',

  // ---- 搜索与列表 ----
  searchPlaceholder: '搜索电台名…',
  filterFavoritesPlaceholder: '筛选收藏…',
  stationsCount: '{count}',

  emptyStationsTitle: '没有找到电台',
  emptyStationsHint: '试试其他关键词或检查网络连接',
  emptyFavoritesHint: '还没有收藏，点击电台卡片右上角的星标即可添加',
  favoriteCountBadge: '{count}',

  // ---- 播放器 ----
  playerRegionLabel: '播放器控制',
  unknownStation: '未知电台',
  play: '播放',
  pause: '暂停',
  stop: '停止',
  volume: '音量',
  randomFromBrowse: '随机播放（电台列表）',
  randomFromFavorites: '随机播放（收藏列表）',
  favorite: '收藏',
  unfavorite: '取消收藏',
  mute: '静音',
  unmute: '取消静音',
  cardPlay: '播放',
  cardPause: '暂停',

  // ---- 电台元信息 ----
  unknownCodec: '未知格式',

  // ---- 错误：前端自身产生 ----
  loadFailed: '加载失败: {error}',
  searchFailed: '搜索失败: {error}',
  loadFavoritesFailed: '加载收藏失败: {error}',
  playFailed: '播放失败',
  actionFailed: '操作失败',
  stopFailed: '停止失败',
  randomFailed: '随机播放失败',
  volumeFailed: '音量调整失败: {error}',
  listenFailed: '播放状态监听失败：{error}',
  superseded: '已被新的操作取代',
  closeFailed: '关闭失败: {error}',

  // ---- 错误：后端返回 ----
  // 这些由 Rust 侧产生。前端原样显示后端的中文文案，见 i18n/README.md
  // 里"后端错误码"一节。
  backendError: '{message}',

  // ---- 播放状态 ----
  backendConnecting: '连接中',
} as const

export type TranslationKey = keyof typeof zhCN
export type Translations = Record<TranslationKey, string>