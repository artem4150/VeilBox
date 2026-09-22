export function normalizeCountryCode(code?: string | null) {
  const normalized = code?.trim().toUpperCase();
  return normalized && normalized.length === 2 ? normalized : null;
}

export function countryFlagAssetUrl(code?: string | null) {
  const normalized = normalizeCountryCode(code);
  if (!normalized) {
    return null;
  }

  return `${import.meta.env.BASE_URL}flags/${normalized.toLowerCase()}.svg`;
}

export function countryFlagFallback() {
  return String.fromCodePoint(0x1f310);
}
