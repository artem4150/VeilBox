import { Trash2 } from 'lucide-react';
import { CountryFlag } from '../../components/CountryFlag';
import { formatLatency } from '../../lib/format';
import { t } from '../../lib/i18n';
import { useAppStore } from '../../store/useAppStore';
import type { Profile, ProfileCountry, ProfileLatency } from '../../types';

interface ProfileListItemProps {
  profile: Profile;
  active: boolean;
  selected: boolean;
  latency?: ProfileLatency | null;
  country?: ProfileCountry | null;
  onSelect: () => void;
  onActivate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

export function ProfileListItem({
  profile,
  active,
  selected,
  latency,
  country,
  onSelect,
  onDelete,
}: ProfileListItemProps) {
  const language = useAppStore((state) => state.settings.language);
  const latencyTone = latency?.status ?? 'checking';

  return (
    <div
      role="button"
      tabIndex={0}
      className={`profile-list-item dashboard-profile-row${selected ? ' profile-list-item-selected' : ''}`}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
        }
      }}
    >
      <div className="dashboard-profile-main">
        <CountryFlag
          code={country?.countryCode}
          className="profile-flag"
          title={country?.countryName ?? t(language, 'locationUnknown')}
        />
        <strong className="dashboard-profile-name">{profile.name}</strong>
        {active ? <span className="dashboard-active-chip">{t(language, 'profileActive')}</span> : null}
        <span
          className="dashboard-profile-meta"
          title={`${profile.networkType.toUpperCase()} / ${profile.securityType.toUpperCase()}`}
        >
          {profile.networkType.toUpperCase()}/{profile.securityType.toUpperCase()}
        </span>
      </div>

      <div className="dashboard-profile-actions">
        <span className={`latency-chip latency-${latencyTone}`}>{formatLatency(latency, language)}</span>
        <button
          type="button"
          className="dashboard-delete-icon"
          onClick={(event) => {
            event.stopPropagation();
            onDelete();
          }}
          aria-label={t(language, 'subscriptionDelete')}
          title={t(language, 'subscriptionDelete')}
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
}
