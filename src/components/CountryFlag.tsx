import {
  countryFlagEmojiUrl,
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
  const src = countryFlagEmojiUrl(normalized);

  return (
    <span className={className} title={title}>
      {src ? (
        <img
          src={src}
          alt={normalized ?? 'flag'}
          className="profile-flag-image"
          draggable={false}
          referrerPolicy="no-referrer"
        />
      ) : (
        countryFlagFallback()
      )}
    </span>
  );
}
