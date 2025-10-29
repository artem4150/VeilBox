//go:build windows

package main

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"syscall"
	"time"
)

const (
	errSharingViolation = syscall.Errno(32)
	errLockViolation    = syscall.Errno(33)
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

	cachePath, err := r.prepareCacheFile(dataDir)
	if err != nil {
		return fmt.Errorf("prepare cache file: %w", err)
	}
	configJSON = strings.ReplaceAll(configJSON, "__CACHE_FILE_PATH__", jsonEscape(cachePath))

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

func (r *CoreRunner) prepareCacheFile(dataDir string) (string, error) {
	preferred := filepath.Join(dataDir, "cache.db")
	if err := waitForWritable(preferred, 6*time.Second); err == nil {
		return preferred, nil
	} else {
		fallback, fbErr := createFallbackCache(dataDir)
		if fbErr != nil {
			return "", fmt.Errorf("cache.db busy: %w (last error: %v)", fbErr, err)
		}
		r.emit(fmt.Sprintf("cache.db busy (%v), switched to %s", err, filepath.Base(fallback)))
		cleanupOldCaches(dataDir, fallback)
		return fallback, nil
	}
}

func waitForWritable(path string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	var lastErr error
	for time.Now().Before(deadline) {
		f, err := os.OpenFile(path, os.O_RDWR|os.O_CREATE, 0o600)
		if err == nil {
			if cerr := f.Close(); cerr != nil {
				lastErr = cerr
			} else {
				return nil
			}
		} else {
			lastErr = err
			if os.IsPermission(err) {
				_ = os.Remove(path)
			} else if !isSharingViolation(err) && !errors.Is(err, syscall.ERROR_ACCESS_DENIED) {
				return err
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	if lastErr == nil {
		lastErr = fmt.Errorf("cache file timed out")
	}
	return lastErr
}

func createFallbackCache(dir string) (string, error) {
	for i := 0; i < 6; i++ {
		name := fmt.Sprintf("cache-%d.db", time.Now().UnixNano())
		path := filepath.Join(dir, name)
		f, err := os.OpenFile(path, os.O_RDWR|os.O_CREATE|os.O_EXCL, 0o600)
		if err == nil {
			_ = f.Close()
			return path, nil
		}
		if !os.IsExist(err) && !isSharingViolation(err) {
			return "", err
		}
		time.Sleep(100 * time.Millisecond)
	}
	return "", fmt.Errorf("no alternative cache file available")
}

func cleanupOldCaches(dir string, keep string) {
	matches, err := filepath.Glob(filepath.Join(dir, "cache-*.db"))
	if err != nil || len(matches) == 0 {
		return
	}
	sort.Slice(matches, func(i, j int) bool {
		ii, errI := os.Stat(matches[i])
		jj, errJ := os.Stat(matches[j])
		if errI != nil || errJ != nil {
			return matches[i] < matches[j]
		}
		return ii.ModTime().Before(jj.ModTime())
	})
	keepSet := map[string]struct{}{keep: {}}
	for i := len(matches) - 1; i >= 0 && len(keepSet) < 3; i-- {
		keepSet[matches[i]] = struct{}{}
	}
	for _, path := range matches {
		if _, ok := keepSet[path]; ok {
			continue
		}
		_ = os.Remove(path)
	}
}

func isSharingViolation(err error) bool {
	for {
		switch e := err.(type) {
		case syscall.Errno:
			return e == errSharingViolation || e == errLockViolation
		case *os.PathError:
			err = e.Err
		default:
			var errno syscall.Errno
			if errors.As(err, &errno) {
				return errno == errSharingViolation || errno == errLockViolation
			}
			return false
		}
	}
}
