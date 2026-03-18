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
import { Panel } from '../components/Panel';
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
  const duplicateProfile = useAppStore((state) => state.duplicateProfile);
  const deleteProfile = useAppStore((state) => state.deleteProfile);
  const refreshSubscription = useAppStore((state) => state.refreshSubscription);
  const refreshAllSubscriptions = useAppStore((state) => state.refreshAllSubscriptions);
  const deleteSubscription = useAppStore((state) => state.deleteSubscription);
  const refreshLatencies = useAppStore((state) => state.refreshLatencies);
  const currentProfile = useMemo(
    () => profiles.find((profile) => profile.id === selectedProfileId) ?? null,
    [profiles, selectedProfileId],
  );

  const [modalMode, setModalMode] = useState<ModalMode>('new');
  const [modalOpen, setModalOpen] = useState(false);
  const [pinging, setPinging] = useState(false);
  const [syncingAllSubscriptions, setSyncingAllSubscriptions] = useState(false);
  const [subscriptionActionId, setSubscriptionActionId] = useState<string | null>(null);
  const [collapsedSubscriptions, setCollapsedSubscriptions] = useState<Record<string, boolean>>(
    {},
  );
  const [search, setSearch] = useState('');

  const openModal = (mode: ModalMode) => {
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
      aLatency?.status === 'ok' ? (aLatency.latencyMs ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
    const bScore =
      bLatency?.status === 'ok' ? (bLatency.latencyMs ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;

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

  const ungroupedSubscriptionProfiles = useMemo(
    () =>
      profiles
        .filter(
          (profile) =>
            profile.source === 'subscription' &&
            (!profile.subscriptionId ||
              !subscriptions.some((subscription) => subscription.id === profile.subscriptionId)),
        )
        .filter(matchesSearch)
        .sort(compareProfiles),
    [profiles, subscriptions, latencies, profileCountries, selectedProfileId, normalizedSearch],
  );

  const hasVisibleProfiles =
    manualProfiles.length > 0 ||
    subscriptionGroups.some((group) => group.profiles.length > 0) ||
    ungroupedSubscriptionProfiles.length > 0;

  const handleRefreshAllSubscriptions = async () => {
    setSyncingAllSubscriptions(true);
    try {
      await refreshAllSubscriptions();
    } finally {
      setSyncingAllSubscriptions(false);
    }
  };

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
    <div className="page">
      <div className="page-header">
        <div>
          <span className="eyebrow">{t(language, 'dashboardEyebrow')}</span>
          <h1>{t(language, 'dashboardTitle')}</h1>
        </div>
      </div>

      <ConnectionHero />

      <div className="dashboard-grid dashboard-grid-single">
        <Panel
          className="profiles-panel"
          title={t(language, 'profilesTitle')}
          action={
            <div className="button-row profiles-toolbar">
              <Button variant="secondary" onClick={() => openModal('new')}>
                <Plus size={16} />
                {t(language, 'profileNew')}
              </Button>
              <Button variant="ghost" onClick={() => openModal('uri')}>
                <Upload size={16} />
                VLESS URI
              </Button>
              <Button variant="ghost" onClick={() => openModal('json')}>
                <FileJson size={16} />
                JSON
              </Button>
              <Button variant="ghost" onClick={() => openModal('subscription')}>
                <Link2 size={16} />
                {t(language, 'subscriptionAction')}
              </Button>
              <Button
                variant="ghost"
                disabled={syncingAllSubscriptions || subscriptions.length === 0}
                onClick={() => void handleRefreshAllSubscriptions()}
              >
                <RefreshCw
                  size={16}
                  className={syncingAllSubscriptions ? 'icon-spin' : undefined}
                />
                {syncingAllSubscriptions
                  ? t(language, 'subscriptionsUpdating')
                  : t(language, 'subscriptionsUpdateAll')}
              </Button>
              <Button
                variant="ghost"
                disabled={pinging || profiles.length === 0}
                onClick={() => void pingAll()}
              >
                <RefreshCw size={16} className={pinging ? 'icon-spin' : undefined} />
                {pinging ? t(language, 'pinging') : t(language, 'pingAll')}
              </Button>
            </div>
          }
        >
          {profiles.length ? (
            <div className="profile-list">
              <label className="profile-search profiles-search-compact">
                <Search size={16} />
                <input
                  className="input profile-search-input"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder={t(language, 'profilesSearchPlaceholder')}
                />
              </label>

              {manualProfiles.length > 0 ? (
                <>
                  {subscriptionGroups.length > 0 || ungroupedSubscriptionProfiles.length > 0 ? (
                    <div className="profile-group-label">{t(language, 'profilesManualGroup')}</div>
                  ) : null}
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
                      onDuplicate={() => void duplicateProfile(profile.id)}
                      onDelete={() => void deleteProfile(profile.id)}
                    />
                  ))}
                </>
              ) : null}

              {subscriptionGroups.map(({ subscription, profiles: groupProfiles }) =>
                groupProfiles.length > 0 ? (
                  <div key={subscription.id} className="subscription-group">
                    <div className="subscription-group-header">
                      <button
                        type="button"
                        className="subscription-group-toggle"
                        onClick={() => toggleSubscription(subscription.id)}
                      >
                        <ChevronDown
                          size={16}
                          className={`subscription-chevron${
                            collapsedSubscriptions[subscription.id] ? ' is-collapsed' : ''
                          }`}
                        />
                        <div className="subscription-group-title-row">
                          <strong>{subscription.name}</strong>
                          <span className="subscription-group-meta">
                            {groupProfiles.length} {t(language, 'subscriptionProfilesCount')} ·{' '}
                            {t(language, 'subscriptionUpdatedAt')}{' '}
                            {formatTimestamp(subscription.updatedAt, language)}
                          </span>
                        </div>
                      </button>
                      <div className="subscription-group-actions">
                        <Button
                          variant="ghost"
                          disabled={subscriptionActionId === subscription.id}
                          onClick={() => void handleRefreshSubscription(subscription.id)}
                          title={t(language, 'subscriptionRefresh')}
                        >
                          <RefreshCw
                            size={15}
                            className={subscriptionActionId === subscription.id ? 'icon-spin' : undefined}
                          />
                          {t(language, 'subscriptionRefresh')}
                        </Button>
                        <Button
                          variant="ghost"
                          className="danger-ghost"
                          disabled={subscriptionActionId === subscription.id}
                          onClick={() => void handleDeleteSubscription(subscription.id)}
                          title={t(language, 'subscriptionDelete')}
                        >
                          <Trash2 size={15} />
                          {t(language, 'subscriptionDelete')}
                        </Button>
                      </div>
                    </div>
                    {!collapsedSubscriptions[subscription.id] ? (
                      <div className="subscription-group-body">
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
                            onDuplicate={() => void duplicateProfile(profile.id)}
                            onDelete={() => void deleteProfile(profile.id)}
                          />
                        ))}
                      </div>
                    ) : null}
                  </div>
                ) : null,
              )}

              {ungroupedSubscriptionProfiles.length > 0 ? (
                <>
                  <div className="profile-group-label">
                    {t(language, 'profilesSubscriptionGroup')}
                  </div>
                  {ungroupedSubscriptionProfiles.map((profile) => (
                    <ProfileListItem
                      key={profile.id}
                      profile={profile}
                      selected={selectedProfileId === profile.id}
                      active={selectedProfileId === profile.id}
                      latency={latencies[profile.id]}
                      country={profileCountries[profile.id]}
                      onSelect={() => void selectProfile(profile.id)}
                      onActivate={() => void selectProfile(profile.id)}
                      onDuplicate={() => void duplicateProfile(profile.id)}
                      onDelete={() => void deleteProfile(profile.id)}
                    />
                  ))}
                </>
              ) : null}

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
        </Panel>

      </div>

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
