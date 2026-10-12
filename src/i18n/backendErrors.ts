import type { BackendErrorCode } from '../lib/errors'

/**
 * What the UI says for each backend failure.
 *
 * The backend reports a code and a message written in the app's source
 * language. This table is what turns that code into the reader's language;
 * anything absent falls back to the message the backend sent, so a new code is
 * never silent -- it just is not translated yet.
 *
 * `function` rather than a plain string where the copy needs the detail, which
 * is where a bare "HTTP 404" on its own tells the user nothing.
 */
export const backendErrors: Partial<
  Record<BackendErrorCode, string | ((detail?: string) => string)>
> = {
  output_unavailable: '音频输出不可用：{detail}',
  connection_failed: '无法连接该电台：{detail}',
  unsupported_format: '该电台的音频格式无法识别，本应用无法解码',
  format_not_supported: '本应用无法解码该电台的格式',
  decode_error: '解码失败：{detail}',
  stream_ended: '电台已停止传输',
  cancelled: '操作已取消',
  nothing_playing: '没有正在播放的电台',
  empty_list: '当前列表为空',
  unknown_list: '未知列表：{detail}',
  network: '网络请求失败：{detail}',
  json: '数据解析失败：{detail}',
  all_mirrors_failed: '所有镜像均不可用：{detail}',
  no_mirrors: '未找到可用的镜像服务',
  http: '服务器返回错误：{detail}',
  io: '读写失败：{detail}',
  url_parse: '地址格式错误：{detail}',
  config_dir_unavailable: '配置目录不可用',
  superseded: '已被新的操作取代',
}

export const backendErrorsEn: typeof backendErrors = {
  output_unavailable: 'Audio output unavailable: {detail}',
  connection_failed: 'Could not reach this station: {detail}',
  unsupported_format: 'This station uses a format the app cannot decode',
  format_not_supported: 'The app cannot decode this station’s format',
  decode_error: 'Decoding failed: {detail}',
  stream_ended: 'The station stopped transmitting',
  cancelled: 'Cancelled',
  nothing_playing: 'Nothing is playing',
  empty_list: 'This list is empty',
  unknown_list: 'Unknown list: {detail}',
  network: 'Network request failed: {detail}',
  json: 'Could not read the response: {detail}',
  all_mirrors_failed: 'No mirror responded: {detail}',
  no_mirrors: 'No mirror service could be found',
  http: 'The server returned an error: {detail}',
  io: 'Read or write failed: {detail}',
  url_parse: 'The address could not be read: {detail}',
  config_dir_unavailable: 'The configuration folder is not available',
  superseded: 'Replaced by a newer request',
}