import { Component, type ErrorInfo, type ReactNode } from 'react'

interface Props {
  children: ReactNode
}

interface State {
  error: Error | null
}

/**
 * Keeps a render failure in one place instead of letting React unmount the
 * whole tree.
 *
 * A thrown render error with no boundary empties #root: the window then shows
 * only its background colour, which looks exactly like a frozen webview and is
 * impossible to act on. This fallback at least names the problem and offers a
 * reload.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('界面渲染失败', error, info)
  }

  render() {
    if (!this.state.error) return this.props.children

    return (
      <div
        style={{
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          padding: 24,
          background: '#131313',
          color: '#f2f2f2',
          fontFamily: "'Segoe UI', 'Microsoft YaHei', sans-serif",
          textAlign: 'center',
        }}
      >
        <h1 style={{ fontSize: 18, margin: 0 }}>界面出错了 / Something went wrong</h1>
        <p style={{ fontSize: 13, margin: 0, maxWidth: 640, color: '#8d8d8d', wordBreak: 'break-word' }}>
          {this.state.error.message}
        </p>
        <button
          onClick={() => window.location.reload()}
          style={{
            padding: '8px 16px',
            border: 'none',
            borderRadius: 8,
            background: '#ff6b1a',
            color: '#131313',
            cursor: 'pointer',
          }}
        >
          重新加载 / Reload
        </button>
      </div>
    )
  }
}
