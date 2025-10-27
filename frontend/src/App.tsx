import { useEffect, useState } from "react";
import {
  Connect,
  Disconnect,
  EnableSystemProxy,
  DisableSystemProxy,
  TailLogs
} from "../wailsjs/go/main/App";




export default function App() {
  const [uri, setURI] = useState("");
  const [mode, setMode] = useState<"proxy" | "tun">("proxy");
  const [connected, setConnected] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    const t = setInterval(async () => {
      try { setLogs(await TailLogs(200)); } catch {}
    }, 700);
    return () => clearInterval(t);
  }, []);

  const onConnect = async () => {
    try { await Connect({ VLESSURI: uri, Mode: mode }); setConnected(true); }
    catch (e:any) { alert("Connect error: " + e?.toString()); }
  };

  const onDisconnect = async () => {
    try { await Disconnect(); setConnected(false); }
    catch (e:any) { alert("Disconnect error: " + e?.toString()); }
  };

  return (
    <div style={{ fontFamily: "Inter, system-ui, Arial", padding: 16 }}>
      <h1>VeilBox</h1>
      <div style={{ display: "grid", gap: 8, maxWidth: 800 }}>
        <label>VLESS URI:</label>
        <input value={uri} onChange={(e)=>setURI(e.target.value)}
               placeholder="vless://<uuid>@host:port?...&type=grpc&serviceName=...#Node"
               style={{ padding: 8, border: "1px solid #ccc", borderRadius: 6 }} />

        <div>
          <label>Mode: </label>
          <select value={mode} onChange={(e)=>setMode(e.target.value as any)}>
            <option value="proxy">System Proxy</option>
            <option value="tun" disabled>TUN (soon)</option>
          </select>
        </div>

        <div style={{ display: "flex", gap: 8 }}>
          {!connected
            ? <button onClick={onConnect} style={{ padding:"8px 14px" }}>Connect</button>
            : <button onClick={onDisconnect} style={{ padding:"8px 14px" }}>Disconnect</button>}
          <button onClick={()=>EnableSystemProxy()} style={{ padding:"8px 14px" }}>Enable System Proxy</button>
          <button onClick={()=>DisableSystemProxy()} style={{ padding:"8px 14px" }}>Disable System Proxy</button>
        </div>

        <div>
          <h3>Logs</h3>
          <pre style={{ background:"#0b0d12", color:"#cfe1ff", padding:12, borderRadius:8, height:300, overflow:"auto" }}>
            {logs.join("\n")}
          </pre>
        </div>
      </div>
    </div>
  );
}
