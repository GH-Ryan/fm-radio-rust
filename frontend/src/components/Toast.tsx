interface ToastProps {
  message: string
  onClose: () => void
}

export function Toast({ message, onClose }: ToastProps) {
  return (
    <div className="toast" role="alert">
      <span>{message}</span>
      <button
        onClick={onClose}
        style={{
          marginLeft: '12px',
          padding: '0 8px',
          background: 'transparent',
          border: 'none',
          color: 'inherit',
          cursor: 'pointer',
          fontSize: '18px',
          lineHeight: 1,
        }}
        aria-label="关闭"
      >
        ×
      </button>
    </div>
  )
}