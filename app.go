package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"
)

type App struct {
	ctx           context.Context
	runner        *CoreRunner
	logBuffer     *RingBuffer
	mu            sync.Mutex
	lastRequest   *ConnectRequest
	connected     bool
	windowVisible bool
	quitRequested bool
	trayOnce      sync.Once
	tray          *trayController
	metricsCancel context.CancelFunc
}

func NewApp() *App {
	return &App{
		runner:        NewCoreRunner(),
		logBuffer:     NewRingBuffer(2000),
		windowVisible: true,
	}
}

func (a *App) Startup(ctx context.Context) {
	a.ctx = ctx
	a.runner.OnLog(func(s string) { a.logBuffer.Append(s) })
	a.initTray()
}

func (a *App) BeforeClose(ctx context.Context) bool {
	if a.shouldQuit() {
		return false
	}
	a.setWindowVisible(false)
	runtime.Hide(ctx)
	return true
}

type ConnectRequest struct {
	VLESSURI      string                 `json:"VLESSURI"`
	Mode          string                 `json:"Mode"`
	SplitTunnel   *SplitTunnelSettings   `json:"SplitTunnel"`
	DNS           *DNSSettings           `json:"DNS"`
	RegionRouting *RegionRoutingSettings `json:"RegionRouting"`
	Metrics       *MetricsSettings       `json:"Metrics"`
}

func (a *App) Connect(req ConnectRequest) (string, error) {
	prof, err := ParseVLESS(req.VLESSURI)
	if err != nil {
		return "", fmt.Errorf("parse failed: %w", err)
	}
	opts := ConnectionOptions{
		Mode:          req.Mode,
		SplitTunnel:   req.SplitTunnel,
		DNS:           req.DNS,
		RegionRouting: req.RegionRouting,
		Metrics:       req.Metrics,
	}
	jsonCfg, err := BuildConfigFromProfile(prof, opts)
	if err != nil {
		return "", fmt.Errorf("config build failed: %w", err)
	}
	if err := a.runner.Start(jsonCfg); err != nil {
		return "", fmt.Errorf("start core failed: %w", err)
	}
	a.setConnected(true)
	a.setLastRequest(&req)
	a.startMetricsCollector(req.Metrics)
	return "connected", nil
}

func (a *App) Disconnect() (string, error) {
	a.runner.Stop()
	a.setConnected(false)
	a.stopMetricsCollector()
	return "disconnected", nil
}

func (a *App) EnableSystemProxy() error {
	return SetSystemProxy("127.0.0.1", 10809)
}

func (a *App) DisableSystemProxy() error {
	return DisableSystemProxy()
}

func (a *App) TailLogs(last int) []string {
	return a.logBuffer.LastN(last)
}

func (a *App) PingHost(host string) (int64, error) {
	t0 := time.Now()
	_ = host
	return time.Since(t0).Milliseconds(), nil
}

func (a *App) setConnected(connected bool) {
	a.mu.Lock()
	a.connected = connected
	a.trayUpdateLocked()
	a.mu.Unlock()
}

func (a *App) setLastRequest(req *ConnectRequest) {
	a.mu.Lock()
	if req == nil {
		a.lastRequest = nil
	} else {
		a.lastRequest = cloneConnectRequest(*req)
	}
	a.trayUpdateLocked()
	a.mu.Unlock()
}

func (a *App) getLastRequest() *ConnectRequest {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.lastRequest == nil {
		return nil
	}
	return cloneConnectRequest(*a.lastRequest)
}

func (a *App) isConnected() bool {
	a.mu.Lock()
	defer a.mu.Unlock()
	return a.connected
}

func (a *App) setWindowVisible(visible bool) {
	a.mu.Lock()
	a.windowVisible = visible
	a.trayUpdateWindowLocked()
	a.mu.Unlock()
}

func (a *App) requestQuit() {
	a.mu.Lock()
	a.quitRequested = true
	a.mu.Unlock()
}

func (a *App) shouldQuit() bool {
	a.mu.Lock()
	defer a.mu.Unlock()
	return a.quitRequested
}

