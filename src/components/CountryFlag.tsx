import { useState } from 'react';
import {
  countryFlagAssetUrl,
  countryFlagFallback,
  normalizeCountryCode,
} from '../lib/country';

interface CountryFlagProps {
  code?: string | null;
  title?: string;
  className?: string;
}

export function CountryFlag({ code, title, className = 'profile-flag' }: CountryFlagProps) {
  const normalized = normalizeCountryCode(code);
  const src = countryFlagAssetUrl(normalized);
  const [failedSource, setFailedSource] = useState<string | null>(null);

  return (
    <span className={className} title={title}>
      {src && failedSource !== src ? (
        <img
          src={src}
          alt={normalized ?? 'flag'}
          className="profile-flag-image"
          draggable={false}
          onError={() => setFailedSource(src)}
        />
      ) : (
        normalized ?? countryFlagFallback()
      )}
    </span>
  );
}
