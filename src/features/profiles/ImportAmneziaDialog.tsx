import { useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextArea, TextInput } from '../../components/Field';

export function ImportAmneziaDialog({
  onImport,
}: {
  onImport: (config: string, name?: string) => Promise<void>;
}) {
  const [name, setName] = useState('');
  const [config, setConfig] = useState('');
  const [loading, setLoading] = useState(false);

  const handleImport = async () => {
    if (!config.trim()) {
      return;
    }

    setLoading(true);
    try {
      await onImport(config.trim(), name.trim() || undefined);
      setName('');
      setConfig('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="import-card">
      <div className="section-title-row">
        <div>
          <h3>Import AmneziaWG</h3>
          <p>Paste an Amnezia/WireGuard-style config with [Interface] and [Peer] sections.</p>
        </div>
      </div>
      <div className="field-grid">
        <Field label="Display name" hint="Optional. If empty, a name will be generated from Endpoint.">
          <TextInput
            value={name}
            placeholder="Amnezia Netherlands"
            onChange={(event) => setName(event.target.value)}
          />
        </Field>
      </div>
      <Field label="Amnezia config">
        <TextArea
          rows={12}
          value={config}
          placeholder={`[Interface]\nPrivateKey = ...\nAddress = 10.8.0.2/32\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = ...\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = example.com:51820\n\nor paste vpn://... share link`}
          onChange={(event) => setConfig(event.target.value)}
        />
      </Field>
      <Button onClick={() => void handleImport()} disabled={loading || !config.trim()}>
        {loading ? 'Importing...' : 'Import Amnezia config'}
      </Button>
    </div>
  );
}
