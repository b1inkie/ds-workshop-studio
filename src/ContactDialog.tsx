import { MessageCircle } from 'lucide-react';
import Dialog from './Dialog';
import { contactInfo } from './contactInfo';

export default function ContactDialog({ onClose }: { onClose: () => void }) {
  const entries = contactInfo.entries.filter(entry => entry.value.trim());
  return <Dialog title={contactInfo.title} onClose={onClose}>
    <div className="contact-intro"><MessageCircle size={26} /><p>{contactInfo.description}</p></div>
    {entries.length ? <dl className="contact-entries">{entries.map(entry => <div key={entry.label}><dt>{entry.label}</dt><dd>{entry.value}</dd></div>)}</dl> : <div className="contact-placeholder"><strong>交流信息待补充</strong><p>后续将在这里展示 QQ 群号和其他联系方式。</p></div>}
  </Dialog>;
}