func cloneConnectRequest(req ConnectRequest) *ConnectRequest {
	out := req
	if req.SplitTunnel != nil {
		clone := *req.SplitTunnel
		clone.BypassDomains = append([]string(nil), req.SplitTunnel.BypassDomains...)
		clone.BypassIPs = append([]string(nil), req.SplitTunnel.BypassIPs...)
		clone.ProxyDomains = append([]string(nil), req.SplitTunnel.ProxyDomains...)
		clone.ProxyIPs = append([]string(nil), req.SplitTunnel.ProxyIPs...)
		clone.BlockDomains = append([]string(nil), req.SplitTunnel.BlockDomains...)
		clone.BlockIPs = append([]string(nil), req.SplitTunnel.BlockIPs...)
		clone.BypassProcesses = append([]string(nil), req.SplitTunnel.BypassProcesses...)
		clone.ProxyProcesses = append([]string(nil), req.SplitTunnel.ProxyProcesses...)
		clone.BlockProcesses = append([]string(nil), req.SplitTunnel.BlockProcesses...)
		out.SplitTunnel = &clone
	}
	if req.DNS != nil {
		clone := *req.DNS
		if len(req.DNS.Servers) > 0 {
			servers := make([]DNSUpstream, len(req.DNS.Servers))
			copy(servers, req.DNS.Servers)
			clone.Servers = servers
		}
		out.DNS = &clone
	}
	if req.RegionRouting != nil {
		clone := *req.RegionRouting
		clone.ProxyCountries = append([]string(nil), req.RegionRouting.ProxyCountries...)
		clone.DirectCountries = append([]string(nil), req.RegionRouting.DirectCountries...)
		clone.BlockCountries = append([]string(nil), req.RegionRouting.BlockCountries...)
		out.RegionRouting = &clone
	}
	if req.Metrics != nil {
		clone := *req.Metrics
		out.Metrics = &clone
	}
	return &out
}

func (a *App) startMetricsCollector(settings *MetricsSettings) {
	a.stopMetricsCollector()
	if settings == nil || !settings.EnableObservatory {
		return
	}
	listen := strings.TrimSpace(settings.ObservatoryListen)
	if listen == "" {
		listen = "127.0.0.1:9090"
	}
	token := strings.TrimSpace(settings.ObservatoryToken)
	baseURL := listen
	if !strings.Contains(baseURL, "://") {
		baseURL = "http://" + baseURL
	}
	baseURL = strings.TrimRight(baseURL, "/")

	ctx, cancel := context.WithCancel(context.Background())
	a.mu.Lock()
	a.metricsCancel = cancel
	a.mu.Unlock()

	go a.pollTraffic(ctx, baseURL+"/traffic", token)
}

func (a *App) stopMetricsCollector() {
	a.mu.Lock()
	cancel := a.metricsCancel
	a.metricsCancel = nil
	a.mu.Unlock()
	if cancel != nil {
		cancel()
	}
}

func (a *App) pollTraffic(ctx context.Context, endpoint string, token string) {
	client := &http.Client{
		Timeout: 15 * time.Second,
	}
	delay := time.Second
	for {
		if ctx.Err() != nil {
			return
		}
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
		if err != nil {
			return
		}
		req.Header.Set("Accept", "application/json")
		if token != "" {
			req.Header.Set("Authorization", "Bearer "+token)
		}
		resp, err := client.Do(req)
		if err != nil {
			if ctx.Err() != nil {
				return
			}
			time.Sleep(delay)
			if delay < 30*time.Second {
				delay *= 2
			}
			continue
		}

		delay = time.Second
		err = a.streamTraffic(ctx, resp.Body)
		_ = resp.Body.Close()
		if ctx.Err() != nil {
			return
		}
		if err != nil {
			time.Sleep(delay)
			if delay < 30*time.Second {
				delay *= 2
			}
		}
	}
}

func (a *App) streamTraffic(ctx context.Context, reader io.Reader) error {
	type sample struct {
		Up   int64 `json:"up"`
		Down int64 `json:"down"`
	}
	decoder := json.NewDecoder(reader)
	for {
		if ctx.Err() != nil {
			return ctx.Err()
		}
		var s sample
		if err := decoder.Decode(&s); err != nil {
			return err
		}
		if a.ctx != nil {
			runtime.EventsEmit(a.ctx, "core:throughput", map[string]int64{
				"up":   s.Up,
				"down": s.Down,
			})
		}
	}
}
