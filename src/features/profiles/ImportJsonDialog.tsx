import { useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextArea } from '../../components/Field';

export function ImportJsonDialog({
  onImport,
}: {
  onImport: (json: string) => Promise<void>;
}) {
  const [json, setJson] = useState('');
  const [loading, setLoading] = useState(false);

  const handleImport = async () => {
    if (!json.trim()) {
      return;
    }

    setLoading(true);
    try {
      await onImport(json.trim());
      setJson('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="import-card">
      <div className="section-title-row">
        <div>
          <h3>Импорт из JSON</h3>
          <p>Поддерживается один профиль, массив профилей или объект с полем `profiles`.</p>
        </div>
      </div>
      <Field label="JSON профиля">
        <TextArea
          rows={8}
          value={json}
          placeholder='{"name":"Мой профиль","serverAddress":"example.com","port":443}'
          onChange={(event) => setJson(event.target.value)}
        />
      </Field>
      <Button onClick={() => void handleImport()} disabled={loading || !json.trim()}>
        {loading ? 'Импорт...' : 'Импортировать JSON'}
      </Button>
    </div>
  );
}
