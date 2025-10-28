//go:build windows

package main

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"syscall"
	"time"
)

type CoreRunner struct {
	mu     sync.Mutex
	cmd    *exec.Cmd
	cancel context.CancelFunc

	onLogMu sync.Mutex
	onLog   func(string) // коллбек для логов (используется из app.go)
}

// Конструктор, которого не хватало в app.go
func NewCoreRunner() *CoreRunner {
	return &CoreRunner{}
}

// Позволяет app.go подписаться на новые строки логов core
func (r *CoreRunner) OnLog(fn func(string)) {
	r.onLogMu.Lock()
	defer r.onLogMu.Unlock()
	r.onLog = fn
}

func (r *CoreRunner) emit(line string) {
	r.onLogMu.Lock()
	fn := r.onLog
	r.onLogMu.Unlock()
	if fn != nil {
		fn(line)
	}
}

// Start принимает уже собранный JSON-конфиг sing-box (как строку),
// сохраняет его в %LOCALAPPDATA%\VeilBox\sb_config.json,
// и запускает sing-box СКРЫТО с рабочей директорией = %LOCALAPPDATA%\VeilBox,
// чтобы не требовались админ-права и не появлялось консольное окно.
func (r *CoreRunner) Start(configJSON string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	// Если уже запущено — мягко остановим
	if r.cmd != nil && r.cmd.Process != nil {
		_ = r.stopLocked(2 * time.Second)
	}

	// Папка данных юзера
	dataDir := filepath.Join(os.Getenv("LOCALAPPDATA"), "VeilBox")
	if err := os.MkdirAll(dataDir, 0o755); err != nil {
		return fmt.Errorf("create data dir: %w", err)
	}

	// Конфиг
	cfgPath := filepath.Join(dataDir, "sb_config.json")
	if err := os.WriteFile(cfgPath, []byte(configJSON), 0o644); err != nil {
		return fmt.Errorf("write config: %w", err)
	}

	// Путь к sing-box.exe: {app}\core\sing-box.exe
	exe, err := os.Executable()
	if err != nil {
		return fmt.Errorf("get executable: %w", err)
	}
	appDir := filepath.Dir(exe)
	coreExe := filepath.Join(appDir, "core", "sing-box.exe")
	if _, err := os.Stat(coreExe); err != nil {
		return fmt.Errorf("sing-box.exe not found at %s: %w", coreExe, err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	r.cancel = cancel

	cmd := exec.CommandContext(ctx, coreExe, "run", "-c", cfgPath)
	cmd.Dir = dataDir // здесь создастся cache.db и сложатся логи

	// Скрыть консольное окно
	cmd.SysProcAttr = &syscall.SysProcAttr{
		HideWindow:    true,
		CreationFlags: 0x08000000, // CREATE_NO_WINDOW
	}

	// Подключим пайпы
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("stdout: %w", err)
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("stderr: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start core: %w", err)
	}
	r.cmd = cmd

	// Читаем логи и прокидываем в подписчиков
	go r.pipe(stdout)
	go r.pipe(stderr)

	// Следим за завершением
	go func() {
		_ = cmd.Wait()
		r.mu.Lock()
		r.cmd = nil
		r.cancel = nil
		r.mu.Unlock()
	}()

	return nil
}

func (r *CoreRunner) pipe(rd io.Reader) {
	sc := bufio.NewScanner(rd)
	for sc.Scan() {
		line := sc.Text()
		r.emit(line)
	}
}

func (r *CoreRunner) Stop() error {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.stopLocked(2 * time.Second)
}

func (r *CoreRunner) stopLocked(grace time.Duration) error {
	if r.cancel != nil {
		r.cancel()
	}
	if r.cmd != nil && r.cmd.Process != nil {
		done := make(chan struct{})
		go func() {
			_ = r.cmd.Wait()
			close(done)
		}()
		select {
		case <-done:
		case <-time.After(grace):
			_ = r.cmd.Process.Kill()
		}
	}
	r.cmd = nil
	r.cancel = nil
	return nil
}
