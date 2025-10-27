package main

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"time"
)

type CoreRunner struct {
	mu     sync.Mutex
	cmd    *exec.Cmd
	onLog  func(string)
	cancel context.CancelFunc
}

func NewCoreRunner() *CoreRunner            { return &CoreRunner{} }
func (r *CoreRunner) OnLog(cb func(string)) { r.onLog = cb }

func (r *CoreRunner) Start(configJSON string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if r.cmd != nil && r.cmd.Process != nil {
		return fmt.Errorf("already running")
	}

	tmpDir := filepath.Join(os.TempDir(), "veilbox")
	_ = os.MkdirAll(tmpDir, 0o755)
	cfgPath := filepath.Join(tmpDir, fmt.Sprintf("sb_%d.json", time.Now().UnixNano()))
	if err := os.WriteFile(cfgPath, []byte(configJSON), 0o600); err != nil {
		return fmt.Errorf("write cfg: %w", err)
	}

	exe, err := os.Executable()
	if err != nil {
		return err
	}
	base := filepath.Dir(exe)
	coreExe := filepath.Join(base, "core", "sing-box.exe")
	if _, err := os.Stat(coreExe); err != nil {
		return fmt.Errorf("sing-box.exe not found at %s", coreExe)
	}

	ctx, cancel := context.WithCancel(context.Background())
	r.cancel = cancel
	cmd := exec.CommandContext(ctx, coreExe, "run", "-c", cfgPath)
	cmd.Dir = filepath.Dir(coreExe)

	stdout, _ := cmd.StdoutPipe()
	stderr, _ := cmd.StderrPipe()
	go r.pipe(stdout)
	go r.pipe(stderr)

	if err := cmd.Start(); err != nil {
		cancel()
		return fmt.Errorf("core start: %w", err)
	}
	r.cmd = cmd
	go func() { _ = cmd.Wait(); r.log("[core] exited") }()
	r.log("[core] started")
	return nil
}

func (r *CoreRunner) Stop() {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.cancel != nil {
		r.cancel()
	}
	if r.cmd != nil && r.cmd.Process != nil {
		_ = r.cmd.Process.Kill()
	}
	r.cmd = nil
	r.log("[core] stopped")
}

func (r *CoreRunner) pipe(rc io.Reader) {
	buf := make([]byte, 4096)
	for {
		n, err := rc.Read(buf)
		if n > 0 {
			r.log(string(bytes.TrimSpace(buf[:n])))
		}
		if err != nil {
			return
		}
	}
}

func (r *CoreRunner) log(s string) {
	if r.onLog != nil {
		r.onLog(s)
	}
}

// ---- ring buffer ----
type RingBuffer struct {
	mu   sync.Mutex
	data []string
	cap  int
}

func NewRingBuffer(cap int) *RingBuffer { return &RingBuffer{cap: cap} }
func (rb *RingBuffer) Append(s string) {
	rb.mu.Lock()
	defer rb.mu.Unlock()
	if len(rb.data) == rb.cap {
		rb.data = rb.data[1:]
	}
	rb.data = append(rb.data, s)
}
func (rb *RingBuffer) LastN(n int) []string {
	rb.mu.Lock()
	defer rb.mu.Unlock()
	if len(rb.data) == 0 {
		return []string{}
	}
	if n >= len(rb.data) {
		out := make([]string, len(rb.data))
		copy(out, rb.data)
		return out
	}
	out := make([]string, n)
	copy(out, rb.data[len(rb.data)-n:])
	return out
}
