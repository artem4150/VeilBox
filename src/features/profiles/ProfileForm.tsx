import { useEffect, useMemo, useState } from 'react';
import { Button } from '../../components/Button';
import { Field, TextArea, TextInput } from '../../components/Field';
import type { ManualProfileDraft, Profile } from '../../types';

interface ProfileFormProps {
  profile?: Profile | null;
  onSave: (draft: Partial<Profile>) => Promise<void>;
  onCancel: () => void;
}

const emptyDraft: ManualProfileDraft = {
  protocol: 'vless',
  password: '',
  method: 'aes-256-gcm',
  obfsPassword: '',
  name: '',
  serverAddress: '',
  port: 443,
  uuid: '',
  networkType: 'tcp',
  securityType: 'reality',
  flow: '',
  sni: '',
  fingerprint: 'chrome',
  publicKey: '',
  shortId: '',
  spiderX: '/',
  path: '/',
  hostHeader: '',
  serviceName: '',
  xhttpMode: 'auto',
  transportHeaderType: 'none',
  seed: '',
  alpn: ['h2', 'http/1.1'],
  allowInsecure: false,
  remark: '',
};

function normalizeDraft(profile?: Profile | null): ManualProfileDraft {
  if (!profile) {
    return emptyDraft;
  }

  return {
    name: profile.name,
    protocol: profile.protocol ?? 'vless',
    password: profile.password ?? '',
    method: profile.method ?? 'aes-256-gcm',
    obfsPassword: profile.obfsPassword ?? '',
    serverAddress: profile.serverAddress,
    port: profile.port,
    uuid: profile.uuid,
    networkType: profile.networkType,
    securityType: profile.securityType,
    flow: profile.flow ?? '',
    sni: profile.sni ?? '',
    fingerprint: profile.fingerprint ?? '',
    publicKey: profile.publicKey ?? '',
    shortId: profile.shortId ?? '',
    spiderX: profile.spiderX ?? '',
    path: profile.path ?? '',
    hostHeader: profile.hostHeader ?? '',
    serviceName: profile.serviceName ?? '',
    xhttpMode: profile.xhttpMode ?? 'auto',
    transportHeaderType: profile.transportHeaderType ?? 'none',
    seed: profile.seed ?? '',
    alpn: profile.alpn,
    allowInsecure: profile.allowInsecure,
    remark: profile.remark ?? '',
  };
}

