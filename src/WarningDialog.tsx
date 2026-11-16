import Dialog from './Dialog';

export default function WarningDialog({ message, onClose }: { message: string; onClose: () => void }) {
  return <Dialog title="需要处理" role="alertdialog" closeLabel="关闭警告" onClose={onClose}>
    <p role="alert">{message}</p>
  </Dialog>;
}
