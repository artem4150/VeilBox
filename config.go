package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

type Profile struct {
	UUID           string
	Host           string
	Port           int
	SNI            string
	PublicKey      string
	ShortID        string
	Transport      string // "grpc"|"h2"|"ws"
	ServiceName    string
	Flow           string
	PacketEncoding string
	SpiderX        string
}

type SplitTunnelSettings struct {
	BypassDomains   []string `json:"bypassDomains"`
	BypassIPs       []string `json:"bypassIPs"`
	BypassProcesses []string `json:"bypassProcesses"`
	ProxyDomains    []string `json:"proxyDomains"`
	ProxyIPs        []string `json:"proxyIPs"`
	ProxyProcesses  []string `json:"proxyProcesses"`
	BlockDomains    []string `json:"blockDomains"`
	BlockIPs        []string `json:"blockIPs"`
	BlockProcesses  []string `json:"blockProcesses"`
}

type DNSUpstream struct {
	Tag      string `json:"tag"`
	Type     string `json:"type"`
	Address  string `json:"address"`
	Detour   string `json:"detour,omitempty"`
	Strategy string `json:"strategy,omitempty"`
}

type DNSSettings struct {
	Strategy string        `json:"strategy"`
	Servers  []DNSUpstream `json:"servers"`
}

type RegionRoutingSettings struct {
	ProxyCountries  []string `json:"proxyCountries"`
	DirectCountries []string `json:"directCountries"`
	BlockCountries  []string `json:"blockCountries"`
}

type MetricsSettings struct {
	EnableObservatory bool   `json:"enableObservatory"`
	ObservatoryListen string `json:"observatoryListen"`
	ObservatoryToken  string `json:"observatoryToken"`
}

type ConnectionOptions struct {
	Mode          string                 `json:"mode"`
	SplitTunnel   *SplitTunnelSettings   `json:"splitTunnel"`
	DNS           *DNSSettings           `json:"dns"`
	RegionRouting *RegionRoutingSettings `json:"regionRouting"`
	Metrics       *MetricsSettings       `json:"metrics"`
}

func BuildConfigFromProfile(p Profile, opts ConnectionOptions) (string, error) {
	var tpl string
	mode := strings.ToLower(opts.Mode)
	switch mode {
	case "", "proxy":
		tpl = string(TplVlessRealityGrpcProxy)
	case "tun":
		tpl = string(TplVlessRealityGrpcTun)
	default:
		return "", fmt.Errorf("unsupported mode %q", mode)
	}

	repls := map[string]string{
		"__HOST__":         jsonEscape(p.Host),
		"__PORT__":         strconv.Itoa(p.Port),
		"__UUID__":         jsonEscape(p.UUID),
		"__SNI__":          jsonEscape(p.SNI),
		"__PUBKEY__":       jsonEscape(p.PublicKey),
		"__SHORTID__":      jsonEscape(p.ShortID),
		"__GRPC_SERVICE__": jsonEscape(p.ServiceName),
		"__FLOW__":         jsonEscape(p.Flow),
		"__PACKET_ENCODING__": func() string {
			if p.PacketEncoding == "" {
				return ""
			}
			return jsonEscape(p.PacketEncoding)
		}(),
	}

	var out bytes.Buffer
	out.WriteString(tpl)
	js := out.String()
	for k, v := range repls {
		js = strings.ReplaceAll(js, k, v)
	}
	transport := strings.ToLower(p.Transport)
	if transport == "" {
		transport = "grpc"
	}
	var transportBlock string
	var multiplexBlock string
	switch transport {
	case "grpc":
		service := p.ServiceName
		transportBlock = fmt.Sprintf(`      "transport": { "type": "grpc", "service_name": "%s", "idle_timeout": "15s", "permit_without_stream": true },`, jsonEscape(service))
		multiplexBlock = `      "multiplex": { "enabled": true, "max_connections": 8, "min_streams": 4, "max_streams": 32 }`
	case "tcp":
		multiplexBlock = `      "multiplex": { "enabled": false }`
	default:
		return "", fmt.Errorf("unsupported transport %q", transport)
	}
	js = strings.ReplaceAll(js, "__TRANSPORT_BLOCK__", transportBlock)
	js = strings.ReplaceAll(js, "__MULTIPLEX_BLOCK__", multiplexBlock)
	dnsBlock := renderDNSBlock(opts.DNS)
	js = strings.ReplaceAll(js, "__DNS_BLOCK__", dnsBlock)
	inboundTags := []string{"socks-in", "http-in"}
	if mode == "tun" {
		inboundTags = append(inboundTags, "tun-in")
	}
	rulesBlock, ruleSetsBlock := renderRouteSections(opts.SplitTunnel, opts.RegionRouting, inboundTags)
	js = strings.ReplaceAll(js, "__ROUTE_RULES__", rulesBlock)
	js = strings.ReplaceAll(js, "__ROUTE_RULE_SETS__", ruleSetsBlock)
	experimentalBlock := renderExperimentalBlock(opts.Metrics)
	js = strings.ReplaceAll(js, "__EXPERIMENTAL_BLOCK__", experimentalBlock)
	return js, nil
}