export function ProfileForm({ profile, onSave, onCancel }: ProfileFormProps) {
  const [draft, setDraft] = useState<ManualProfileDraft>(normalizeDraft(profile));
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(normalizeDraft(profile));
  }, [profile]);

  const title = useMemo(
    () => (profile ? 'Редактирование профиля' : 'Новый профиль'),
    [profile],
  );

  const update = <K extends keyof ManualProfileDraft>(key: K, value: ManualProfileDraft[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));

  const submit = async () => {
    setSaving(true);
    try {
      await onSave({
        id: profile?.id,
        engine: 'xray',
        ...draft,
        password: draft.password || null,
        method: draft.protocol === 'shadowsocks' ? draft.method || null : null,
        obfsPassword: draft.protocol === 'hysteria2' ? draft.obfsPassword || null : null,
        flow: draft.flow || null,
        sni: draft.sni || null,
        fingerprint: draft.fingerprint || null,
        publicKey: draft.publicKey || null,
        shortId: draft.shortId || null,
        spiderX: draft.spiderX || null,
        path: draft.path || null,
        hostHeader: draft.hostHeader || null,
        serviceName: draft.serviceName || null,
        xhttpMode: draft.xhttpMode || null,
        transportHeaderType: draft.transportHeaderType || null,
        seed: draft.seed || null,
        remark: draft.remark || null,
        alpn: draft.alpn.filter(Boolean),
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="profile-form">
      <div className="section-title-row">
        <div>
          <h3>{title}</h3>
          <p>VLESS, Shadowsocks и Hysteria2 через Xray.</p>
        </div>
      </div>

      <div className="field-grid">
        <Field label="Protocol">
          <select className="select" value={draft.protocol}
            onChange={(event) => {
              const protocol = event.target.value as ManualProfileDraft['protocol'];
              setDraft((current) => ({...current, protocol,
                securityType: protocol === 'hysteria2' ? 'tls' : protocol === 'shadowsocks' ? 'none' : current.securityType}));
            }}>
            <option value="vless">VLESS</option>
            <option value="shadowsocks">Shadowsocks</option>
            <option value="hysteria2">Hysteria2</option>
          </select>
        </Field>
        <Field label="Имя профиля">
          <TextInput value={draft.name} onChange={(event) => update('name', event.target.value)} />
        </Field>
        <Field label="Адрес сервера">
          <TextInput
            value={draft.serverAddress}
            onChange={(event) => update('serverAddress', event.target.value)}
          />
        </Field>
        <Field label="Порт">
          <TextInput
            type="number"
            min={1}
            max={65535}
            value={draft.port}
            onChange={(event) => update('port', Number(event.target.value))}
          />
        </Field>
        {draft.protocol === 'vless' ? <Field label="UUID">
          <TextInput value={draft.uuid} onChange={(event) => update('uuid', event.target.value)} />
        </Field> : null}
        {draft.protocol !== 'vless' ? <Field label={draft.protocol === 'shadowsocks' ? 'Password' : 'Auth password'}>
          <TextInput type="password" value={draft.password} onChange={(event) => update('password', event.target.value)} />
        </Field> : null}
        {draft.protocol === 'shadowsocks' ? <Field label="Cipher method">
          <TextInput value={draft.method} onChange={(event) => update('method', event.target.value)} />
        </Field> : null}
        {draft.protocol === 'hysteria2' ? <Field label="Salamander obfs password (optional)">
          <TextInput type="password" value={draft.obfsPassword} onChange={(event) => update('obfsPassword', event.target.value)} />
        </Field> : null}
        {draft.protocol === 'vless' ? <>
        <Field label="Тип транспорта">
          <select
            className="select"
            value={draft.networkType}
            onChange={(event) =>
              update('networkType', event.target.value as ManualProfileDraft['networkType'])
            }
          >
            <option value="raw">RAW / TCP</option>
            <option value="tcp">TCP</option>
            <option value="ws">WebSocket</option>
            <option value="grpc">gRPC</option>
            <option value="xhttp">XHTTP</option>
            <option value="httpupgrade">HTTPUpgrade</option>
            <option value="kcp">mKCP</option>
          </select>
        </Field>
        <Field label="Тип безопасности">
          <select
            className="select"
            value={draft.securityType}
            onChange={(event) =>
              update('securityType', event.target.value as ManualProfileDraft['securityType'])
            }
          >
            <option value="none">None</option>
            <option value="reality">Reality</option>
            <option value="tls">TLS</option>
          </select>
        </Field>
        <Field label="Flow">
          <TextInput value={draft.flow} onChange={(event) => update('flow', event.target.value)} />
        </Field>
        <Field label="SNI / имя сервера">
          <TextInput value={draft.sni} onChange={(event) => update('sni', event.target.value)} />
        </Field>
        <Field label="Fingerprint">
          <TextInput
            value={draft.fingerprint}
            onChange={(event) => update('fingerprint', event.target.value)}
          />
        </Field>
        <Field label="Public key">
          <TextInput
            value={draft.publicKey}
            onChange={(event) => update('publicKey', event.target.value)}
          />
        </Field>
        <Field label="Short ID">
          <TextInput value={draft.shortId} onChange={(event) => update('shortId', event.target.value)} />
        </Field>
        <Field label="SpiderX">
          <TextInput value={draft.spiderX} onChange={(event) => update('spiderX', event.target.value)} />
        </Field>
        <Field label="Path">
          <TextInput value={draft.path} onChange={(event) => update('path', event.target.value)} />
        </Field>
        <Field label="Host / Authority">
          <TextInput
            value={draft.hostHeader}
            onChange={(event) => update('hostHeader', event.target.value)}
          />
        </Field>
        <Field label="Service name">
          <TextInput
            value={draft.serviceName}
            onChange={(event) => update('serviceName', event.target.value)}
          />
        </Field>
        <Field label="Режим XHTTP">
          <select
            className="select"
            value={draft.xhttpMode}
            onChange={(event) => update('xhttpMode', event.target.value)}
          >
            <option value="auto">auto</option>
            <option value="packet-up">packet-up</option>
            <option value="stream-up">stream-up</option>
            <option value="stream-one">stream-one</option>
          </select>
        </Field>
        <Field label="Тип transport header">
          <select
            className="select"
            value={draft.transportHeaderType}
            onChange={(event) => update('transportHeaderType', event.target.value)}
          >
            <option value="none">none</option>
            <option value="srtp">srtp</option>
            <option value="utp">utp</option>
            <option value="wechat-video">wechat-video</option>
            <option value="dtls">dtls</option>
            <option value="wireguard">wireguard</option>
          </select>
        </Field>
        <Field label="Seed">
          <TextInput value={draft.seed} onChange={(event) => update('seed', event.target.value)} />
        </Field>
        </> : null}
        {draft.protocol === 'hysteria2' ? <Field label="SNI">
          <TextInput value={draft.sni} onChange={(event) => update('sni', event.target.value)} />
        </Field> : null}
        {draft.protocol !== 'shadowsocks' ? <Field label="ALPN" hint="Через запятую, например h2,http/1.1">
          <TextInput
            value={draft.alpn.join(',')}
            onChange={(event) =>
              update(
                'alpn',
                event.target.value
                  .split(',')
                  .map((value) => value.trim())
                  .filter(Boolean),
              )
            }
          />
        </Field> : null}
      </div>

      <Field label="Описание / remark">
        <TextArea
          rows={4}
          value={draft.remark}
          onChange={(event) => update('remark', event.target.value)}
        />
      </Field>

      {draft.protocol !== 'shadowsocks' ? <label className="checkbox-row">
        <input
          type="checkbox"
          checked={draft.allowInsecure}
          onChange={(event) => update('allowInsecure', event.target.checked)}
        />
        <span>Разрешить небезопасную TLS-проверку</span>
      </label> : null}

      <div className="button-row">
        <Button onClick={() => void submit()} disabled={saving}>
          {saving ? 'Сохранение...' : 'Сохранить профиль'}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          Отмена
        </Button>
      </div>
    </div>
  );
}
