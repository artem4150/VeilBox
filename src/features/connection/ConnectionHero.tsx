import { useEffect, useMemo, useRef, useState } from 'react';
import { FlaskConical, PencilLine } from 'lucide-react';
import { Button } from '../../components/Button';
import { Modal } from '../../components/Modal';
import { HeroShutterText } from '../../components/ui/hero-shutter-text';
import { profileSubtitle } from '../../lib/format';
import { t } from '../../lib/i18n';
import { useAppStore } from '../../store/useAppStore';
import { CountryFlag } from '../../components/CountryFlag';

type HeroDisplayState = 'disconnected' | 'connecting' | 'connected' | 'failed';

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
  const [displayState, setDisplayState] = useState<HeroDisplayState>(
    status.state === 'error' ? 'disconnected' : status.state,
  );
  const connectingStartedAtRef = useRef<number | null>(null);
  const transitionTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (!(connected && status.connectedAt)) {
      return;
    }

    const interval = window.setInterval(() => {
      setTimerNow(Date.now());
    }, 1000);

    return () => window.clearInterval(interval);
  }, [connected, status.connectedAt]);

  useEffect(() => {
    const clearPendingTransition = () => {
      if (transitionTimerRef.current != null) {
        window.clearTimeout(transitionTimerRef.current);
        transitionTimerRef.current = null;
      }
    };

    if (status.state === 'connecting') {
      clearPendingTransition();
      connectingStartedAtRef.current = Date.now();
      setDisplayState('connecting');
      return () => clearPendingTransition();
    }

    const elapsed = connectingStartedAtRef.current
      ? Date.now() - connectingStartedAtRef.current
      : 720;
    const remaining = Math.max(0, 720 - elapsed);

    if (status.state === 'error') {
      clearPendingTransition();
      transitionTimerRef.current = window.setTimeout(() => {
        setDisplayState('failed');
        transitionTimerRef.current = window.setTimeout(() => {
          transitionTimerRef.current = null;
          connectingStartedAtRef.current = null;
          setDisplayState('disconnected');
        }, 2000);
      }, remaining);

      return () => clearPendingTransition();
    }

    const nextStableState: HeroDisplayState =
      status.state === 'connected' ? 'connected' : 'disconnected';

    if (remaining > 0) {
      clearPendingTransition();
      transitionTimerRef.current = window.setTimeout(() => {
        transitionTimerRef.current = null;
        connectingStartedAtRef.current = null;
        setDisplayState(nextStableState);
      }, remaining);

      return () => clearPendingTransition();
    }

    clearPendingTransition();
    connectingStartedAtRef.current = null;
    setDisplayState(nextStableState);

    return () => clearPendingTransition();
  }, [status.state]);

  const connectionDuration = useMemo(() => {
    if (!status.connectedAt) {
      return '00:00:00';
    }

    const startedAt = new Date(status.connectedAt).getTime();
    if (!Number.isFinite(startedAt)) {
      return '00:00:00';
    }

    const totalSeconds = Math.max(0, Math.floor((timerNow - startedAt) / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':');
  }, [status.connectedAt, timerNow]);

  const title = displayState === 'connecting'
    ? t(language, 'heroConnecting')
    : displayState === 'failed'
      ? t(language, 'heroFailed')
    : displayState === 'connected'
      ? t(language, 'connected')
      : t(language, 'heroConnect');
  const titleTone =
    displayState === 'connecting'
      ? 'warning'
      : displayState === 'failed'
        ? 'error'
      : displayState === 'connected'
        ? 'success'
        : 'default';

  const modeLabel =
    settings.connectionMode === 'tun'
      ? `TUN ${settings.tunInterfaceName || 'xray0'}`
      : status.localHttpProxyPort
        ? `127.0.0.1:${status.localHttpProxyPort}`
        : t(language, 'heroProxyInactive');

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
    <section className="dashboard-hero">
      <button
        type="button"
        className={`dashboard-hero-title${
          displayState === 'connected' ? ' is-connected' : ''
        }${displayState === 'connecting' ? ' is-connecting' : ''}${
          displayState === 'failed' ? ' is-failed' : ''
        }`}
        onClick={() => void onPrimaryAction()}
        disabled={busy || (!connected && !selectedProfile)}
      >
        <HeroShutterText text={title} tone={titleTone} />
      </button>

      <div className="dashboard-hero-meta">
        <span>
          {t(language, 'heroConnectedFor')} {connectionDuration}
        </span>
        <span>|</span>
        <span>HTTP proxy {modeLabel}</span>
        <span>|</span>
        <span>
          {t(language, 'heroRestarts')} {status.restartCount}
        </span>
        {selectedProfile ? (
          <span className="dashboard-hero-profile-meta">
            <span>|</span>
            <button
              type="button"
              className="dashboard-hero-profile-link"
              onClick={() => setProfileDetailsOpen(true)}
            >
              {t(language, 'heroCurrentProfile')}:&nbsp;
              <CountryFlag
                code={selectedCountry?.countryCode}
                className="profile-flag"
                title={selectedCountry?.countryName ?? t(language, 'locationUnknown')}
              />
              <span>{selectedProfile.name}</span>
            </button>
          </span>
        ) : null}
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
