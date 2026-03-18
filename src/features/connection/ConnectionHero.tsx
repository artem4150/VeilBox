import { useEffect, useMemo, useState } from 'react';
import { FlaskConical, LoaderCircle, PencilLine, Power, ShieldCheck } from 'lucide-react';
import { Button } from '../../components/Button';
import { Modal } from '../../components/Modal';
import { StatusBadge } from '../../components/StatusBadge';
import { profileSubtitle } from '../../lib/format';
import { t } from '../../lib/i18n';
import { useAppStore } from '../../store/useAppStore';
import { CountryFlag } from '../../components/CountryFlag';

export function ConnectionHero() {
  const status = useAppStore((state) => state.connectionStatus);
  const settings = useAppStore((state) => state.settings);
  const language = settings.language;
  const selectedProfileId = useAppStore((state) => state.selectedProfileId);
  const profiles = useAppStore((state) => state.profiles);
  const profileCountries = useAppStore((state) => state.profileCountries);
  const connect = useAppStore((state) => state.connect);
  const disconnect = useAppStore((state) => state.disconnect);
  const testProfileConnection = useAppStore((state) => state.testProfileConnection);
  const selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) ?? null;
  const selectedCountry = selectedProfile ? profileCountries[selectedProfile.id] : null;
  const busy = status.state === 'connecting';
  const connected = status.state === 'connected';
  const [testing, setTesting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [profileDetailsOpen, setProfileDetailsOpen] = useState(false);
  const [timerNow, setTimerNow] = useState(() => Date.now());

  useEffect(() => {
    if (!(connected && status.connectedAt)) {
      return;
    }

    const interval = window.setInterval(() => {
      setTimerNow(Date.now());
    }, 1000);

    return () => window.clearInterval(interval);
  }, [connected, status.connectedAt]);

  const connectionDuration = useMemo(() => {
    if (!status.connectedAt) {
      return t(language, 'unavailable');
    }

    const startedAt = new Date(status.connectedAt).getTime();
    if (!Number.isFinite(startedAt)) {
      return t(language, 'unavailable');
    }

    const totalSeconds = Math.max(0, Math.floor((timerNow - startedAt) / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':');
  }, [status.connectedAt, timerNow, language]);

  const onPrimaryAction = async () => {
    if (connected) {
      await disconnect();
      return;
    }

    if (selectedProfile) {
      await connect(selectedProfile.id);
    }
  };

  const onTestAction = async () => {
    if (!selectedProfile) {
      return;
    }
    setTesting(true);
    try {
      await testProfileConnection(selectedProfile.id);
    } finally {
      setTesting(false);
    }
  };

  const onEditAction = () => {
    if (!selectedProfile) {
      return;
    }
    setEditing(true);
    window.dispatchEvent(new CustomEvent('vailbox-open-edit-profile'));
    queueMicrotask(() => setEditing(false));
  };

  return (
    <section className={`hero-card${connected ? ' hero-card-connected' : ''}${busy ? ' hero-card-connecting' : ''}`}>
      <div className="hero-content">
        <div>
          <div className="hero-topline">
            <StatusBadge state={status.state} />
          </div>
          <h2>{connected ? t(language, 'heroConnected') : t(language, 'heroReady')}</h2>

          <div className="hero-metrics">
            <div>
              <span>{t(language, 'heroConnectedFor')}</span>
              <strong>{connected ? connectionDuration : t(language, 'unavailable')}</strong>
            </div>
            <div>
              <span>{settings.connectionMode === 'tun' ? 'Режим' : 'HTTP proxy'}</span>
              <strong>
                {settings.connectionMode === 'tun'
                  ? `TUN / ${settings.tunInterfaceName || 'xray0'}`
                  : status.localHttpProxyPort
                    ? `127.0.0.1:${status.localHttpProxyPort}`
                    : t(language, 'heroProxyInactive')}
              </strong>
            </div>
            <div>
              <span>{t(language, 'heroRestarts')}</span>
              <strong>{status.restartCount}</strong>
            </div>
            {selectedProfile ? (
              <button
                type="button"
                className="hero-profile-metric hero-profile-button"
                onClick={() => setProfileDetailsOpen(true)}
              >
                <span>{t(language, 'heroCurrentProfile')}</span>
                <strong className="hero-profile-title">
                  <CountryFlag
                    code={selectedCountry?.countryCode}
                    className="profile-flag"
                    title={selectedCountry?.countryName ?? t(language, 'locationUnknown')}
                  />
                  <span>{selectedProfile.name}</span>
                </strong>
              </button>
            ) : null}
          </div>
        </div>

        <div className="hero-actions">
          <Button
            wide
            className={`hero-connect-button${connected ? ' is-connected' : ''}${busy ? ' is-connecting' : ''}`}
            disabled={busy || (!connected && !selectedProfile)}
            onClick={() => void onPrimaryAction()}
          >
            {busy ? (
              <LoaderCircle size={22} className="hero-connect-icon hero-connect-spinner" />
            ) : (
              <>
                {connected ? (
                  <ShieldCheck size={28} className="hero-connect-icon" />
                ) : (
                  <Power size={28} className="hero-connect-icon" />
                )}
              </>
            )}
            {busy ? t(language, 'heroConnecting') : connected ? t(language, 'connected') : t(language, 'heroConnect')}
            {!connected ? (
              <small className="hero-connect-subtext">
                {busy ? status.message ?? t(language, 'heroPreparing') : t(language, 'heroTapToConnect')}
              </small>
            ) : null}
          </Button>
        </div>
      </div>

      {selectedProfile ? (
        <Modal
          open={profileDetailsOpen}
          title={t(language, 'heroCurrentProfile')}
          onClose={() => setProfileDetailsOpen(false)}
        >
          <div className="hero-profile-modal">
            <strong className="hero-profile-title">
              <CountryFlag
                code={selectedCountry?.countryCode}
                className="profile-flag"
                title={selectedCountry?.countryName ?? t(language, 'locationUnknown')}
              />
              <span>{selectedProfile.name}</span>
            </strong>
            <p>{profileSubtitle(selectedProfile)}</p>
            <div className="button-row hero-inline-actions">
              <Button
                variant="ghost"
                disabled={testing || busy}
                onClick={() => void onTestAction()}
                title={testing ? t(language, 'testingConnection') : t(language, 'testConnection')}
                aria-label={testing ? t(language, 'testingConnection') : t(language, 'testConnection')}
              >
                <FlaskConical size={16} />
              </Button>
              <Button
                variant="ghost"
                disabled={editing || busy}
                onClick={onEditAction}
                title={t(language, 'profileEdit')}
                aria-label={t(language, 'profileEdit')}
              >
                <PencilLine size={16} />
              </Button>
            </div>
          </div>
        </Modal>
      ) : null}
    </section>
  );
}
