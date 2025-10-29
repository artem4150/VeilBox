import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Connect,
  Disconnect,
  EnableSystemProxy,
  DisableSystemProxy,
  TailLogs
} from "../wailsjs/go/main/App";
import { EventsOn } from "../wailsjs/runtime/runtime";
import { main } from "../wailsjs/go/models";

import "./App.css";
import logoMarkUrl from './assets/veilbox-mark.svg';   // без текста (для свернутого)
import logoFullUrl from './assets/veilbox-full.svg';   // полный логотип (для развернутого)
type NavKey = "dashboard" | "logs" | "settings";

type ProfileInfo = {
  nodeName: string;
  host: string;
  port: string;
  transport: string;
  flow: string;
  security: string;
  fingerprint: string;
  sni: string;
  shortId: string;
  country: string;
};

type StoredProfile = {
  id: string;
  label: string;
  uri: string;
  info: ProfileInfo;
};

const NAV_ITEMS: { key: NavKey; label: string }[] = [
  { key: "dashboard", label: "Dashboard" },
  { key: "logs", label: "Logs" },
  { key: "settings", label: "Settings" }
];

const PROFILES_KEY = "veilbox.profiles";
const SELECTED_KEY = "veilbox.selectedProfile";
const CHART_WIDTH = 320;
const CHART_HEIGHT = 80;
const MAX_THROUGHPUT_SAMPLES = 180;
const SPLIT_ENABLED_KEY = "veilbox.splitEnabled";
const SPLIT_FORM_KEY = "veilbox.splitForm";

type SplitTunnelFormState = {
  bypassDomains: string;
  bypassIPs: string;
  bypassApps: string;
  proxyDomains: string;
  proxyIPs: string;
  proxyApps: string;
  blockDomains: string;
  blockIPs: string;
  blockApps: string;
};

type DNSFormRow = {
  id: string;
  tag: string;
  type: string;
  address: string;
  detour: string;
  strategy: string;
};

type DNSFormState = {
  strategy: string;
  servers: DNSFormRow[];
};

type RegionFormState = {
  proxyCountries: string;
  directCountries: string;
  blockCountries: string;
};

type MetricsFormState = {
  enableObservatory: boolean;
  observatoryListen: string;
  observatoryToken: string;
};

type ToastTone = "info" | "error";

type ToastMessage = {
  id: string;
  message: string;
  tone: ToastTone;
};

type ThroughputSample = {
  ts: number;
  down: number;
  up: number;
};

type ConnectionState = "idle" | "connecting" | "connected" | "error";

function createId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return Math.random().toString(36).slice(2, 10);
}



function formatDuration(ms: number): string {
  if (ms <= 0) {
    return "00:00:00";
  }
  const totalSeconds = Math.floor(ms / 1000);
  const hours = String(Math.floor(totalSeconds / 3600)).padStart(2, "0");
  const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, "0");
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
}

function guessCountry(name: string, host: string): string {
  const patterns = [name, host];
  for (const value of patterns) {
    const match = /(?:^|[-_])([a-z]{2})(?:[-_]|$)/i.exec(value);
    if (match) {
      const code = match[1].toUpperCase();
      try {
        const formatter = new Intl.DisplayNames(["en"], { type: "region" });
        return formatter.of(code) ?? code;
      } catch {
        return code;
      }
    }
  }
  return "-";
}

