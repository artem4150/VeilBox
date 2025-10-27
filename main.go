package main

import (
	"embed"
	"log"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/logger"
	"github.com/wailsapp/wails/v2/pkg/options"
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
    OnStartup: app.Startup,
    Bind:    []interface{}{app},

    // добавь это:
    Debug: options.Debug{
        OpenInspectorOnStartup: true, // откроет DevTools
    },
})
	if err != nil {
		log.Fatal(err)
	}
}
