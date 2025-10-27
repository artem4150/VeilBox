package main

import (
	"fmt"
	"net/url"
	"strconv"
)

func ParseVLESS(uri string) (Profile, error) {
	u, err := url.Parse(uri)
	if err != nil {
		return Profile{}, err
	}
	if u.Scheme != "vless" {
		return Profile{}, fmt.Errorf("not vless scheme")
	}

	uuid := u.User.Username()
	host := u.Hostname()
	if u.Port() == "" {
		return Profile{}, fmt.Errorf("missing port")
	}
	port, err := strconv.Atoi(u.Port())
	if err != nil {
		return Profile{}, fmt.Errorf("bad port: %w", err)
	}

	q := u.Query()
	p := Profile{
		UUID:           uuid,
		Host:           host,
		Port:           port,
		SNI:            q.Get("sni"),
		PublicKey:      q.Get("pbk"),
		ShortID:        q.Get("sid"),
		Transport:      q.Get("type"),
		ServiceName:    q.Get("serviceName"),
		Flow:           q.Get("flow"),
		PacketEncoding: q.Get("packetEncoding"),
		SpiderX:        q.Get("spx"),
	}
	if p.Transport == "" {
		p.Transport = "grpc"
	}
	return p, nil
}
