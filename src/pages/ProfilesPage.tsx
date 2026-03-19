import { useMemo, useState, useEffect } from 'react';
import { Button } from '../components/Button';
import { EmptyState } from '../components/EmptyState';
import { Panel } from '../components/Panel';
import { ImportProfileDialog } from '../features/profiles/ImportProfileDialog';
import { ProfileForm } from '../features/profiles/ProfileForm';
import { ProfileListItem } from '../features/profiles/ProfileListItem';
import { useAppStore } from '../store/useAppStore';

import { useLocation, useNavigate } from 'react-router-dom';

export function ProfilesPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const profiles = useAppStore((state) => state.profiles);
  const latencies = useAppStore((state) => state.latencies);
  const selectedProfileId = useAppStore((state) => state.selectedProfileId);
  const saveProfile = useAppStore((state) => state.saveProfile);
  const importProfile = useAppStore((state) => state.importProfile);
  const deleteProfile = useAppStore((state) => state.deleteProfile);
  const duplicateProfile = useAppStore((state) => state.duplicateProfile);
  const selectProfile = useAppStore((state) => state.selectProfile);
  const [editorProfileId, setEditorProfileId] = useState<string | null>(selectedProfileId);

  useEffect(() => {
    if (new URLSearchParams(location.search).get('new') === '1') {
      setEditorProfileId(null);
      navigate('/profiles', { replace: true });
    }
  }, [location.search, navigate]);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === editorProfileId) ?? null,
    [editorProfileId, profiles],
  );

  const sortedProfiles = useMemo(() => {
    return [...profiles].sort((a, b) => {
      if (a.id === selectedProfileId) return -1;
      if (b.id === selectedProfileId) return 1;
      return 0;
    });
  }, [profiles, selectedProfileId]);

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <span className="eyebrow">Profiles</span>
          <h1>Profile library</h1>
          <p>Manual entry and strict `vless://` import with validation.</p>
        </div>
      </div>

      <div className="profiles-layout">
        <Panel title="Stored profiles" description="Select an active route or manage existing ones">
          {sortedProfiles.length ? (
            <div className="profile-list">
                {sortedProfiles.map((profile) => (
                  <ProfileListItem
                    key={profile.id}
                    profile={profile}
                    selected={editorProfileId === profile.id}
                    active={selectedProfileId === profile.id}
                    latency={latencies[profile.id]}
                    onSelect={() => setEditorProfileId(profile.id)}
                    onActivate={() => void selectProfile(profile.id)}
                    onDuplicate={() => void duplicateProfile(profile.id)}
                    onDelete={() => void deleteProfile(profile.id)}
                  />
                ))}
            </div>
          ) : (
            <EmptyState
              title="No profiles saved"
              message="Import a VLESS URI or create a profile manually."
            />
          )}
        </Panel>

        <div className="profiles-right">
          <Panel title="Import" description="Parse a single VLESS URI into a normalized profile">
            <ImportProfileDialog onImport={importProfile} />
          </Panel>

          <Panel
            title={selectedProfile ? 'Edit profile' : 'Create profile'}
            description="The builder validates the same supported modes used by the backend."
          >
            <ProfileForm
              profile={selectedProfile}
              onSave={saveProfile}
              onCancel={() => setEditorProfileId(null)}
            />
          </Panel>
        </div>
      </div>
    </div>
  );
}