func jsonEscape(s string) string {
	repl := strings.NewReplacer(`\`, `\\`, `"`, `\"`)
	return repl.Replace(s)
}

func renderDNSBlock(settings *DNSSettings) string {
	defaultServers := []map[string]any{
		{
			"tag":             "secure",
			"type":            "https",
			"server":          "dns.google",
			"domain_resolver": "local",
		},
		{
			"tag":  "local",
			"type": "local",
		},
	}
	dnsObj := map[string]any{
		"servers":  defaultServers,
		"strategy": "prefer_ipv4",
	}
	if settings != nil {
		if len(settings.Servers) > 0 {
			var servers []map[string]any
			for _, srv := range settings.Servers {
				address := strings.TrimSpace(srv.Address)
				if address == "" {
					continue
				}
				serverType := srv.Type
				if serverType == "" {
					if strings.HasPrefix(address, "https://") || strings.Contains(address, "://") && strings.HasPrefix(address, "https") {
						serverType = "https"
					} else if strings.HasPrefix(address, "tls://") {
						serverType = "tls"
					} else if address == "local" {
						serverType = "local"
					} else {
						serverType = "udp"
					}
				}
				entry := map[string]any{
					"tag":  firstNonEmpty(srv.Tag, addressTag(address)),
					"type": serverType,
				}
				if serverType != "local" {
					entry["server"] = address
				}
				if srv.Detour != "" {
					entry["detour"] = srv.Detour
				}
				if srv.Strategy != "" {
					entry["strategy"] = srv.Strategy
				}
				servers = append(servers, entry)
			}
			if len(servers) > 0 {
				dnsObj["servers"] = servers
			}
		}
		if settings.Strategy != "" {
			dnsObj["strategy"] = settings.Strategy
		}
	}
	b, err := json.MarshalIndent(dnsObj, "", "  ")
	if err != nil {
		return "{\n    \"servers\": []\n  }"
	}
	return indentTailLines(string(b), "  ")
}

func renderRouteSections(split *SplitTunnelSettings, region *RegionRoutingSettings, inboundTags []string) (string, string) {
	var blockRules []map[string]any
	var directRules []map[string]any
	var proxyRules []map[string]any

	if split != nil {
		if domains := cleanStringList(split.BlockDomains); len(domains) > 0 {
			blockRules = append(blockRules, map[string]any{
				"domain":   domains,
				"outbound": "block",
			})
		}
		if ips := cleanStringList(split.BlockIPs); len(ips) > 0 {
			blockRules = append(blockRules, map[string]any{
				"ip_cidr":  ips,
				"outbound": "block",
			})
		}
		if processes := cleanStringList(split.BlockProcesses); len(processes) > 0 {
			blockRules = append(blockRules, map[string]any{
				"process_name": processes,
				"outbound":     "block",
			})
		}
		if domains := cleanStringList(split.BypassDomains); len(domains) > 0 {
			directRules = append(directRules, map[string]any{
				"domain":   domains,
				"outbound": "direct",
			})
		}
		if ips := cleanStringList(split.BypassIPs); len(ips) > 0 {
			directRules = append(directRules, map[string]any{
				"ip_cidr":  ips,
				"outbound": "direct",
			})
		}
		if processes := cleanStringList(split.BypassProcesses); len(processes) > 0 {
			directRules = append(directRules, map[string]any{
				"process_name": processes,
				"outbound":     "direct",
			})
		}
		if domains := cleanStringList(split.ProxyDomains); len(domains) > 0 {
			proxyRules = append(proxyRules, map[string]any{
				"domain":   domains,
				"outbound": "proxy",
			})
		}
		if ips := cleanStringList(split.ProxyIPs); len(ips) > 0 {
			proxyRules = append(proxyRules, map[string]any{
				"ip_cidr":  ips,
				"outbound": "proxy",
			})
		}
		if processes := cleanStringList(split.ProxyProcesses); len(processes) > 0 {
			proxyRules = append(proxyRules, map[string]any{
				"process_name": processes,
				"outbound":     "proxy",
			})
		}
	}

	if region != nil {
		if countries := uppercaseCodes(region.BlockCountries); len(countries) > 0 {
			blockRules = append(blockRules, map[string]any{
				"geoip":    countries,
				"outbound": "block",
			})
		}
		if countries := uppercaseCodes(region.DirectCountries); len(countries) > 0 {
			directRules = append(directRules, map[string]any{
				"geoip":    countries,
				"outbound": "direct",
			})
		}
		if countries := uppercaseCodes(region.ProxyCountries); len(countries) > 0 {
			proxyRules = append(proxyRules, map[string]any{
				"geoip":    countries,
				"outbound": "proxy",
			})
		}
	}

	// default block rule for ads
	blockRules = append(blockRules, map[string]any{
		"rule_set": "geosite-category-ads-all",
		"outbound": "block",
	})

	var rulesSequence []map[string]any
	rulesSequence = append(rulesSequence, blockRules...)
	rulesSequence = append(rulesSequence, directRules...)
	rulesSequence = append(rulesSequence, map[string]any{
		"ip_is_private": true,
		"outbound":      "direct",
	})
	rulesSequence = append(rulesSequence, proxyRules...)
	rulesSequence = append(rulesSequence, map[string]any{
		"inbound":  inboundTags,
		"outbound": "proxy",
	})

	rulesBlock := marshalListWithIndent(rulesSequence, "      ")

	defaultRuleSet := map[string]any{
		"tag":             "geosite-category-ads-all",
		"type":            "remote",
		"format":          "binary",
		"url":             "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/geosite-category-ads-all.srs",
		"download_detour": "direct",
	}
	ruleSetsBlock := marshalListWithIndent([]map[string]any{defaultRuleSet}, "      ")

	return rulesBlock, ruleSetsBlock
}

func renderExperimentalBlock(settings *MetricsSettings) string {
	experimental := map[string]any{
		"cache_file": map[string]any{"enabled": true},
	}
	if settings != nil && settings.EnableObservatory {
		listen := strings.TrimSpace(settings.ObservatoryListen)
		if listen == "" {
			listen = "127.0.0.1:9090"
		}
		token := strings.TrimSpace(settings.ObservatoryToken)
		ob := map[string]any{
			"enabled": true,
			"listen":  listen,
		}
		if token != "" {
			ob["token"] = token
		}
		experimental["observatory"] = ob

		clash := map[string]any{
			"external_controller":         listen,
			"access_control_allow_origin": []string{"*"},
		}
		if token != "" {
			clash["secret"] = token
		}
		experimental["clash_api"] = clash
	}
	b, err := json.MarshalIndent(experimental, "", "  ")
	if err != nil {
		return "{\n    \"cache_file\": { \"enabled\": true }\n  }"
	}
	return indentTailLines(string(b), "  ")
}

func marshalListWithIndent(items []map[string]any, indent string) string {
	if len(items) == 0 {
		return ""
	}
	lines := make([]string, 0, len(items))
	for i, item := range items {
		data, err := json.MarshalIndent(item, "", "  ")
		if err != nil {
			continue
		}
		block := indentAllLines(string(data), indent)
		if i < len(items)-1 {
			block += ","
		}
		lines = append(lines, block)
	}
	return strings.Join(lines, "\n")
}

func indentTailLines(block string, indent string) string {
	if !strings.Contains(block, "\n") {
		return block
	}
	lines := strings.Split(block, "\n")
	for i := 1; i < len(lines); i++ {
		lines[i] = indent + lines[i]
	}
	return strings.Join(lines, "\n")
}

func indentAllLines(block string, indent string) string {
	lines := strings.Split(block, "\n")
	for i := range lines {
		lines[i] = indent + lines[i]
	}
	return strings.Join(lines, "\n")
}

func cleanStringList(values []string) []string {
	var out []string
	for _, v := range values {
		if trimmed := strings.TrimSpace(v); trimmed != "" {
			out = append(out, trimmed)
		}
	}
	return out
}

func uppercaseCodes(values []string) []string {
	var out []string
	for _, v := range values {
		if trimmed := strings.TrimSpace(v); trimmed != "" {
			out = append(out, strings.ToUpper(trimmed))
		}
	}
	return out
}

func firstNonEmpty(values ...string) string {
	for _, v := range values {
		if strings.TrimSpace(v) != "" {
			return v
		}
	}
	return ""
}

func addressTag(addr string) string {
	addr = strings.TrimSpace(addr)
	if addr == "" {
		return "dns"
	}
	addr = strings.TrimPrefix(addr, "https://")
	addr = strings.TrimPrefix(addr, "tls://")
	addr = strings.TrimPrefix(addr, "udp://")
	addr = strings.TrimPrefix(addr, "tcp://")
	if idx := strings.Index(addr, "/"); idx > 0 {
		addr = addr[:idx]
	}
	return addr
}