function parseVless(uri: string): ProfileInfo | null {
  if (!uri.trim()) {
    return null;
  }
  try {
    const normalized = uri.replace(/^vless:\/\//i, "https://");
    const target = new URL(normalized);
    const nodeName =
      decodeURIComponent(target.hash.replace(/^#/, "")) ||
      target.hostname ||
      "Unnamed";
    const host = target.hostname || "-";
    const port = target.port || "443";
    const search = target.searchParams;
    const transport = (search.get("type") || "grpc").toUpperCase();
    const flow = search.get("flow") || "-";
    const security = (search.get("security") || "reality").toUpperCase();
    const fingerprint = (search.get("fp") || "chrome").toUpperCase();
    const sni = search.get("sni") || host;
    const shortId = search.get("sid") || "-";
    const country = guessCountry(nodeName, host);
    return {
      nodeName,
      host,
      port,
      transport,
      flow,
      security,
      fingerprint,
      sni,
      shortId,
      country
    };
  } catch {
    return null;
  }
}

function parseInputList(raw: string): string[] {
  return raw
    .split(/[\n,;]/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function normalizeSplitForm(raw: unknown): SplitTunnelFormState | null {
  if (!raw || typeof raw !== "object") {
    return null;
  }
  const source = raw as Partial<Record<keyof SplitTunnelFormState, unknown>>;
  return {
    bypassDomains: typeof source.bypassDomains === "string" ? source.bypassDomains : "",
    bypassIPs: typeof source.bypassIPs === "string" ? source.bypassIPs : "",
    bypassApps: typeof source.bypassApps === "string" ? source.bypassApps : "",
    proxyDomains: typeof source.proxyDomains === "string" ? source.proxyDomains : "",
    proxyIPs: typeof source.proxyIPs === "string" ? source.proxyIPs : "",
    proxyApps: typeof source.proxyApps === "string" ? source.proxyApps : "",
    blockDomains: typeof source.blockDomains === "string" ? source.blockDomains : "",
    blockIPs: typeof source.blockIPs === "string" ? source.blockIPs : "",
    blockApps: typeof source.blockApps === "string" ? source.blockApps : ""
  };
}

function formatRate(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B/s";
  }
  const units = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

function deriveDnsTag(address: string): string {
  let clean = address.trim();
  if (!clean) {
    return "dns";
  }
  clean = clean.replace(/^https?:\/\//i, "");
  clean = clean.replace(/^tls:\/\//i, "");
  clean = clean.replace(/^tcp:\/\//i, "");
  clean = clean.replace(/^udp:\/\//i, "");
  const slashIndex = clean.indexOf("/");
  if (slashIndex > 0) {
    clean = clean.slice(0, slashIndex);
  }
  return clean || "dns";
}

function buildSparklinePath(
  samples: ThroughputSample[],
  accessor: (sample: ThroughputSample) => number,
  width: number,
  height: number
): string {
  if (samples.length === 0) {
    return "";
  }
  const minTs = samples[0].ts;
  const maxTs = samples[samples.length - 1].ts || minTs + 1;
  const maxValue = Math.max(...samples.map(accessor), 1);
  const rangeTs = Math.max(maxTs - minTs, 1);

  return samples
    .map((sample, index) => {
      const x = ((sample.ts - minTs) / rangeTs) * width;
      const yValue = accessor(sample);
      const y = height - (height * yValue) / maxValue;
      const command = index === 0 ? "M" : "L";
      return `${command}${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function PowerIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M12 2v8m5.657-3.657a8 8 0 11-11.314 0"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <rect x="7" y="7" width="10" height="10" rx="2" fill="currentColor" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M19.43 12.98l1.34-2.33-1.45-2.51-2.69.33-1.07-2.45h-2.73L10.76 8.5 8.07 8.14 6.62 10.65l1.34 2.33-1.34 2.33 1.45 2.51 2.69-.33 1.07 2.45h2.73l1.07-2.45 2.69.33 1.45-2.51-1.34-2.33zM12 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7z"
        fill="currentColor"
      />
    </svg>
  );
}

function RefreshIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M17.65 6.35A7.95 7.95 0 0 0 12 4V1L7 6l5 5V7c2.76 0 5 2.24 5 5a5 5 0 0 1-8.9 3H6.26A7 7 0 0 0 19 12a7 7 0 0 0-1.35-5.65z"
        fill="currentColor"
      />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M12 5a1 1 0 0 1 1 1v5h5a1 1 0 1 1 0 2h-5v5a1 1 0 1 1-2 0v-5H6a1 1 0 1 1 0-2h5V6a1 1 0 0 1 1-1z"
        fill="currentColor"
      />
    </svg>
  );
}

function ShieldIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M12 2l8 4v5c0 5.25-3.33 9.92-8 11-4.67-1.08-8-5.75-8-11V6l8-4z" fill="currentColor" />
    </svg>
  );
}

export default function App(): JSX.Element {
  const [view, setView] = useState<NavKey>("dashboard");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [profiles, setProfiles] = useState<StoredProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [showProfileModal, setShowProfileModal] = useState(false);
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [formLabel, setFormLabel] = useState("");
  const [formUri, setFormUri] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [previewCountry, setPreviewCountry] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [connectedAt, setConnectedAt] = useState<number | null>(null);
  const [elapsed, setElapsed] = useState("00:00:00");
  const [pingMs, setPingMs] = useState<number | null>(null);
  const [connectionState, setConnectionState] = useState<ConnectionState>("idle");
  const [connectionMessage, setConnectionMessage] = useState(
    "Choose a profile and connect to enable secure browsing."
  );
  const [logs, setLogs] = useState<string[]>([]);
  const [publicIp, setPublicIp] = useState("-");
  const [publicLocation, setPublicLocation] = useState("-");
  const [splitEnabled, setSplitEnabled] = useState(true);
  const [splitForm, setSplitForm] = useState<SplitTunnelFormState>({
    bypassDomains: "",
    bypassIPs: "",
    bypassApps: "",
    proxyDomains: "",
    proxyIPs: "",
    proxyApps: "",
    blockDomains: "",
    blockIPs: "",
    blockApps: ""
  });
  const [dnsForm, setDnsForm] = useState<DNSFormState>({
    strategy: "prefer_ipv4",
    servers: []
  });
  const [regionForm, setRegionForm] = useState<RegionFormState>({
    proxyCountries: "",
    directCountries: "",
    blockCountries: ""
  });
  const [metricsForm, setMetricsForm] = useState<MetricsFormState>({
    enableObservatory: false,
    observatoryListen: "127.0.0.1:9090",
    observatoryToken: ""
  });
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [throughput, setThroughput] = useState<ThroughputSample[]>([]);

  const countryCacheRef = useRef<Record<string, string>>({});
  const toastTimersRef = useRef<Map<string, number>>(new Map());

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
    const timer = toastTimersRef.current.get(id);
    if (timer) {
      window.clearTimeout(timer);
      toastTimersRef.current.delete(id);
    }
  }, []);

  const pushToast = useCallback((message: string, tone: ToastTone = "info") => {
    const id = createId();
    setToasts((prev) => [...prev, { id, message, tone }]);
    const timeout = window.setTimeout(() => {
      setToasts((prev) => prev.filter((toast) => toast.id !== id));
      toastTimersRef.current.delete(id);
    }, tone === "error" ? 6000 : 4000);
    toastTimersRef.current.set(id, timeout);
  }, []);

  const updateConnectionState = useCallback(
    (state: ConnectionState, message?: string) => {
      setConnectionState(state);
      if (message) {
        setConnectionMessage(message);
        return;
      }
      switch (state) {
        case "connected":
          setConnectionMessage("Connection is active.");
          break;
        case "connecting":
          setConnectionMessage("Establishing a secure tunnel...");
          break;
        case "error":
          setConnectionMessage("Connection error. Check the profile and try again.");
          break;
        default:
          setConnectionMessage("Ready to connect.");
          break;
      }
    },
    []
  );

  const resolveCountry = useCallback(async (host: string): Promise<string> => {
    const key = host?.trim().toLowerCase();
    if (!key) {
      return "-";
    }
    if (countryCacheRef.current[key]) {
      return countryCacheRef.current[key];
    }
    try {
      const response = await fetch(`https://ipapi.co/${encodeURIComponent(host)}/json/`);
      if (!response.ok) {
        throw new Error("country lookup failed");
      }
      const data = await response.json();
      const resolved: string =
        data.country_name || data.country || data.country_code || "-";
      const value = resolved || "-";
      countryCacheRef.current[key] = value;
      return value;
    } catch {
      return "-";
    }
  }, []);

  const refreshPublicIp = useCallback(async () => {
    try {
      const response = await fetch("https://ipapi.co/json/");
      if (!response.ok) {
        throw new Error("ip lookup failed");
      }
      const data = await response.json();
      setPublicIp(data.ip ?? "-");
      setPublicLocation(data.country_name ?? data.country ?? "-");
    } catch {
      setPublicIp("-");
      setPublicLocation("-");
    }
  }, []);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === selectedProfileId) ?? null,
    [profiles, selectedProfileId]
  );

  const parsedPreview = useMemo(() => parseVless(formUri.trim()), [formUri]);

  useEffect(() => {
    refreshPublicIp();
  }, [refreshPublicIp]);

  useEffect(() => {
    return () => {
      toastTimersRef.current.forEach((timeout) => window.clearTimeout(timeout));
      toastTimersRef.current.clear();
    };
  }, []);

  useEffect(() => {
    if (!parsedPreview) {
      setPreviewCountry(null);
      return;
    }
    const existingCountry =
      editingProfileId &&
      profiles.find((profile) => profile.id === editingProfileId)?.info.country;
    const initialCountry =
      (existingCountry && existingCountry !== "-"
        ? existingCountry
        : parsedPreview.country) || "-";
    setPreviewCountry(initialCountry);
    let cancelled = false;
    (async () => {
      const country = await resolveCountry(parsedPreview.host);
      if (!cancelled) {
        setPreviewCountry(country || "-");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [parsedPreview, resolveCountry, editingProfileId, profiles]);

  useEffect(() => {
    try {
      const rawProfiles = localStorage.getItem(PROFILES_KEY);
      if (rawProfiles) {
        const stored: StoredProfile[] = JSON.parse(rawProfiles);
        setProfiles(stored);
      }
      const rawSelected = localStorage.getItem(SELECTED_KEY);
      if (rawSelected) {
        setSelectedProfileId(rawSelected);
      }
      const rawSplit = localStorage.getItem(SPLIT_ENABLED_KEY);
      if (rawSplit !== null) {
        setSplitEnabled(rawSplit !== "false");
      }
      const rawSplitForm = localStorage.getItem(SPLIT_FORM_KEY);
      if (rawSplitForm) {
        let stored: unknown;
        try {
          stored = JSON.parse(rawSplitForm);
        } catch {
          stored = null;
        }
        const normalized = normalizeSplitForm(stored);
        if (normalized) {
          setSplitForm(normalized);
        }
      }
    } catch {
      // ignore persistence errors
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
    } catch {
      // ignore persistence errors
    }
  }, [profiles]);

  useEffect(() => {
    try {
      localStorage.setItem(SPLIT_ENABLED_KEY, String(splitEnabled));
    } catch {
      // ignore persistence errors
    }
  }, [splitEnabled]);

  useEffect(() => {
    try {
      localStorage.setItem(SPLIT_FORM_KEY, JSON.stringify(splitForm));
    } catch {
      // ignore persistence errors
    }
  }, [splitForm]);

  useEffect(() => {
    if (selectedProfileId) {
      try {
        localStorage.setItem(SELECTED_KEY, selectedProfileId);
      } catch {
        // ignore persistence errors
      }
    }
  }, [selectedProfileId]);

  useEffect(() => {
    const missing = profiles.filter(
      (profile) => !profile.info.country || profile.info.country === "-"
    );
    if (!missing.length) {
      return;
    }
    let cancelled = false;
    (async () => {
      const updates: Record<string, string> = {};
      await Promise.all(
        missing.map(async (profile) => {
          const country = await resolveCountry(profile.info.host);
          if (country && country !== profile.info.country && country !== "-") {
            updates[profile.id] = country;
          }
        })
      );
      if (cancelled || !Object.keys(updates).length) {
        return;
      }
      setProfiles((prev) =>
        prev.map((profile) =>
          updates[profile.id]
            ? { ...profile, info: { ...profile.info, country: updates[profile.id] } }
            : profile
        )
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [profiles, resolveCountry]);

  useEffect(() => {
    const timer = setInterval(async () => {
      try {
        const nextLogs = await TailLogs(200);
        setLogs(nextLogs);
      } catch {
        // ignore tail errors
      }
    }, 700);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!connected || !connectedAt) {
      setElapsed("00:00:00");
      return;
    }
    const update = () => setElapsed(formatDuration(Date.now() - connectedAt));
    update();
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, [connected, connectedAt]);

  useEffect(() => {
    if (!showProfileModal) {
      return undefined;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setShowProfileModal(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [showProfileModal]);

  const addDnsRow = () => {
    setDnsForm((prev) => ({
      ...prev,
      servers: [
        ...prev.servers,
        { id: createId(), tag: "", type: "https", address: "", detour: "", strategy: "" }
      ]
    }));
  };

  const updateDnsRow = (id: string, field: keyof DNSFormRow, value: string) => {
    setDnsForm((prev) => ({
      ...prev,
      servers: prev.servers.map((row) => (row.id === id ? { ...row, [field]: value } : row))
    }));
  };

  const removeDnsRow = (id: string) => {
    setDnsForm((prev) => ({
      ...prev,
      servers: prev.servers.filter((row) => row.id !== id)
    }));
  };

  const toggleSidebar = () => {
    setSidebarCollapsed((prev) => !prev);
  };

  const openProfileModal = (profile?: StoredProfile | null) => {
    if (profile) {
      setEditingProfileId(profile.id);
      setFormLabel(profile.label);
      setFormUri(profile.uri);
      setPreviewCountry(profile.info.country || null);
    } else {
      setEditingProfileId(null);
      setFormLabel("");
      setFormUri("");
      setPreviewCountry(null);
    }
    setFormError(null);
    setShowProfileModal(true);
  };

  const resetModal = () => {
    setEditingProfileId(null);
    setFormLabel("");
    setFormUri("");
    setPreviewCountry(null);
    setFormError(null);
    setShowProfileModal(false);
  };

  const buildSplitTunnelPayload = () => {
    if (!splitEnabled) {
      return undefined;
    }
    const payload = {
      bypassDomains: parseInputList(splitForm.bypassDomains),
      bypassIPs: parseInputList(splitForm.bypassIPs),
      bypassProcesses: parseInputList(splitForm.bypassApps),
      proxyDomains: parseInputList(splitForm.proxyDomains),
      proxyIPs: parseInputList(splitForm.proxyIPs),
      proxyProcesses: parseInputList(splitForm.proxyApps),
      blockDomains: parseInputList(splitForm.blockDomains),
      blockIPs: parseInputList(splitForm.blockIPs),
      blockProcesses: parseInputList(splitForm.blockApps)
    };
    const filtered: Record<string, string[]> = {};
    Object.entries(payload).forEach(([key, list]) => {
      if (list.length) {
        filtered[key] = list;
      }
    });
    const total = Object.values(filtered).reduce((sum, list) => sum + list.length, 0);
    return total > 0 ? filtered : undefined;
  };

  const buildDnsSettingsPayload = () => {
    const normalized = dnsForm.servers
      .map((row) => {
        const address = row.address.trim();
        if (!address) {
          return null;
        }
        const typeHint = row.type.trim().toLowerCase();
        const type =
          typeHint ||
          (address === "local"
            ? "local"
            : address.startsWith("https://")
            ? "https"
            : address.startsWith("tls://")
            ? "tls"
            : address.startsWith("tcp://")
            ? "tcp"
            : address.startsWith("udp://")
            ? "udp"
            : "udp");
        const entry: Record<string, string> = {
          tag: row.tag.trim() || deriveDnsTag(address),
          type,
          address
        };
        if (row.detour.trim()) {
          entry.detour = row.detour.trim();
        }
        if (row.strategy.trim()) {
          entry.strategy = row.strategy.trim();
        }
        return entry;
      })
      .filter((entry): entry is Record<string, string> => Boolean(entry));
    const strategy = dnsForm.strategy.trim();
    if (!normalized.length && !strategy) {
      return undefined;
    }
    const payload: Record<string, unknown> = {
      strategy: strategy || "prefer_ipv4"
    };
    if (normalized.length) {
      payload.servers = normalized;
    }
    return payload;
  };

  const buildRegionRoutingPayload = () => {
    const proxy = parseInputList(regionForm.proxyCountries).map((entry) => entry.toUpperCase());
    const direct = parseInputList(regionForm.directCountries).map((entry) => entry.toUpperCase());
    const block = parseInputList(regionForm.blockCountries).map((entry) => entry.toUpperCase());
    if (!proxy.length && !direct.length && !block.length) {
      return undefined;
    }
    return {
      proxyCountries: proxy,
      directCountries: direct,
      blockCountries: block
    };
  };

  const buildMetricsPayload = () => {
    if (!metricsForm.enableObservatory) {
      return undefined;
    }
    const listen = metricsForm.observatoryListen.trim() || "127.0.0.1:9090";
    const payload: Record<string, string | boolean> = {
      enableObservatory: true,
      observatoryListen: listen
    };
    if (metricsForm.observatoryToken.trim()) {
      payload.observatoryToken = metricsForm.observatoryToken.trim();
    }
    return payload;
  };

  useEffect(() => {
    const unsubscribers: Array<() => void> = [];
    unsubscribers.push(
      EventsOn("tray:notification", (message: unknown) => {
        if (message != null) {
          pushToast(String(message));
        }
      })
    );
    unsubscribers.push(
      EventsOn("tray:error", (message: unknown) => {
        if (message != null) {
          pushToast(String(message), "error");
        }
      })
    );
    unsubscribers.push(
      EventsOn("tray:state", (state: unknown) => {
        const value = String(state);
        if (value === "connected") {
          if (!connected) {
            pushToast("Connection is active");
          }
          setConnected(true);
          setConnectedAt(Date.now());
          updateConnectionState("connected", "Connection restored from tray.");
        } else if (value === "disconnected") {
          if (connected) {
            pushToast("Connection stopped");
          }
          setConnected(false);
          setConnectedAt(null);
          setPingMs(null);
          updateConnectionState("idle", "Disconnected.");
        }
      })
    );
    unsubscribers.push(
      EventsOn("tray:requestProfile", () => {
        openProfileModal();
        pushToast("Choose a profile to connect");
      })
    );
    unsubscribers.push(
      EventsOn("core:throughput", (payload: any) => {
        const down =
          Number(
            payload?.down ??
              payload?.downstream ??
              payload?.downBytesPerSec ??
              payload?.down_bytes ??
              payload?.down_bps ??
              payload?.downrate
          ) || 0;
        const up =
          Number(
            payload?.up ??
              payload?.upstream ??
              payload?.upBytesPerSec ??
              payload?.up_bytes ??
              payload?.up_bps ??
              payload?.uprate
          ) || 0;
        const sample: ThroughputSample = {
          ts: Date.now(),
          down: Math.max(0, down),
          up: Math.max(0, up)
        };
        setThroughput((prev) => {
          const next = [...prev, sample];
          return next.slice(-MAX_THROUGHPUT_SAMPLES);
        });
      })
    );
    return () => {
      unsubscribers.forEach((off) => {
        try {
          off();
        } catch {
          // ignore unsubscribe errors
        }
      });
    };
  }, [connected, openProfileModal, pushToast, updateConnectionState]);

  useEffect(() => {
    if (!connected) {
      setThroughput([]);
    }
  }, [connected]);

  const handleSaveProfile = async () => {
    const parsed = parseVless(formUri.trim());
    if (!parsed) {
      setFormError("Enter a valid VLESS URI.");
      return;
    }
    const label = formLabel.trim() || parsed.nodeName;
    const country = await resolveCountry(parsed.host);
    const info: ProfileInfo = { ...parsed, country: country || parsed.country || "-" };

    if (editingProfileId) {
      setProfiles((prev) =>
        prev.map((profile) =>
          profile.id === editingProfileId ? { ...profile, label, uri: formUri.trim(), info } : profile
        )
      );
      setSelectedProfileId(editingProfileId);
      pushToast("Profile updated");
    } else {
      const newProfile: StoredProfile = {
        id: createId(),
        label,
        uri: formUri.trim(),
        info
      };
      setProfiles((prev) => [...prev, newProfile]);
      setSelectedProfileId(newProfile.id);
      pushToast("Profile added");
    }
    resetModal();
  };

  const handleSelectProfile = (id: string) => {
    setSelectedProfileId(id);
    const nextProfile = profiles.find((profile) => profile.id === id);
    if (connected && nextProfile) {
      measurePing(nextProfile.info.sni || nextProfile.info.host);
    } else {
      setPingMs(null);
    }
  };

  const handleDeleteProfile = (id: string) => {
    const profile = profiles.find((item) => item.id === id);
    if (!profile) {
      return;
    }
    const confirmed = window.confirm(`Delete profile "${profile.label}"?`);
    if (!confirmed) {
      return;
    }
    const remaining = profiles.filter((item) => item.id !== id);
    setProfiles(remaining);
    if (selectedProfileId === id && connected) {
      void disconnectNow();
    }
    if (selectedProfileId === id) {
      setSelectedProfileId(remaining.length ? remaining[0].id : null);
    }
    if (editingProfileId === id) {
      resetModal();
    }
    pushToast("Profile removed");
  };

  const measurePing = async (target: string) => {
    if (!target) {
      setPingMs(null);
      return;
    }
    const start = performance.now();
    try {
      await fetch(`https://${target}`, { method: "GET", mode: "no-cors", cache: "no-store" });
    } catch {
      // ignore network errors
    }
    const end = performance.now();
    const duration = Math.round(end - start);
    if (Number.isFinite(duration)) {
      setPingMs(duration);
    }
  };

  const connectNow = async () => {
    if (!selectedProfile) {
      openProfileModal();
      updateConnectionState("idle", "Select a profile to connect.");
      return;
    }
    const basePayload: Record<string, unknown> = {
      VLESSURI: selectedProfile.uri,
      Mode: "proxy"
    };
    const splitPayload = buildSplitTunnelPayload();
    if (splitPayload) {
      basePayload.SplitTunnel = splitPayload;
    }
    const dnsPayload = buildDnsSettingsPayload();
    if (dnsPayload) {
      basePayload.DNS = dnsPayload;
    }
    const regionPayload = buildRegionRoutingPayload();
    if (regionPayload) {
      basePayload.RegionRouting = regionPayload;
    }
    const metricsPayload = buildMetricsPayload();
    if (metricsPayload) {
      basePayload.Metrics = metricsPayload;
    }

    try {
      updateConnectionState("connecting", `Connecting to ${selectedProfile.label}...`);
      const connectionPayload = main.ConnectRequest.createFrom(basePayload);
      await Connect(connectionPayload);
      await EnableSystemProxy();
      setConnected(true);
      setConnectedAt(Date.now());
      setPingMs(null);
      const target = selectedProfile.info.sni || selectedProfile.info.host;
      measurePing(target);
      setTimeout(() => {
        void refreshPublicIp();
      }, 1000);
      updateConnectionState("connected", `Connected to ${selectedProfile.label}.`);
      pushToast(`Connected to ${selectedProfile.label}`);
    } catch (error: any) {
      setConnected(false);
      setConnectedAt(null);
      setPingMs(null);
      const message = error instanceof Error ? error.message : String(error);
      updateConnectionState("error", message);
      pushToast(`Connect failed: ${message}`, "error");
    }
  };

  const disconnectNow = async () => {
    let hadError = false;
    try {
      await Disconnect();
    } catch (error: any) {
      const message = error instanceof Error ? error.message : String(error);
      hadError = true;
      updateConnectionState("error", message);
      pushToast(`Disconnect failed: ${message}`, "error");
    } finally {
      try {
        await DisableSystemProxy();
      } catch (error: any) {
        const message = error instanceof Error ? error.message : String(error);
        hadError = true;
        updateConnectionState("error", message);
        pushToast(`Disable proxy failed: ${message}`, "error");
      }
      setConnected(false);
      setConnectedAt(null);
      setPingMs(null);
      await refreshPublicIp();
      if (!hadError) {
        updateConnectionState("idle", "Disconnected. Ready to connect.");
        pushToast("Disconnected");
      }
    }
  };

  const handleSplitToggle = (enabled: boolean) => {
    if (enabled === splitEnabled) {
      return;
    }
    setSplitEnabled(enabled);
    pushToast(enabled ? "Split tunneling enabled" : "Split tunneling disabled");
    if (enabled) {
      pushToast("Reconnect to apply split tunneling changes", "info");
    } else if (connected) {
      pushToast("Reconnect to fully disable split tunneling rules", "info");
    }
  };

  const splitSummary = useMemo(() => {
    const bypassHosts =
      parseInputList(splitForm.bypassDomains).length + parseInputList(splitForm.bypassIPs).length;
    const proxyHosts =
      parseInputList(splitForm.proxyDomains).length + parseInputList(splitForm.proxyIPs).length;
    const blockHosts =
      parseInputList(splitForm.blockDomains).length + parseInputList(splitForm.blockIPs).length;

    const bypassApps = parseInputList(splitForm.bypassApps).length;
    const proxyApps = parseInputList(splitForm.proxyApps).length;
    const blockApps = parseInputList(splitForm.blockApps).length;

    return {
      bypassHosts,
      bypassApps,
      proxyHosts,
      proxyApps,
      blockHosts,
      blockApps
    };
  }, [splitForm]);

  const dnsServerCount = useMemo(
    () => dnsForm.servers.filter((row) => row.address.trim()).length,
    [dnsForm]
  );

  const latestThroughput = useMemo(
    () => (throughput.length ? throughput[throughput.length - 1] : undefined),
    [throughput]
  );

  const throughputPathDown = useMemo(
    () => buildSparklinePath(throughput, (sample) => sample.down, CHART_WIDTH, CHART_HEIGHT),
    [throughput]
  );
  const throughputPathUp = useMemo(
    () => buildSparklinePath(throughput, (sample) => sample.up, CHART_WIDTH, CHART_HEIGHT),
    [throughput]
  );

  const connectionSettings = useMemo(() => {
    const statusLabel =
      connectionState === "connected"
        ? "Connected"
        : connectionState === "connecting"
        ? "Connecting"
        : connectionState === "error"
        ? "Error"
        : "Disconnected";
    const base = [
      { label: "Status", value: `${statusLabel}${connectionMessage ? ` — ${connectionMessage}` : ""}` },
      { label: "Mode", value: "System Proxy" },
      { label: "Public IP", value: publicIp },
      { label: "Location", value: publicLocation },
      {
        label: "Split rules",
        value: splitEnabled ? `Proxy ${splitSummary.proxyHosts} hosts / ${splitSummary.proxyApps} apps | Direct ${splitSummary.bypassHosts} hosts / ${splitSummary.bypassApps} apps | Block ${splitSummary.blockHosts} hosts / ${splitSummary.blockApps} apps` : "Disabled"
      },
      {
        label: "DNS strategy",
        value: dnsForm.strategy || "prefer_ipv4"
      },
      {
        label: "DNS servers",
        value: dnsServerCount ? String(dnsServerCount) : "0"
      },
      {
        label: "Observatory",
        value: metricsForm.enableObservatory ? metricsForm.observatoryListen : "Disabled"
      }
    ];
    if (!selectedProfile) {
      return [
        ...base,
        { label: "Transport", value: "-" },
        { label: "Security", value: "-" },
        { label: "Fingerprint", value: "-" },
        { label: "Flow", value: "-" },
        { label: "Server Country", value: "-" },
        { label: "SNI", value: "-" },
        { label: "Short ID", value: "-" }
      ];
    }
    return [
      ...base,
      { label: "Transport", value: selectedProfile.info.transport || "-" },
      { label: "Security", value: selectedProfile.info.security || "-" },
      { label: "Fingerprint", value: selectedProfile.info.fingerprint || "-" },
      { label: "Flow", value: selectedProfile.info.flow || "-" },
      { label: "Server Country", value: selectedProfile.info.country || "-" },
      { label: "SNI", value: selectedProfile.info.sni || "-" },
      { label: "Short ID", value: selectedProfile.info.shortId || "-" }
    ];
  }, [
    connectionMessage,
    connectionState,
    dnsForm,
    dnsServerCount,
    metricsForm,
    publicIp,
    publicLocation,
    selectedProfile,
    splitSummary
  ]);

  const serverCountry = selectedProfile?.info.country ?? "-";
  const serverHost = selectedProfile
    ? `${selectedProfile.info.host}:${selectedProfile.info.port}`
    : "-";

const renderDashboard = () => {
  const isConnected = connectionState === "connected";
  const isConnecting = connectionState === "connecting";
  const bannerClass = ["panel", "connection-banner"];
  if (isConnected) {
    bannerClass.push("connection-banner--online");
  } else if (connectionState === "error") {
    bannerClass.push("connection-banner--error");
  } else if (isConnecting) {
    bannerClass.push("connection-banner--progress");
  }
  const indicatorClass = [
    "connection-indicator",
    isConnected
      ? "connection-indicator--online"
      : connectionState === "error"
      ? "connection-indicator--error"
      : isConnecting
      ? "connection-indicator--progress"
      : ""
  ]
    .filter(Boolean)
    .join(" ");
  const showServerMeta = selectedProfile && (isConnected || isConnecting);

  return (
    <div className="dashboard">
      <section className={bannerClass.filter(Boolean).join(" ")}>
        <div className="connection-banner__status">
          <span className={indicatorClass} />
          <div>
            <p className="connection-banner__title">
              {connectionState === "connected"
                ? "Secure tunnel is active"
                : connectionState === "connecting"
                ? "Connecting..."
                : connectionState === "error"
                ? "Connection issue"
                : "Disconnected"}
            </p>
            <span className="connection-banner__subtitle">{connectionMessage}</span>
          </div>
        </div>
        <div className="connection-banner__meta">
          <div>
            <span className="meta-label">Server</span>
            <span className="meta-value">{showServerMeta ? serverHost : "-"}</span>
          </div>
          {/* <div>
            <span className="meta-label">Server Country</span>
            <span className="meta-value">{showServerMeta ? serverCountry : "-"}</span>
          </div> */}
          <div>
            <span className="meta-label">Current IP</span>
            <span className="meta-value">{publicIp}</span>
          </div>
          <div>
            <span className="meta-label">Location</span>
            <span className="meta-value">{publicLocation}</span>
          </div>
        </div>
        <div className="connection-banner__actions">
          {connected ? (
            <button className="btn btn--danger" onClick={disconnectNow}>
              <StopIcon />
              Disconnect
            </button>
          ) : (
            <button className="btn btn--primary" onClick={connectNow}>
              <PowerIcon />
              Connect
            </button>
          )}
        </div>
      </section>

      <section className="panel stats-panel">
        <header className="panel__header">
          <div>
            <h2>Connection details</h2>
          </div>
          <div className="panel__actions">
            <label className="toggle">
              <input
                type="checkbox"
                checked={splitEnabled}
                onChange={(event) => handleSplitToggle(event.target.checked)}
              />
              <span className="toggle__slider" />
              <span className="toggle__text">Split tunneling</span>
            </label>
            <button
              className="ghost-button"
              type="button"
              onClick={() => {
                if (selectedProfile) {
                  measurePing(selectedProfile.info.sni || selectedProfile.info.host);
                }
              }}
              disabled={!selectedProfile}
            >
              <RefreshIcon />
              Refresh ping
            </button>
          </div>
        </header>
        <div className="stats-grid">
          <article className="stats-card">
            <span className="stats-card__label">Current profile</span>
            <span className="stats-card__value">
              {selectedProfile ? selectedProfile.label : "Not selected"}
            </span>
          </article>
          <article className="stats-card">
            <span className="stats-card__label">Connection time</span>
            <span className="stats-card__value">{connected ? elapsed : "-"}</span>
          </article>
          <article className="stats-card">
            <span className="stats-card__label">Ping</span>
            <span className="stats-card__value">
              {connected ? (pingMs === null ? "Measuring..." : `${pingMs} ms`) : "-"}
            </span>
          </article>
          <article className="stats-card">
            <span className="stats-card__label">System proxy</span>
            <span className="stats-card__value">{connected ? "Enabled" : "Disabled"}</span>
          </article>
          {isConnected ? (
            <>
              {/* <article className="stats-card">
                <span className="stats-card__label">Download</span>
                <span className="stats-card__value">
                  {latestThroughput ? formatRate(latestThroughput.down) : "-"}
                </span>
              </article>
              <article className="stats-card">
                <span className="stats-card__label">Upload</span>
                <span className="stats-card__value">
                  {latestThroughput ? formatRate(latestThroughput.up) : "-"}
                </span>
              </article> */}
            </>
          ) : null}
        </div>
        {/* {isConnected ? (
          <div className="chart">
            <div className="chart__header">
              <span className="chart__title">Throughput</span>
              <div className="chart__summary">
                <span>&darr; {latestThroughput ? formatRate(latestThroughput.down) : "-"}</span>
                <span>&uarr; {latestThroughput ? formatRate(latestThroughput.up) : "-"}</span>
              </div>
            </div>
            <div className="chart__body">
              {throughput.length ? (
                <svg
                  className="chart__svg"
                  width={CHART_WIDTH}
                  height={CHART_HEIGHT}
                  viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
                  preserveAspectRatio="none"
                >
                  <path className="chart__line chart__line--down" d={throughputPathDown} />
                  <path className="chart__line chart__line--up" d={throughputPathUp} />
                </svg>
              ) : (
                <div className="chart__placeholder">
                  {metricsForm.enableObservatory
                    ? "Waiting for stats..."
                    : "Enable observatory metrics in Settings to see throughput."}
                </div>
              )}
            </div>
          </div>
        ) : null} */}
        <div className="settings-list">
          {connectionSettings.map((item) => (
            <div key={item.label} className="settings-list__item">
              <span className="settings-list__label">{item.label}</span>
              <span className="settings-list__value">{item.value || "-"}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="panel profile-panel">
        <header className="panel__header">
          <h2>Profiles</h2>
          <div className="panel__actions">
            <label className="toggle">
              <input
                type="checkbox"
                checked={splitEnabled}
                onChange={(event) => handleSplitToggle(event.target.checked)}
              />
              <span className="toggle__slider" />
              <span className="toggle__text">Split tunneling</span>
            </label>
            <button className="ghost-button" type="button" onClick={() => openProfileModal()}>
              <PlusIcon />
              Add profiles
            </button>
          </div>
        </header>
        {profiles.length === 0 ? (
          <div className="empty-state">
            <p>You have not added any profiles yet.</p>
            <button className="btn btn--primary" type="button" onClick={() => openProfileModal()}>
              Add your first profile
            </button>
          </div>
        ) : (
          <div className="profile-list">
            {profiles.map((profile) => {
              const active = profile.id === selectedProfileId;
              return (
                <button
                  key={profile.id}
                  type="button"
                  className={`profile-card ${active ? "profile-card--active" : ""}`}
                  onClick={() => handleSelectProfile(profile.id)}
                >
                  <div className="profile-card__header">
                    <h3>{profile.label}</h3>
                    <span className="profile-card__tag">{profile.info.transport}</span>
                  </div>
                  <p className="profile-card__subtitle">
                    {profile.info.host}:{profile.info.port}
                  </p>
                  <div className="profile-card__meta">
                    <span>{profile.info.country}</span>
                    <span>{profile.info.security}</span>
                  </div>
                  <div className="profile-card__actions">
                    <button
                      type="button"
                      className="chip-button"
                      onClick={(event) => {
                        event.stopPropagation();
                        openProfileModal(profile);
                      }}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="chip-button chip-button--danger"
                      onClick={(event) => {
                        event.stopPropagation();
                        handleDeleteProfile(profile.id);
                      }}
                    >
                      Delete
                    </button>
                  </div>
                  {active ? <span className="profile-card__status">Active</span> : null}
                </button>
              );
            })}
          </div>
        )}
      </section>
  </div>
);
};

const renderSettings = () => (
  <div className="settings-page">
    <section className="panel settings-panel">
      <header className="settings-panel__header">
        <div>
          <h2>Connection settings</h2>
          <p className="settings-panel__hint">
            Changes are saved automatically and apply the next time you connect.
          </p>
        </div>
      </header>
      <div className="settings-panel__body">
        <section className="settings-section">
          <div className="settings-section__header">
            <h3>Split tunneling</h3>
            <label className="toggle">
              <input
                type="checkbox"
                checked={splitEnabled}
                onChange={(event) => handleSplitToggle(event.target.checked)}
              />
              <span className="toggle__slider" />
              <span className="toggle__text">{splitEnabled ? "Enabled" : "Disabled"}</span>
            </label>
          </div>
          <p className="settings-section__hint">
            Configure which destinations should use the tunnel or bypass it. Rules stay saved even if the
            toggle is off.
          </p>
          <div className="settings-grid settings-grid--two">
            <label className="field settings-field">
              <span className="field__label">Proxy domains</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={splitForm.proxyDomains}
                onChange={(event) =>
                  setSplitForm((prev) => ({ ...prev, proxyDomains: event.target.value }))
                }
                placeholder="example.com, api.example.com"
              />
        </label>
        <label className="field settings-field">
          <span className="field__label">Proxy IPs / CIDRs</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.proxyIPs}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, proxyIPs: event.target.value }))
            }
            placeholder="203.0.113.5, 2001:db8::1"
          />
        </label>
        <label className="field settings-field">
          <span className="field__label">Proxy apps (process names)</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.proxyApps}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, proxyApps: event.target.value }))
            }
            placeholder="chrome.exe, tailscale-ipn.exe"
          />
        </label>
            <label className="field settings-field">
              <span className="field__label">Direct domains</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={splitForm.bypassDomains}
                onChange={(event) =>
                  setSplitForm((prev) => ({ ...prev, bypassDomains: event.target.value }))
                }
                placeholder="intranet.local"
              />
        </label>
        <label className="field settings-field">
          <span className="field__label">Direct IPs / CIDRs</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.bypassIPs}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, bypassIPs: event.target.value }))
            }
            placeholder="10.0.0.0/8"
          />
        </label>
        <label className="field settings-field">
          <span className="field__label">Direct apps (process names)</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.bypassApps}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, bypassApps: event.target.value }))
            }
            placeholder="onedrive.exe, outlook.exe"
          />
        </label>
            <label className="field settings-field">
              <span className="field__label">Blocked domains</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={splitForm.blockDomains}
                onChange={(event) =>
                  setSplitForm((prev) => ({ ...prev, blockDomains: event.target.value }))
                }
                placeholder="ads.example.com"
              />
        </label>
        <label className="field settings-field">
          <span className="field__label">Blocked IPs / CIDRs</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.blockIPs}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, blockIPs: event.target.value }))
            }
            placeholder="198.51.100.0/24"
          />
        </label>
        <label className="field settings-field">
          <span className="field__label">Blocked apps (process names)</span>
          <textarea
            className="field__input field__input--textarea"
            rows={2}
            value={splitForm.blockApps}
            onChange={(event) =>
              setSplitForm((prev) => ({ ...prev, blockApps: event.target.value }))
            }
            placeholder="game.exe"
          />
        </label>
          </div>
        </section>

        <section className="settings-section">
          <h3>DNS</h3>
          <div className="settings-inline">
            <label className="field settings-field">
              <span className="field__label">Strategy</span>
              <select
                className="field__select"
                value={dnsForm.strategy}
                onChange={(event) => setDnsForm((prev) => ({ ...prev, strategy: event.target.value }))}
              >
                <option value="prefer_ipv4">prefer_ipv4</option>
                <option value="prefer_ipv6">prefer_ipv6</option>
                <option value="ipv4_only">ipv4_only</option>
                <option value="ipv6_only">ipv6_only</option>
              </select>
            </label>
            <button className="chip-button" type="button" onClick={addDnsRow}>
              Add server
            </button>
          </div>
          <div className="dns-rows">
            {dnsForm.servers.length === 0 ? (
              <div className="settings-hint">No custom DNS servers configured.</div>
            ) : (
              dnsForm.servers.map((row) => (
                <div key={row.id} className="dns-row">
                  <input
                    className="dns-row__input"
                    placeholder='Address or "local"'
                    value={row.address}
                    onChange={(event) => updateDnsRow(row.id, "address", event.target.value)}
                  />
                  <select
                    className="dns-row__select"
                    value={row.type}
                    onChange={(event) => updateDnsRow(row.id, "type", event.target.value)}
                  >
                    <option value="https">https</option>
                    <option value="tls">tls</option>
                    <option value="udp">udp</option>
                    <option value="tcp">tcp</option>
                    <option value="local">local</option>
                  </select>
                  <input
                    className="dns-row__input"
                    placeholder="Tag"
                    value={row.tag}
                    onChange={(event) => updateDnsRow(row.id, "tag", event.target.value)}
                  />
                  <input
                    className="dns-row__input"
                    placeholder="Detour"
                    value={row.detour}
                    onChange={(event) => updateDnsRow(row.id, "detour", event.target.value)}
                  />
                  <input
                    className="dns-row__input"
                    placeholder="Strategy override"
                    value={row.strategy}
                    onChange={(event) => updateDnsRow(row.id, "strategy", event.target.value)}
                  />
                  <button
                    type="button"
                    className="chip-button chip-button--danger"
                    onClick={() => removeDnsRow(row.id)}
                  >
                    Remove
                  </button>
                </div>
              ))
            )}
          </div>
        </section>

        <section className="settings-section">
          <h3>Regional routing</h3>
          <p className="settings-section__hint">
            Use two-letter country codes (e.g. US, RU). Separate entries with commas or new lines.
          </p>
          <div className="settings-grid settings-grid--three">
            <label className="field settings-field">
              <span className="field__label">Proxy countries</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={regionForm.proxyCountries}
                onChange={(event) =>
                  setRegionForm((prev) => ({ ...prev, proxyCountries: event.target.value }))
                }
                placeholder="US, GB"
              />
            </label>
            <label className="field settings-field">
              <span className="field__label">Direct countries</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={regionForm.directCountries}
                onChange={(event) =>
                  setRegionForm((prev) => ({ ...prev, directCountries: event.target.value }))
                }
                placeholder="RU, KZ"
              />
            </label>
            <label className="field settings-field">
              <span className="field__label">Blocked countries</span>
              <textarea
                className="field__input field__input--textarea"
                rows={2}
                value={regionForm.blockCountries}
                onChange={(event) =>
                  setRegionForm((prev) => ({ ...prev, blockCountries: event.target.value }))
                }
                placeholder="CN"
              />
            </label>
          </div>
        </section>

        <section className="settings-section">
          <h3>Metrics & observatory</h3>
          <label className="settings-inline settings-inline--checkbox">
            <input
              type="checkbox"
              checked={metricsForm.enableObservatory}
              onChange={(event) =>
                setMetricsForm((prev) => ({ ...prev, enableObservatory: event.target.checked }))
              }
            />
            <span>Expose sing-box observatory endpoint</span>
          </label>
          <div className="settings-grid settings-grid--two">
            <label className="field settings-field">
              <span className="field__label">Listen address</span>
              <input
                className="field__input"
                value={metricsForm.observatoryListen}
                onChange={(event) =>
                  setMetricsForm((prev) => ({ ...prev, observatoryListen: event.target.value }))
                }
                placeholder="127.0.0.1:9090"
              />
            </label>
            <label className="field settings-field">
              <span className="field__label">Access token (optional)</span>
              <input
                className="field__input"
                value={metricsForm.observatoryToken}
                onChange={(event) =>
                  setMetricsForm((prev) => ({ ...prev, observatoryToken: event.target.value }))
                }
                placeholder="Leave empty to disable auth"
              />
            </label>
          </div>
        </section>
      </div>
    </section>
  </div>
);

