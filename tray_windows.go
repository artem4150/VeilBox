//go:build windows

package main

import (
	_ "embed"
	"fmt"

	"github.com/getlantern/systray"
	"github.com/wailsapp/wails/v2/pkg/runtime"
)

//go:embed build/windows/icon.ico
var trayIconBytes []byte

type trayController struct {
	status     *systray.MenuItem
	show       *systray.MenuItem
	hide       *systray.MenuItem
	connect    *systray.MenuItem
	disconnect *systray.MenuItem
	quit       *systray.MenuItem
	done       chan struct{}
}

func (a *App) initTray() {
	a.trayOnce.Do(func() {
		go systray.Run(a.onTrayReady, a.onTrayExit)
	})
}

func (a *App) onTrayReady() {
	if len(trayIconBytes) > 0 {
		systray.SetIcon(trayIconBytes)
	}
	systray.SetTooltip("VeilBox")

	status := systray.AddMenuItem("Starting...", "")
	status.Disable()
	systray.AddSeparator()
	show := systray.AddMenuItem("Show Dashboard", "")
	hide := systray.AddMenuItem("Hide Window", "")
	systray.AddSeparator()
	connect := systray.AddMenuItem("Connect", "")
	disconnect := systray.AddMenuItem("Disconnect", "")
	systray.AddSeparator()
	quit := systray.AddMenuItem("Quit", "")

	controller := &trayController{
		status:     status,
		show:       show,
		hide:       hide,
		connect:    connect,
		disconnect: disconnect,
		quit:       quit,
		done:       make(chan struct{}),
	}

	a.mu.Lock()
	a.tray = controller
	a.trayUpdateLocked()
	a.trayUpdateWindowLocked()
	a.mu.Unlock()

	go a.handleTrayClicks(controller)
}

func (a *App) onTrayExit() {
	a.mu.Lock()
	if a.tray != nil {
		close(a.tray.done)
		a.tray = nil
	}
	a.mu.Unlock()
}

func (a *App) handleTrayClicks(ctrl *trayController) {
	for {
		select {
		case <-ctrl.show.ClickedCh:
			if a.ctx != nil {
				runtime.Show(a.ctx)
			}
			a.setWindowVisible(true)
		case <-ctrl.hide.ClickedCh:
			if a.ctx != nil {
				runtime.Hide(a.ctx)
			}
			a.setWindowVisible(false)
		case <-ctrl.connect.ClickedCh:
			go a.handleTrayConnect()
		case <-ctrl.disconnect.ClickedCh:
			go a.handleTrayDisconnect()
		case <-ctrl.quit.ClickedCh:
			a.handleTrayQuit()
			return
		case <-ctrl.done:
			return
		}
	}
}

func (a *App) handleTrayConnect() {
	if a.isConnected() {
		a.trayNotify("Already connected")
		return
	}
	req := a.getLastRequest()
	if req == nil {
		a.trayNotify("No recent profile to reconnect. Opening dashboard...")
		if a.ctx != nil {
			runtime.Show(a.ctx)
			runtime.EventsEmit(a.ctx, "tray:requestProfile")
		}
		return
	}
	if _, err := a.Connect(*req); err != nil {
		a.trayNotifyError(fmt.Errorf("connect failed: %w", err))
		return
	}
	if err := a.EnableSystemProxy(); err != nil {
		a.trayNotifyError(fmt.Errorf("enable proxy failed: %w", err))
	}
	if a.ctx != nil {
		runtime.EventsEmit(a.ctx, "tray:state", "connected")
	}
}

func (a *App) handleTrayDisconnect() {
	if !a.isConnected() {
		a.trayNotify("Already disconnected")
		return
	}
	if _, err := a.Disconnect(); err != nil {
		a.trayNotifyError(fmt.Errorf("disconnect failed: %w", err))
		return
	}
	if err := a.DisableSystemProxy(); err != nil {
		a.trayNotifyError(fmt.Errorf("disable proxy failed: %w", err))
	}
	if a.ctx != nil {
		runtime.EventsEmit(a.ctx, "tray:state", "disconnected")
	}
}

func (a *App) handleTrayQuit() {
	a.requestQuit()
	_ = a.DisableSystemProxy()
	a.runner.Stop()
	a.stopMetricsCollector()
	systray.Quit()
	if a.ctx != nil {
		runtime.Quit(a.ctx)
	}
}

func (a *App) trayUpdateLocked() {
	if a.tray == nil {
		return
	}
	if a.connected {
		a.tray.status.SetTitle("Connected")
		a.tray.connect.Disable()
		a.tray.disconnect.Enable()
	} else {
		a.tray.status.SetTitle("Disconnected")
		if a.lastRequest == nil {
			a.tray.connect.Disable()
		} else {
			a.tray.connect.Enable()
		}
		a.tray.disconnect.Disable()
	}
}

func (a *App) trayUpdateWindowLocked() {
	if a.tray == nil {
		return
	}
	if a.windowVisible {
		a.tray.show.Disable()
		a.tray.hide.Enable()
	} else {
		a.tray.show.Enable()
		a.tray.hide.Disable()
	}
}

func (a *App) trayNotify(message string) {
	if a.ctx != nil {
		runtime.EventsEmit(a.ctx, "tray:notification", message)
	}
}

func (a *App) trayNotifyError(err error) {
	if a.ctx != nil {
		runtime.EventsEmit(a.ctx, "tray:error", err.Error())
	}
}
