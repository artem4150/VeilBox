import { useEffect, useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextArea, TextInput } from '../../components/Field';
import type { Profile } from '../../types';

interface AmneziaProfileFormProps {
  profile?: Profile | null;
  onSave: (draft: Partial<Profile>) => Promise<void>;
  onCancel: () => void;
}

export function AmneziaProfileForm({ profile, onSave, onCancel }: AmneziaProfileFormProps) {
  const [name, setName] = useState(profile?.name ?? '');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setName(profile?.name ?? '');
  }, [profile]);

  if (!profile?.amneziaConfig) {
    return null;
  }

  const submit = async () => {
    setSaving(true);
    try {
      await onSave({
        id: profile.id,
        engine: 'amneziawg',
        name: name.trim() || profile.name,
        serverAddress: profile.serverAddress,
        port: profile.port,
        uuid: profile.uuid,
        networkType: profile.networkType,
        securityType: profile.securityType,
        flow: profile.flow,
        sni: profile.sni,
        fingerprint: profile.fingerprint,
        publicKey: profile.publicKey,
        shortId: profile.shortId,
        spiderX: profile.spiderX,
        path: profile.path,
        hostHeader: profile.hostHeader,
        serviceName: profile.serviceName,
        xhttpMode: profile.xhttpMode,
        transportHeaderType: profile.transportHeaderType,
        seed: profile.seed,
        alpn: profile.alpn,
        allowInsecure: profile.allowInsecure,
        remark: profile.remark,
        source: profile.source,
        sourceLabel: profile.sourceLabel,
        subscriptionId: profile.subscriptionId,
        amneziaConfig: profile.amneziaConfig,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="profile-form">
      <div className="section-title-row">
        <div>
          <h3>AmneziaWG profile</h3>
          <p>Unified profile shell is ready. This build currently lets you rename and inspect the imported config.</p>
        </div>
      </div>

      <Field label="Display name">
        <TextInput value={name} onChange={(event) => setName(event.target.value)} />
      </Field>

      <Field label="Endpoint">
        <TextInput value={`${profile.serverAddress}:${profile.port}`} readOnly />
      </Field>

      <Field label="Imported config">
        <TextArea rows={12} value={profile.amneziaConfig.rawConfig} readOnly />
      </Field>

      <div className="button-row">
        <Button onClick={() => void submit()} disabled={saving}>
          {saving ? 'Saving...' : 'Save profile'}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
