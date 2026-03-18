import { Check, Copy, Trash2 } from 'lucide-react';
import { Button } from '../../components/Button';
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
  onActivate,
  onDuplicate,
  onDelete,
}: ProfileListItemProps) {
  const language = useAppStore((state) => state.settings.language);
  const latencyTone = latency?.status ?? 'checking';

  return (
    <div
      role="button"
      tabIndex={0}
      className={`profile-list-item${selected ? ' profile-list-item-selected' : ''}`}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
        }
      }}
    >
      <div className="profile-item-main">
        <div className="profile-item-header profile-item-header-inline">
          <CountryFlag
            code={country?.countryCode}
            className="profile-flag"
            title={country?.countryName ?? t(language, 'locationUnknown')}
          />
          <strong>{profile.name}</strong>
          {active ? <span className="mini-chip">{t(language, 'profileActive')}</span> : null}
          <span className="profile-inline-meta" title={`${profile.networkType.toUpperCase()} / ${profile.securityType.toUpperCase()}`}>
            {profile.networkType.toUpperCase()} / {profile.securityType.toUpperCase()}
          </span>
          <span className={`latency-chip latency-${latencyTone}`}>{formatLatency(latency, language)}</span>
        </div>
      </div>
      <div className="profile-item-actions">
        <Button
          variant="ghost"
          aria-label={t(language, 'profileUse')}
          title={t(language, 'profileUse')}
          onClick={(event) => {
            event.stopPropagation();
            onActivate();
          }}
        >
          <Check size={14} />
        </Button>
        <Button
          variant="ghost"
          onClick={(event) => {
            event.stopPropagation();
            onDuplicate();
          }}
        >
          <Copy size={14} />
        </Button>
        <Button
          variant="ghost"
          className="danger-ghost"
          onClick={(event) => {
            event.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 size={14} />
        </Button>
      </div>
    </div>
  );
}
