//go:build !windows

package main

type trayController struct{}

func (a *App) initTray() {}

func (a *App) trayUpdateLocked() {}

func (a *App) trayUpdateWindowLocked() {}
