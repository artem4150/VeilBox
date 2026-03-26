import {
  ChevronDown,
  FileJson,
  Link2,
  Plus,
  RefreshCw,
  Search,
  Trash2,
  Upload,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Button } from '../components/Button';
import { EmptyState } from '../components/EmptyState';
import { Modal } from '../components/Modal';
import { formatTimestamp } from '../lib/format';
import { t } from '../lib/i18n';
import { ConnectionHero } from '../features/connection/ConnectionHero';
import { ImportJsonDialog } from '../features/profiles/ImportJsonDialog';
import { ImportProfileDialog } from '../features/profiles/ImportProfileDialog';
import { ImportSubscriptionDialog } from '../features/profiles/ImportSubscriptionDialog';
import { ProfileForm } from '../features/profiles/ProfileForm';
import { ProfileListItem } from '../features/profiles/ProfileListItem';
import { useAppStore } from '../store/useAppStore';
import type { Profile } from '../types';

type ModalMode = 'new' | 'edit' | 'uri' | 'json' | 'subscription';

export function DashboardPage() {
  const profiles = useAppStore((state) => state.profiles);
  const latencies = useAppStore((state) => state.latencies);
  const profileCountries = useAppStore((state) => state.profileCountries);
  const subscriptions = useAppStore((state) => state.subscriptions);
  const selectedProfileId = useAppStore((state) => state.selectedProfileId);
  const language = useAppStore((state) => state.settings.language);
  const selectProfile = useAppStore((state) => state.selectProfile);
  const saveProfile = useAppStore((state) => state.saveProfile);
  const importProfile = useAppStore((state) => state.importProfile);
  const importProfilesJson = useAppStore((state) => state.importProfilesJson);
  const importSubscription = useAppStore((state) => state.importSubscription);
  const deleteProfile = useAppStore((state) => state.deleteProfile);
  const refreshSubscription = useAppStore((state) => state.refreshSubscription);
  const deleteSubscription = useAppStore((state) => state.deleteSubscription);
  const refreshLatencies = useAppStore((state) => state.refreshLatencies);
  const currentProfile = useMemo(
    () => profiles.find((profile) => profile.id === selectedProfileId) ?? null,
    [profiles, selectedProfileId],
  );

  const [modalMode, setModalMode] = useState<ModalMode>('new');
  const [modalOpen, setModalOpen] = useState(false);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const [pinging, setPinging] = useState(false);
  const [subscriptionActionId, setSubscriptionActionId] = useState<string | null>(null);
  const [collapsedSubscriptions, setCollapsedSubscriptions] = useState<Record<string, boolean>>(
    {},
  );
  const [search, setSearch] = useState('');

  const openModal = (mode: ModalMode) => {
    setCreateMenuOpen(false);
    setModalMode(mode);
    setModalOpen(true);
  };

  const closeModal = () => setModalOpen(false);

  const pingAll = async () => {
    setPinging(true);
    try {
      await refreshLatencies();
    } finally {
      setPinging(false);
    }
  };

  const toggleSubscription = (subscriptionId: string) => {
    setCollapsedSubscriptions((current) => ({
      ...current,
      [subscriptionId]: !current[subscriptionId],
    }));
  };

  const normalizedSearch = search.trim().toLowerCase();

  const matchesSearch = (profile: Profile) => {
    if (!normalizedSearch) {
      return true;
    }
    const country = profileCountries[profile.id];
    return [profile.name, country?.countryName ?? '', country?.countryCode ?? '']
      .join(' ')
      .toLowerCase()
      .includes(normalizedSearch);
  };

  const compareProfiles = (a: Profile, b: Profile) => {
    const aActive = a.id === selectedProfileId ? 1 : 0;
    const bActive = b.id === selectedProfileId ? 1 : 0;
    if (aActive !== bActive) {
      return bActive - aActive;
    }

    const aLatency = latencies[a.id];
    const bLatency = latencies[b.id];
    const aScore =
      aLatency?.status === 'ok'
        ? (aLatency.latencyMs ?? Number.MAX_SAFE_INTEGER)
        : Number.MAX_SAFE_INTEGER;
    const bScore =
      bLatency?.status === 'ok'
        ? (bLatency.latencyMs ?? Number.MAX_SAFE_INTEGER)
        : Number.MAX_SAFE_INTEGER;

    if (aScore !== bScore) {
      return aScore - bScore;
    }

    return a.name.localeCompare(b.name);
  };

  const manualProfiles = useMemo(
    () =>
      profiles
        .filter((profile) => profile.source !== 'subscription')
        .filter(matchesSearch)
        .sort(compareProfiles),
    [profiles, latencies, profileCountries, selectedProfileId, normalizedSearch],
  );

  const subscriptionGroups = useMemo(
    () =>
      subscriptions
        .map((subscription) => ({
          subscription,
          profiles: profiles
            .filter((profile) => profile.subscriptionId === subscription.id)
            .filter(matchesSearch)
            .sort(compareProfiles),
        }))
        .filter((entry) => entry.profiles.length > 0 || !normalizedSearch),
    [profiles, subscriptions, latencies, profileCountries, selectedProfileId, normalizedSearch],
  );

  const hasVisibleProfiles =
    manualProfiles.length > 0 || subscriptionGroups.some((group) => group.profiles.length > 0);

  const handleRefreshSubscription = async (subscriptionId: string) => {
    setSubscriptionActionId(subscriptionId);
    try {
      await refreshSubscription(subscriptionId);
    } finally {
      setSubscriptionActionId((current) => (current === subscriptionId ? null : current));
    }
  };

  const handleDeleteSubscription = async (subscriptionId: string) => {
    setSubscriptionActionId(subscriptionId);
    try {
      await deleteSubscription(subscriptionId);
    } finally {
      setSubscriptionActionId((current) => (current === subscriptionId ? null : current));
    }
  };

  const modalTitle =
    modalMode === 'new'
      ? `${t(language, 'profileNew')} ${t(language, 'profileEntity')}`
      : modalMode === 'edit'
        ? `${t(language, 'profileEdit')} ${t(language, 'profileEntity')}`
        : modalMode === 'uri'
          ? t(language, 'importUri')
          : modalMode === 'json'
            ? t(language, 'importJson')
            : t(language, 'importSubscription');

  useEffect(() => {
    const handler = () => {
      if (selectedProfileId) {
        openModal('edit');
      }
    };
    window.addEventListener('vailbox-open-edit-profile', handler as EventListener);
    return () => {
      window.removeEventListener('vailbox-open-edit-profile', handler as EventListener);
    };
  }, [selectedProfileId]);

  return (
    <div className="page dashboard-page dashboard-page-reference">
      <ConnectionHero />

      <section className="dashboard-toolbar-row">
        <div className="dashboard-toolbar-left">
          <button
            type="button"
            className="dashboard-toolbar-button"
            onClick={() => setCreateMenuOpen(true)}
          >
            <Plus size={18} />
            <span>{t(language, 'profileNew')}</span>
          </button>
          <button
            type="button"
            className="dashboard-toolbar-button"
            disabled={pinging || profiles.length === 0}
            onClick={() => void pingAll()}
          >
            <RefreshCw size={18} className={pinging ? 'icon-spin' : undefined} />
            <span>{pinging ? t(language, 'pinging') : t(language, 'pingAll')}</span>
          </button>
        </div>

        <label className="dashboard-search">
          <Search size={20} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t(language, 'profilesSearchPlaceholder')}
          />
        </label>
      </section>

      <section className="dashboard-list-shell">
        {profiles.length ? (
          <div className="dashboard-flat-list">
            {manualProfiles.map((profile) => (
              <ProfileListItem
                key={profile.id}
                profile={profile}
                selected={selectedProfileId === profile.id}
                active={selectedProfileId === profile.id}
                latency={latencies[profile.id]}
                country={profileCountries[profile.id]}
                onSelect={() => void selectProfile(profile.id)}
                onActivate={() => void selectProfile(profile.id)}
                onDuplicate={() => undefined}
                onDelete={() => void deleteProfile(profile.id)}
              />
            ))}

            {subscriptionGroups.map(({ subscription, profiles: groupProfiles }) =>
              groupProfiles.length > 0 ? (
                <div key={subscription.id} className="dashboard-subscription">
                  <div className="dashboard-subscription-row">
                    <button
                      type="button"
                      className="dashboard-subscription-toggle"
                      onClick={() => toggleSubscription(subscription.id)}
                    >
                      <ChevronDown
                        size={16}
                        className={`subscription-chevron${
                          collapsedSubscriptions[subscription.id] ? ' is-collapsed' : ''
                        }`}
                      />
                      <div className="dashboard-subscription-meta">
                        <strong>{subscription.name}</strong>
                        <span className="dashboard-subscription-count">
                          {groupProfiles.length} {t(language, 'subscriptionProfilesCount')}
                        </span>
                        <span className="dashboard-subscription-date">
                          {formatTimestamp(subscription.updatedAt, language)}
                        </span>
                      </div>
                    </button>
                    <div className="dashboard-subscription-actions">
                      <button
                        type="button"
                        className="dashboard-inline-chip"
                        disabled={subscriptionActionId === subscription.id}
                        onClick={() => void handleRefreshSubscription(subscription.id)}
                      >
                        {t(language, 'subscriptionRefresh')}
                      </button>
                      <button
                        type="button"
                        className="dashboard-delete-icon"
                        disabled={subscriptionActionId === subscription.id}
                        onClick={() => void handleDeleteSubscription(subscription.id)}
                        aria-label={t(language, 'subscriptionDelete')}
                        title={t(language, 'subscriptionDelete')}
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  </div>
                  {!collapsedSubscriptions[subscription.id] ? (
                    <div className="dashboard-subscription-list">
                      {groupProfiles.map((profile) => (
                        <ProfileListItem
                          key={profile.id}
                          profile={profile}
                          selected={selectedProfileId === profile.id}
                          active={selectedProfileId === profile.id}
                          latency={latencies[profile.id]}
                          country={profileCountries[profile.id]}
                          onSelect={() => void selectProfile(profile.id)}
                          onActivate={() => void selectProfile(profile.id)}
                          onDuplicate={() => undefined}
                          onDelete={() => void deleteProfile(profile.id)}
                        />
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : null,
            )}

            {!hasVisibleProfiles ? (
              <EmptyState
                title={t(language, 'profilesSearchEmptyTitle')}
                message={t(language, 'profilesSearchEmptyBody')}
              />
            ) : null}
          </div>
        ) : (
          <EmptyState
            title={t(language, 'noProfilesTitle')}
            message={t(language, 'noProfilesBody')}
          />
        )}
      </section>

      <Modal
        open={createMenuOpen}
        title={t(language, 'profileNew')}
        onClose={() => setCreateMenuOpen(false)}
      >
        <div className="import-option-grid">
          <button type="button" className="import-option-button" onClick={() => openModal('new')}>
            <Plus size={18} />
            <span>{t(language, 'profileNew')}</span>
          </button>
          <button type="button" className="import-option-button" onClick={() => openModal('uri')}>
            <Upload size={18} />
            <span>VLESS URI</span>
          </button>
          <button type="button" className="import-option-button" onClick={() => openModal('json')}>
            <FileJson size={18} />
            <span>JSON</span>
          </button>
          <button
            type="button"
            className="import-option-button"
            onClick={() => openModal('subscription')}
          >
            <Link2 size={18} />
            <span>{t(language, 'subscriptionAction')}</span>
          </button>
        </div>
      </Modal>

      <Modal open={modalOpen} title={modalTitle} onClose={closeModal}>
        {modalMode === 'uri' ? (
          <ImportProfileDialog
            onImport={async (uri) => {
              await importProfile(uri);
              closeModal();
            }}
          />
        ) : modalMode === 'json' ? (
          <ImportJsonDialog
            onImport={async (json) => {
              await importProfilesJson(json);
              closeModal();
            }}
          />
        ) : modalMode === 'subscription' ? (
          <ImportSubscriptionDialog
            onImport={async (url) => {
              await importSubscription(url);
              closeModal();
            }}
          />
        ) : (
          <ProfileForm
            profile={modalMode === 'edit' ? currentProfile : null}
            onSave={async (draft) => {
              await saveProfile(draft);
              closeModal();
            }}
            onCancel={closeModal}
          />
        )}
      </Modal>
    </div>
  );
}
