import { useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextArea } from '../../components/Field';

export function ImportProfileDialog({
  onImport,
}: {
  onImport: (uri: string) => Promise<boolean>;
}) {
  const [uri, setUri] = useState('');
  const [loading, setLoading] = useState(false);

  const handleImport = async () => {
    if (!uri.trim()) {
      return;
    }

    setLoading(true);
    try {
      if (await onImport(uri.trim())) {
        setUri('');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="import-card">
      <div className="section-title-row">
        <div>
          <h3>Import VLESS / Shadowsocks / Hysteria2</h3>
          <p>Некорректные ссылки отклоняются с явной ошибкой парсера.</p>
        </div>
      </div>
      <Field label="vless://, ss://, hy2:// or hysteria2:// URI">
        <TextArea
          rows={4}
          value={uri}
          placeholder="vless://uuid@example.com:443?type=ws&security=tls&path=%2Fws#Мой профиль"
          onChange={(event) => setUri(event.target.value)}
        />
      </Field>
      <Button onClick={() => void handleImport()} disabled={loading || !uri.trim()}>
        {loading ? 'Импорт...' : 'Импортировать URI'}
      </Button>
    </div>
  );
}