const renderLogs = () => (
  <section className="panel logs-panel">
      <header className="panel__header">
        <h2>Core logs</h2>
        <span className="meta-label">{logs.length} entries</span>
      </header>
      <div className="logs-panel__body">
        {logs.length ? (
          <pre className="logs-panel__pre" aria-live="polite">
            {logs.join("\n")}
          </pre>
        ) : (
          <div className="empty-state empty-state--compact">
            <p>Logs will appear here as soon as the core starts emitting events.</p>
          </div>
        )}
      </div>
    </section>
  );

  return (
    <div className={`app-shell ${sidebarCollapsed ? "app-shell--collapsed" : ""}`}>
      <aside className={`sidebar ${sidebarCollapsed ? "sidebar--collapsed" : ""}`}>
        <button
          type="button"
          className="sidebar__collapse"
          onClick={toggleSidebar}
          aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          {sidebarCollapsed ? ">" : "<"}
        </button>
<div className="brand">
  <img
    src={sidebarCollapsed ? logoMarkUrl : logoFullUrl}
    alt="VeilBox"
    className={`brand__mark ${sidebarCollapsed ? "brand__mark--icon" : "brand__mark--full"}`}
    height={28}          // управление размером по высоте, ширина сама подстроится
    decoding="async"
  />
  {!sidebarCollapsed && (
    <span className="brand__tagline">Reality / VLESS</span>
  )}
</div>
        <nav className="nav">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.key}
              type="button"
              className={`nav__item ${view === item.key ? "nav__item--active" : ""}`}
              onClick={() => setView(item.key)}
              aria-current={view === item.key}
            >
              <span className="nav__bullet" aria-hidden="true" />
              <span className="nav__label">{item.label}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar__footer">
          <div className="sidebar__status">
            <span className={`connection-indicator ${connected ? "connection-indicator--online" : ""}`} />
            {connected ? "Connected" : "Idle"}
          </div>
          <span className="sidebar__footnote">sing-box 1.12.11</span>
        </div>
      </aside>
      <main className="content">
        <header className="content__header">
          <div>
            <h1>{view === "dashboard" ? "Dashboard" : "Logs"}</h1>
            <p>
              {view === "dashboard"
                ? "Monitor your secure tunnel, switch profiles, and connect with one click."
                : "Live output directly from the embedded sing-box core."}
            </p>
          </div>
          <div className="header-actions">
            <button
              className="ghost-button"
              type="button"
              onClick={() => openProfileModal(selectedProfile)}
            >
              <ShieldIcon />
              Manage profiles
            </button>
          </div>
        </header>
        {view === "dashboard" && renderDashboard()}
        {view === "logs" && renderLogs()}
        {view === "settings" && renderSettings()}
      </main>

      <div className="toast-stack" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast toast--${toast.tone}`}>
            <span className="toast__message">{toast.message}</span>
            <button
              type="button"
              className="toast__close"
              onClick={() => removeToast(toast.id)}
              aria-label="Dismiss notification"
            >
              ×
            </button>
          </div>
        ))}
      </div>



      {showProfileModal ? (
        <div className="modal-backdrop" role="dialog" aria-modal="true">
          <div className="modal">
            <header className="modal__header">
              <h2>{editingProfileId ? "Edit profile" : "Add profile"}</h2>
            </header>
            <div className="modal__body">
              <label className="field">
                <span className="field__label">Display name</span>
                <input
                  className="field__input"
                  value={formLabel}
                  onChange={(event) => setFormLabel(event.target.value)}
                  placeholder="My Reality Node"
                />
              </label>
              <label className="field">
                <span className="field__label">VLESS URI</span>
                <textarea
                  className="field__input field__input--textarea"
                  rows={3}
                  value={formUri}
                  onChange={(event) => {
                    setFormUri(event.target.value);
                    setFormError(null);
                  }}
                  placeholder="vless://<uuid>@host:443?type=tcp&security=reality..."
                />
              </label>
              {formError ? <p className="field__error">{formError}</p> : null}
              {parsedPreview ? (
                <div className="preview">
                  <h3>Preview</h3>
                  <div className="preview__grid">
                    <div>
                      <span className="preview__label">Node</span>
                      <span className="preview__value">{parsedPreview.nodeName}</span>
                    </div>
                    <div>
                      <span className="preview__label">Host</span>
                      <span className="preview__value">
                        {parsedPreview.host}:{parsedPreview.port}
                      </span>
                    </div>
                    <div>
                      <span className="preview__label">Transport</span>
                      <span className="preview__value">{parsedPreview.transport}</span>
                    </div>
                    <div>
                      <span className="preview__label">Flow</span>
                      <span className="preview__value">{parsedPreview.flow || "-"}</span>
                    </div>
                    <div>
                      <span className="preview__label">Country</span>
                      <span className="preview__value">{previewCountry ?? parsedPreview.country}</span>
                    </div>
                    <div>
                      <span className="preview__label">SNI</span>
                      <span className="preview__value">{parsedPreview.sni}</span>
                    </div>
                  </div>
                </div>
              ) : null}
            </div>
            <footer className="modal__footer">
              <button className="ghost-button" type="button" onClick={resetModal}>
                Cancel
              </button>
              <button className="btn btn--primary" type="button" onClick={() => void handleSaveProfile()}>
                {editingProfileId ? "Save changes" : "Save profile"}
              </button>
            </footer>
          </div>
        </div>
      ) : null}
    </div>
  );
}




