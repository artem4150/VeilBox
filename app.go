package main

import (
	"context"
	"fmt"
	"time"
)

type App struct {
	ctx       context.Context
	runner    *CoreRunner
	logBuffer *RingBuffer
}

func NewApp() *App {
	return &App{
		runner:    NewCoreRunner(),
		logBuffer: NewRingBuffer(2000),
	}
}

func (a *App) Startup(ctx context.Context) {
	a.ctx = ctx
	a.runner.OnLog(func(s string) { a.logBuffer.Append(s) })
}

type ConnectRequest struct {
	VLESSURI string
	Mode     string // "proxy" | "tun" (пока используем proxy)
}

func (a *App) Connect(req ConnectRequest) (string, error) {
	prof, err := ParseVLESS(req.VLESSURI)
	if err != nil {
		return "", fmt.Errorf("parse failed: %w", err)
	}
	jsonCfg, err := BuildConfigFromProfile(prof, req.Mode)
	if err != nil {
		return "", fmt.Errorf("config build failed: %w", err)
	}
	if err := a.runner.Start(jsonCfg); err != nil {
		return "", fmt.Errorf("start core failed: %w", err)
	}
	return "connected", nil
}

func (a *App) Disconnect() (string, error) {
	a.runner.Stop()
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
