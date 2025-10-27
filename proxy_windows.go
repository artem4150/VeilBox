//go:build windows

package main

import (
	"fmt"
	"golang.org/x/sys/windows/registry"
)

func SetSystemProxy(host string, port int) error {
	k, err := registry.OpenKey(registry.CURRENT_USER,
		`Software\Microsoft\Windows\CurrentVersion\Internet Settings`, registry.SET_VALUE)
	if err != nil { return fmt.Errorf("open reg: %w", err) }
	defer k.Close()
	if err := k.SetDWordValue("ProxyEnable", 1); err != nil { return err }
	if err := k.SetStringValue("ProxyServer", fmt.Sprintf("%s:%d", host, port)); err != nil { return err }
	return nil
}

func DisableSystemProxy() error {
	k, err := registry.OpenKey(registry.CURRENT_USER,
		`Software\Microsoft\Windows\CurrentVersion\Internet Settings`, registry.SET_VALUE)
	if err != nil { return fmt.Errorf("open reg: %w", err) }
	defer k.Close()
	if err := k.SetDWordValue("ProxyEnable", 0); err != nil { return err }
	return nil
}
