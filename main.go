package main

import (
	"embed"
	"github.com/google/uuid"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/logger"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/windows"
	"github.com/wailsapp/wails/v2/pkg/runtime"
	"log"
)

//go:embed frontend/dist
var assets embed.FS

func main() {
	app := NewApp()

	err := wails.Run(&options.App{
		Title:   "VeilBox",
		Width:   900,
		Height:  640,
		Logger:  logger.NewDefaultLogger(),
		Assets:  assets,
		Windows: &windows.Options{},
		SingleInstanceLock: &options.SingleInstanceLock{
			UniqueId: "veilbox-" + uuid.NewString(), // один и тот же ID в проде
			OnSecondInstanceLaunch: func(data options.SecondInstanceData) {
				if app != nil && app.ctx != nil {
					runtime.Show(app.ctx)
					runtime.EventsEmit(app.ctx, "tray:notification", "VeilBox уже запущён")
				}
			},
		},
		OnStartup:         app.Startup,
		OnBeforeClose:     app.BeforeClose,
		HideWindowOnClose: true,
		Bind:              []interface{}{app},
		Debug: options.Debug{
			OpenInspectorOnStartup: true,
		},
	})
	if err != nil {
		log.Fatal(err)
	}
}
