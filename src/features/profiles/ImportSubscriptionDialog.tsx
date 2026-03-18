import { useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextInput } from '../../components/Field';

export function ImportSubscriptionDialog({
  onImport,
}: {
  onImport: (url: string) => Promise<void>;
}) {
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);

  const handleImport = async () => {
    if (!url.trim()) {
      return;
    }

    setLoading(true);
    try {
      await onImport(url.trim());
      setUrl('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="import-card">
      <div className="section-title-row">
        <div>
          <h3>Импорт подписки</h3>
          <p>Поддерживаются обычные текстовые и base64-кодированные подписки с несколькими VLESS-ссылками.</p>
        </div>
      </div>
      <Field label="URL подписки">
        <TextInput
          value={url}
          placeholder="https://example.com/subscription.txt"
          onChange={(event) => setUrl(event.target.value)}
        />
      </Field>
      <Button onClick={() => void handleImport()} disabled={loading || !url.trim()}>
        {loading ? 'Импорт...' : 'Импортировать подписку'}
      </Button>
    </div>
  );
}
