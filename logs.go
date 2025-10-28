package main

import "sync"

// RingBuffer — простой потокобезопасный кольцевой буфер строк.
// Совместим с вызовами из app.go: NewRingBuffer, Append, LastN.
type RingBuffer struct {
	mu   sync.Mutex
	cap  int
	data []string
}

func NewRingBuffer(capacity int) *RingBuffer {
	if capacity <= 0 {
		capacity = 500
	}
	return &RingBuffer{cap: capacity}
}

func (rb *RingBuffer) Append(s string) {
	rb.mu.Lock()
	defer rb.mu.Unlock()
	if rb.cap <= 0 {
		rb.cap = 500
	}
	if len(rb.data) >= rb.cap {
		copy(rb.data, rb.data[1:])
		rb.data[len(rb.data)-1] = s
	} else {
		rb.data = append(rb.data, s)
	}
}

// LastN — именно так ожидает app.go
func (rb *RingBuffer) LastN(n int) []string {
	rb.mu.Lock()
	defer rb.mu.Unlock()
	if len(rb.data) == 0 {
		return []string{}
	}
	if n <= 0 || n >= len(rb.data) {
		out := make([]string, len(rb.data))
		copy(out, rb.data)
		return out
	}
	out := make([]string, n)
	copy(out, rb.data[len(rb.data)-n:])
	return out
}
